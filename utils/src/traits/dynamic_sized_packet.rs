pub trait DynamicSizedPacket {
    fn get_packet_bytes_count(&self) -> usize;
}

pub trait DynamicSizedBitsPacket {
    fn get_packet_bits_count(&self) -> usize;
}
