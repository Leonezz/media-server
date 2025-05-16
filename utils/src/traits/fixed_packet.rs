pub trait FixedPacket {
    fn bytes_count() -> usize;
}

pub trait FixedBitwisePacket {
    fn bits_count() -> usize;
}