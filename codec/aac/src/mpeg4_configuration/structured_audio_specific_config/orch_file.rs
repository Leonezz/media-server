use bitstream_io::BitWrite;
use utils::traits::{dynamic_sized_packet::DynamicSizedBitsPacket, writer::BitwiseWriteTo};

use crate::errors::AACCodecError;

use super::{DataWithLength, Symbol, write_data_with_length};

#[derive(Debug, Clone)]
pub struct OrchToken {
    pub done: i32,
    pub token: u8,
    pub content: OrchTokenContent,
}

impl DynamicSizedBitsPacket for OrchToken {
    fn get_packet_bits_count(&self) -> usize {
        8 + // token
        self.content.get_packet_bits_count()
    }
}

#[derive(Debug, Clone)]
pub enum OrchTokenContent {
    Sym(Symbol),
    ValF32(f32),
    ValU32(u32),
    String(DataWithLength<u8, u8>),
    ValU8(u8),
    End,
}

impl OrchTokenContent {
    pub fn get_token(&self) -> u8 {
        match self {
            Self::Sym(_) => 0xF0,
            Self::ValF32(_) => 0xF1,
            Self::ValU32(_) => 0xF2,
            Self::String(_) => 0xF3,
            Self::ValU8(_) => 0xF4,
            Self::End => 0xFF,
        }
    }
}

impl DynamicSizedBitsPacket for OrchTokenContent {
    fn get_packet_bits_count(&self) -> usize {
        match self {
            Self::Sym(_) => 16,
            Self::ValF32(_) => 32,
            Self::ValU32(_) => 32,
            Self::String(str) => {
                8 + // length
                str.data.len() * 8
            }
            Self::ValU8(_) => 8,
            Self::End => 0,
        }
    }
}

pub type OrchFile = DataWithLength<u16, OrchToken>;

impl<W: BitWrite> BitwiseWriteTo<W> for OrchFile {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        write_data_with_length(self, 16, writer)
    }
}

impl DynamicSizedBitsPacket for OrchFile {
    fn get_packet_bits_count(&self) -> usize {
        16 + // length
        self.data.iter().fold(0, |prev, item| prev + item.get_packet_bits_count())
    }
}
