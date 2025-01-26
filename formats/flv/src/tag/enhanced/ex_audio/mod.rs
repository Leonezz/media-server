use ex_audio_header::{AudioPacketType, ExAudioTagHeader};

pub mod ex_audio_body;
pub mod ex_audio_header;
pub mod reader;
pub mod writer;

impl ExAudioTagHeader {
    #[inline]
    pub fn is_sequence_header(&self) -> bool {
        matches!(self.packet_type, AudioPacketType::SequenceStart)
    }
}
