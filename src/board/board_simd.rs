use super::*;

use crate::rand::Rand;

use core::arch::x86_64::{_mm256_castps_si256, _mm256_castsi256_ps, _mm256_permutevar_ps};
use std::simd::{
    cmp::SimdPartialOrd, i16x8, mask16x8, mask32x4, num::SimdInt, num::SimdUint, simd_swizzle, u16x4, u16x8, u32x8,
    u8x16, usizex4,
};

type CoordVec = u16x8;
type IndexVec = usizex4;
type BoardSquareVec = u16x4;
type MoveVec = u16x4;

type SnakeMask = mask32x4;

const X_MASK: [bool; 8] = [true, false, true, false, true, false, true, false];
const Y_MASK: [bool; 8] = [false, true, false, true, false, true, false, true];

const X_INDEX: [usize; 4] = [0, 2, 4, 6];
const Y_INDEX: [usize; 4] = [1, 3, 5, 7];

impl Board {
    fn shuffle_vec(&self, vec: u32x8, idx: u32x8) -> u32x8 {
        // Portable simd doesn't have dynamic shuffle yet
        // Use intrinsics as suggested by https://stackoverflow.com/a/56039255
        unsafe { _mm256_castps_si256(_mm256_permutevar_ps(_mm256_castsi256_ps(vec.into()), idx.into())) }.into()
    }

    // pub fn gen_move_simd(&self, game: &Game, rng: &mut impl Rand) -> MoveVec {
    //     let valid_moves = u8x16::splat(0);

    //     let mut mv_mask = u8x16::from_array([])

    //     for mv_idx in 0..4 {
    //         let mv = Move::from_idx(mv_idx);
    //         let valid_mvs = self.valid_move_simd(game, mv);
    //     }

    //     if num_valid > 0 {
    //         let mv_idx = rng.range(0, num_valid - 1);
    //         valid_moves[mv_idx as usize]
    //     } else {
    //         Move::Left
    //     }
    // }

    pub fn valid_move_simd(&self, game: &Game, mv: Move) -> SnakeMask {
        let heads = self.snake_head_simd();
        let squares = self.move_to_coord_simd(heads, mv, game.ruleset);
        let on_board_mask = self.on_board_simd(squares);

        let sqrs = self.at_simd(squares);

        let mut mask = SnakeMask::splat(false);

        for snake_idx in 0..4 {
            let valid = match BoardSquare::from_raw(sqrs[snake_idx]) {
                BoardSquare::Empty | BoardSquare::Food | BoardSquare::FoodHazard => true,
                BoardSquare::Hazard => {
                    (self.snakes[snake_idx].health - game.api.ruleset.settings.hazard_damage_per_turn) > 0
                }
                BoardSquare::SnakeHead(_)
                | BoardSquare::SnakeHeadHazard(_)
                | BoardSquare::SnakeBody(_)
                | BoardSquare::SnakeBodyHazard(_) => false,
                BoardSquare::SnakeTail(i) => {
                    let idx = i as usize;
                    self.snake_tail(idx) != self.snakes[idx].at_tail_offset(-1)
                }
                BoardSquare::SnakeTailHazard(i) => {
                    let idx = i as usize;

                    (self.snakes[snake_idx].health - game.api.ruleset.settings.hazard_damage_per_turn) > 0
                        && self.snake_tail(idx) != self.snakes[idx].at_tail_offset(-1)
                }
            };

            mask.set(snake_idx, valid);
        }

        mask & on_board_mask
    }

    pub fn snake_head_idx_simd(&self) -> IndexVec {
        IndexVec::from_array([
            self.idx_from_coord(self.snakes[0].body[self.snakes[0].head_ptr as usize]),
            self.idx_from_coord(self.snakes[1].body[self.snakes[1].head_ptr as usize]),
            self.idx_from_coord(self.snakes[2].body[self.snakes[2].head_ptr as usize]),
            self.idx_from_coord(self.snakes[3].body[self.snakes[3].head_ptr as usize]),
        ])
    }

    pub fn snake_head_simd(&self) -> CoordVec {
        CoordVec::from_array([
            self.snakes[0].body[self.snakes[0].head_ptr as usize].x as u16,
            self.snakes[0].body[self.snakes[0].head_ptr as usize].y as u16,
            self.snakes[1].body[self.snakes[1].head_ptr as usize].x as u16,
            self.snakes[1].body[self.snakes[1].head_ptr as usize].y as u16,
            self.snakes[2].body[self.snakes[2].head_ptr as usize].x as u16,
            self.snakes[2].body[self.snakes[2].head_ptr as usize].y as u16,
            self.snakes[3].body[self.snakes[3].head_ptr as usize].x as u16,
            self.snakes[3].body[self.snakes[3].head_ptr as usize].y as u16,
        ])
    }

    pub fn at_simd(&self, coords: CoordVec) -> BoardSquareVec {
        let idxs_simd = self.idx_from_coord_simd(coords);
        BoardSquareVec::gather_or_default(&self.board_mat, idxs_simd)
    }

    pub fn idx_from_coord_simd(&self, coords: CoordVec) -> IndexVec {
        let locs_x = simd_swizzle!(coords, X_INDEX).cast::<usize>();
        let locs_y = simd_swizzle!(coords, Y_INDEX).cast::<usize>();

        (locs_x + (locs_y * IndexVec::splat(self.width as usize))).cast()
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
