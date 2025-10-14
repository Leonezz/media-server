use crate::errors::TurnMessageError;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use num::ToPrimitive;
use utils::traits::{reader::ReadRemainingFrom, writer::WriteTo};

//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |          Channel Number       |             Length            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                                                               |
// /                        Application Data                       /
// /                                                               /
// |                                                               |
// |                               +-------------------------------+
// |                               |
// +-------------------------------+
#[derive(Debug, Clone)]
pub struct ChannelData {
    channel_number: u16,
    length: u16,
    application_data: Vec<u8>,
}

impl ChannelData {
    pub fn new(channel_number: u16, data: Vec<u8>) -> Self {
        Self {
            channel_number,
            length: data.len().to_u16().unwrap(),
            application_data: data,
        }
    }
}

pub const TURN_CHANNEL_DATA_FIRST_BYTE_MIN: u8 = 64;
pub const TURN_CHANNEL_DATA_FIRST_BYTE_MAX: u8 = 79;
impl<R: std::io::Read> ReadRemainingFrom<u8, R> for ChannelData {
    type Error = TurnMessageError;
    fn read_remaining_from(header: u8, reader: &mut R) -> Result<Self, Self::Error> {
        if header < TURN_CHANNEL_DATA_FIRST_BYTE_MIN || header > TURN_CHANNEL_DATA_FIRST_BYTE_MAX {
            return Err(TurnMessageError::NotChannelData(header));
        }
        let second_byte = reader.read_u8()?;
        let channel_number = u16::from_be_bytes([header, second_byte]);
        let length = reader.read_u16::<BigEndian>()?;
        let mut application_data = Vec::with_capacity(length as usize);
        reader.read_exact(&mut application_data)?;
        Ok(Self {
            channel_number,
            length,
            application_data,
        })
    }
}

impl<W: std::io::Write> WriteTo<W> for ChannelData {
    type Error = TurnMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u16::<BigEndian>(self.channel_number)?;
        writer.write_u16::<BigEndian>(self.length)?;
        writer.write_all(&self.application_data)?;
        Ok(())
    }
}
