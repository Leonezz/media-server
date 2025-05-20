use utils::traits::dynamic_sized_packet::DynamicSizedBitsPacket;

use super::DataWithLength;

// u24 length and u8 data
pub type MidiEvent = DataWithLength<u32, u8>;
impl DynamicSizedBitsPacket for MidiEvent {
    fn get_packet_bits_count(&self) -> usize {
        24 + // length
        self.data.len() * 8
    }
}
pub type MidiFile = DataWithLength<u32, u8>;
