use rand::{Rng, SeedableRng, rngs::SmallRng, seq::SliceRandom};

use std::simd::i32x4;

pub trait Rand: Send + Sync + 'static {
    fn new() -> Self;
    fn int_n(&mut self, n: i32) -> i32;
    fn range(&mut self, min: i32, max: i32) -> i32;
    fn range_simd(&mut self, min: i32, max: i32) -> i32x4;
    fn shuffle<A>(&mut self, arr: &mut [A], n: usize);
    fn sample_n(&mut self, arr: &mut [u32], length: u32, amount: u32);
}

pub struct FastRand {
    rng: SmallRng,
}

impl Rand for FastRand {
    fn new() -> Self {
        let rng = SeedableRng::seed_from_u64(0);
        FastRand { rng }
    }

    fn int_n(&mut self, n: i32) -> i32 {
        self.range(0, n - 1)
    }

    fn range(&mut self, min: i32, max: i32) -> i32 {
        self.rng.random_range(min..(max + 1))
    }

    fn range_simd(&mut self, min: i32, max: i32) -> i32x4 {
        self.rng.random_range(i32x4::splat(min)..(i32x4::splat(max + 1)))
    }

    fn shuffle<T>(&mut self, arr: &mut [T], n: usize) {
        arr.partial_shuffle(&mut self.rng, n);
    }

    fn sample_n(&mut self, arr: &mut [u32], length: u32, amount: u32) {
        debug_assert!(amount <= length);
        debug_assert!(arr.len() >= amount as usize);
        for (index, j) in (length - amount..length).enumerate() {
            let t = self.rng.random_range(..=j);
            if let Some(pos) = arr.iter().position(|&x| x == t) {
                arr[pos] = j;
            }
            arr[index] = t;
        }
    }
}

pub struct MaxRand;

impl Rand for MaxRand {
    fn new() -> Self {
        MaxRand {}
    }

    fn int_n(&mut self, n: i32) -> i32 {
        n - 1
    }

    // Closed interval
    fn range(&mut self, _min: i32, max: i32) -> i32 {
        max
    }

    fn range_simd(&mut self, _min: i32, max: i32) -> i32x4 {
        i32x4::splat(max)
    }

    fn shuffle<T>(&mut self, arr: &mut [T], _n: usize) {
        arr.rotate_left(1);
    }

    fn sample_n(&mut self, arr: &mut [u32], length: u32, amount: u32) {
        for i in 0..amount {
            arr[i as usize] = length - i - 1;
        }
    }
}
