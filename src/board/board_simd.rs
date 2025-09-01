use super::*;

use core::arch::x86_64::{_mm256_castps_si256, _mm256_castsi256_ps, _mm256_permutevar_ps};
use std::simd::{
    cmp::SimdPartialEq, cmp::SimdPartialOrd, i16x8, mask8x8, mask16x8, mask32x4, mask32x8, num::SimdInt, num::SimdUint,
    simd_swizzle, u8x8, u16x8, u32x8,
};

pub type CoordVec = u16x8;
pub type IndexVec = u32x8;
pub type IndexMask = mask32x8;
pub type BoardVec = u8x8;
pub type BoardMask = mask8x8;

type SnakeMask = mask32x4;

pub const INDEXES: IndexVec = IndexVec::from_array([0, 1, 2, 3, 4, 5, 6, 7]);

const X_MASK: [bool; 8] = [true, false, true, false, true, false, true, false];
const Y_MASK: [bool; 8] = [false, true, false, true, false, true, false, true];

const X_INDEX: [usize; 4] = [0, 2, 4, 6];
const Y_INDEX: [usize; 4] = [1, 3, 5, 7];

pub fn any_bits_set_simd(sqrs: BoardVec, bits: u8) -> BoardMask {
    (sqrs & u8x8::splat(bits)).simd_ne(u8x8::splat(0))
}

impl Board {
    fn shuffle_vec(&self, vec: u32x8, idx: u32x8) -> u32x8 {
        // Portable simd doesn't have dynamic shuffle yet
        // Use intrinsics as suggested by https://stackoverflow.com/a/56039255
        unsafe { _mm256_castps_si256(_mm256_permutevar_ps(_mm256_castsi256_ps(vec.into()), idx.into())) }.into()
    }

    pub fn at_idx_simd(&self, idx: usize, len: usize) -> BoardVec {
        let end = std::cmp::min(len, idx + BoardVec::LEN);
        BoardVec::load_or_default(&self.board_arr[idx..end])
    }

    pub fn snake_head_simd(&self) -> CoordVec {
        CoordVec::from_array([
            self.snakes[0].body[self.snakes[0].head_ptr as usize].x() as u16,
            self.snakes[0].body[self.snakes[0].head_ptr as usize].y() as u16,
            self.snakes[1].body[self.snakes[1].head_ptr as usize].x() as u16,
            self.snakes[1].body[self.snakes[1].head_ptr as usize].y() as u16,
            self.snakes[2].body[self.snakes[2].head_ptr as usize].x() as u16,
            self.snakes[2].body[self.snakes[2].head_ptr as usize].y() as u16,
            self.snakes[3].body[self.snakes[3].head_ptr as usize].x() as u16,
            self.snakes[3].body[self.snakes[3].head_ptr as usize].y() as u16,
        ])
    }

    pub fn on_board_simd(&self, squares: CoordVec) -> SnakeMask {
        let x_mask = mask16x8::from_array(X_MASK);
        let y_mask = mask16x8::from_array(Y_MASK);

        let mut on_board_mask = x_mask.select_mask(squares.simd_ge(CoordVec::splat(0)), mask16x8::splat(false));
        on_board_mask &= x_mask.select_mask(
            squares.simd_lt(CoordVec::splat(self.width as u16)),
            mask16x8::splat(false),
        );

        on_board_mask &= y_mask.select_mask(squares.simd_ge(CoordVec::splat(0)), mask16x8::splat(false));
        on_board_mask &= y_mask.select_mask(
            squares.simd_lt(CoordVec::splat(self.height as u16)),
            mask16x8::splat(false),
        );

        let x_on_board = SnakeMask::from_int(simd_swizzle!(on_board_mask.to_int(), X_INDEX).cast::<i32>());
        let y_on_board = SnakeMask::from_int(simd_swizzle!(on_board_mask.to_int(), Y_INDEX).cast::<i32>());

        x_on_board & y_on_board
    }

    pub fn move_to_coord_simd(&self, heads: CoordVec, mv: Move, rules: Rules) -> CoordVec {
        let mv_incr = i16x8::from_array([-1, 0, 1, 0, 0, 1, 0, -1]);
        let mvs = CoordVec::splat(mv.idx() as u16);

        let mvs_incr: CoordVec = self.shuffle_vec(mv_incr.cast(), mvs.cast()).cast();

        let mut squares = heads + mvs_incr;

        if let Rules::Wrapped = rules {
            let x_mask = mask16x8::from_array(X_MASK);
            let y_mask = mask16x8::from_array(Y_MASK);

            squares = x_mask.select(squares % CoordVec::splat(self.width as u16), squares);
            squares = y_mask.select(squares % CoordVec::splat(self.height as u16), squares);

            let wrap_mask_x = squares.simd_ge(CoordVec::splat(0));
            let wrap_mask_y = squares.simd_ge(CoordVec::splat(0));

            squares = x_mask.select(
                wrap_mask_x.select(squares + CoordVec::splat(self.width as u16), squares),
                squares,
            );

            squares = y_mask.select(
                wrap_mask_y.select(squares + CoordVec::splat(self.width as u16), squares),
                squares,
            );
        }

        squares
    }
}
