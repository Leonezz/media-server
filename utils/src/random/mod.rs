use rand::prelude::Distribution;

pub fn random_fill(buffer: &mut [u8]) {
    for i in buffer {
        *i = rand::random();
    }
}

pub fn random_u64() -> u64 {
    rand::random::<u64>()
}

pub fn random_u32() -> u32 {
    rand::random::<u32>()
}

pub fn random_u8() -> u8 {
    rand::random::<u8>()
}

pub fn random_f64() -> f64 {
    rand::random::<f64>()
}

pub fn uniform_random_f64(min: f64, max: f64) -> f64 {
    let mut rng = rand::rng();
    let uniform = rand::distr::Uniform::new(min, max).unwrap();
    uniform.sample(&mut rng)
}
