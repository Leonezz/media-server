use std::io;

use bitstream_io::{BigEndian, BitRead, BitReader, BitWrite, BitWriter};
use utils::bits::{bool_from_bit, bool_to_bit};

use crate::errors::AacResult;

#[derive(Debug)]
pub struct VariableHeader {
    pub copyright_identification_bit: bool,     // 1 bit
    pub copyright_identification_start: bool,   // 1 bit
    pub aac_frame_length: u16,                  // 13 bits
    pub adts_buffer_fullness: u16,              // 11 bits
    pub number_of_raw_data_blocks_in_frame: u8, // 2 bits
}

impl VariableHeader {
    pub fn read_from<R: io::Read>(reader: &mut BitReader<R, BigEndian>) -> AacResult<Self> {
        let copyright_identification_bit = bool_from_bit(reader.read::<u8>(1)?);
        let copyright_identification_start = bool_from_bit(reader.read::<u8>(1)?);
        let aac_frame_length = reader.read::<u16>(13)?;
        let adts_buffer_fullness = reader.read::<u16>(11)?;
        let number_of_raw_data_blocks_in_frame = reader.read::<u8>(2)?;

        Ok(Self {
            copyright_identification_bit,
            copyright_identification_start,
            aac_frame_length,
            adts_buffer_fullness,
            number_of_raw_data_blocks_in_frame,
        })
    }

    pub fn write_to<W: io::Write>(&self, writer: &mut BitWriter<W, BigEndian>) -> AacResult<()> {
        writer.write(1, bool_to_bit(self.copyright_identification_bit))?;
        writer.write(1, bool_to_bit(self.copyright_identification_start))?;
        writer.write(13, self.aac_frame_length)?;
        writer.write(11, self.adts_buffer_fullness)?;
        writer.write(2, self.number_of_raw_data_blocks_in_frame)?;
        Ok(())
    }
}
