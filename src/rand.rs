use rand::{rngs::SmallRng, seq::SliceRandom, Rng, SeedableRng};

pub trait Rand: Send + Sync + 'static {
    fn new() -> Self;
    fn int_n(&mut self, n: i32) -> i32;
    fn range(&mut self, min: i32, max: i32) -> i32;
    fn shuffle<A>(&mut self, arr: &mut [A], n: usize);
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
        self.rng.gen_range(min..(max + 1))
    }

    fn shuffle<A>(&mut self, arr: &mut [A], n: usize) {
        arr.partial_shuffle(&mut self.rng, n);
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

    fn shuffle<A>(&mut self, arr: &mut [A], _n: usize) {
        arr.rotate_left(1);
    }
}
