pub trait DynamicSizedPacket {
    fn get_packet_bytes_count(&self) -> usize;
}
