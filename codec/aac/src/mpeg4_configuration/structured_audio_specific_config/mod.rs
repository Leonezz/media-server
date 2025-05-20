use bitstream_io::{BitRead, BitWrite};
use midi_file::MidiFile;
use num::{ToPrimitive, Unsigned};
use orch_file::OrchFile;
use sample::Sample;
use sbf::Sbf;
use score_file::ScoreFile;
use utils::traits::{dynamic_sized_packet::DynamicSizedBitsPacket, reader::BitwiseReadFrom, writer::BitwiseWriteTo};

use crate::errors::AACCodecError;

pub mod midi_file;
pub mod orch_file;
pub mod sample;
pub mod sbf;
pub mod score_file;
pub mod reader;
pub mod writer;
/// @see: 5.5.2 Bitstream syntax
#[derive(Debug, Clone)]
pub struct StructuredAudioSpecificConfig {
    pub chunks: Vec<StructuredAudioSpecificConfigChunk>,
}

#[derive(Debug, Clone)]
pub enum StructuredAudioSpecificConfigChunk {
    Orc(OrchFile),
    Score(ScoreFile),
    SMF(MidiFile),
    Samp(Sample),
    SampleBank(Sbf),
    Sym(SymbolTable),
}

impl StructuredAudioSpecificConfigChunk {
    pub fn chunk_type(&self) -> u8 {
        match self {
            Self::Orc(_) => 0b000,
            Self::Score(_) => 0b001,
            Self::SMF(_) => 0b010,
            Self::Samp(_) => 0b011,
            Self::SampleBank(_) => 0b100,
            Self::Sym(_) => 0b101,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataWithLength<L: Unsigned, T> {
    pub length: L,
    pub data: Vec<T>,
}

pub fn read_data_with_length<L: Unsigned + ToPrimitive + Copy, R: BitRead, T: BitwiseReadFrom<R, Error = AACCodecError>>(length: L, reader: &mut R) -> Result<DataWithLength<L, T>, AACCodecError> {
    let mut data = vec![];
    for _ in 0..length.to_usize().unwrap() {
        data.push(T::read_from(reader)?);
    }
    Ok(DataWithLength { length, data })
}

pub fn write_data_with_length<L: Unsigned + ToPrimitive + Copy, W: BitWrite, T: BitwiseWriteTo<W, Error = AACCodecError>>(data: &DataWithLength<L, T>, length: L, writer: &mut W) -> Result<(), AACCodecError> {
    writer.write_var(length.to_u32().unwrap(), data.length.to_u64().unwrap())?;
    data.data.iter().try_for_each(|item| item.write_to(writer))?;
    Ok(())
}

pub type Symbol = u16;

#[derive(Debug, Clone)]
pub struct SymbolName {
  pub length: u8, // 4 bits
  pub name: Vec<u8>, // u8 name[length]
}

impl DynamicSizedBitsPacket for SymbolName {
    fn get_packet_bits_count(&self) -> usize {
        4 + // length
        self.name.len() * 8
    }
}

pub type SymbolTable = DataWithLength<u16, SymbolName>;

impl DynamicSizedBitsPacket for StructuredAudioSpecificConfigChunk {
    fn get_packet_bits_count(&self) -> usize {
        match self {
            Self::Orc(orch_file) => orch_file.get_packet_bits_count(),
            Self::Score(score_file) => {
                20 + // num_lines
                score_file.data.iter().fold(0, |prev, item| prev + item.get_packet_bits_count())
            },
            Self::SMF(midi_file) => {
                32 + // length
                midi_file.data.len() * 8
            }
            Self::Samp(sample) => sample.get_packet_bits_count(),
            Self::SampleBank(sbf) => {
                32 + // length
                sbf.data.len() * 8
            }
            Self::Sym(symbol_table) => {
                16 + // length
                symbol_table.data.iter().fold(0, |prev, item| prev + item.get_packet_bits_count())
            }
        }
    }
}

impl DynamicSizedBitsPacket for StructuredAudioSpecificConfig {
    fn get_packet_bits_count(&self) -> usize {
        self.chunks.iter().fold(
            0,
            |prev, item| {
                prev +
                item.get_packet_bits_count() + 
                3 +// chunk_type
                1
            }, // more_data
        )
    }
}

