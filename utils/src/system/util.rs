pub fn random_fill(buffer: &mut [u8]) {
    for i in buffer {
        *i = rand::random();
    }
}
