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
