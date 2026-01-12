use rand::prelude::*;
use rand_xoshiro::Xoshiro256PlusPlus;
use std::cell::RefCell;

thread_local! {
    static GENERATOR: RefCell<RandomNumber> = RefCell::new(RandomNumber::create())
}

#[inline]
pub fn get_random_number() -> f64 {
    GENERATOR.with(|g| g.borrow_mut().generate())
}

pub struct RandomNumber {
    rng: Xoshiro256PlusPlus,
}

impl RandomNumber {
    pub fn create() -> Self {
        RandomNumber {
            rng: Xoshiro256PlusPlus::from_rng(thread_rng()).unwrap(),
        }
    }

    pub fn generate(&mut self) -> f64 {
        self.rng.gen()
    }
}
