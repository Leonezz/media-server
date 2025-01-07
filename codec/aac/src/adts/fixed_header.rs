use crate::errors::{AacError, AacResult};
use bitstream_io::{BigEndian, BitRead, BitReader, BitWrite, BitWriter};
use std::io;
use utils::bits::{bool_from_bit, bool_to_bit};

pub const SYNC_WORD: u16 = 0b1111_1111_1111;

#[derive(Debug)]
pub struct FixedHeader {
    pub sync_word: u16,               // 12 bits
    pub id: bool,                     // 1 bit
    pub layer: u8,                    // 2 bits
    pub protection_absent: bool,      // 1 bit, byte boundary
    pub profile: u8,                  // 2 bits
    pub sampling_frequency_index: u8, // 4 bits
    pub private_bit: bool,            // 1 bit
    pub channel_configuration: u8,    // 3 bits
    pub original_copy: bool,          // 1 bit
    pub home: bool,                   // 1 bit
}

impl FixedHeader {
    pub fn read_from<R: io::Read>(reader: &mut BitReader<R, BigEndian>) -> AacResult<Self> {
        let sync_word: u16 = reader.read(12)?;
        if sync_word != SYNC_WORD {
            return Err(AacError::WrongSyncWord(sync_word));
        }

        let id = bool_from_bit(reader.read::<u8>(1)?);
        let layer = reader.read::<u8>(2)?;
        let protection_absent = bool_from_bit(reader.read::<u8>(1)?);
        let profile = reader.read::<u8>(2)?;
        let sampling_frequency_index = reader.read::<u8>(4)?;
        let private_bit = bool_from_bit(reader.read::<u8>(1)?);
        let channel_configuration = reader.read::<u8>(3)?;
        let original_copy = bool_from_bit(reader.read::<u8>(1)?);
        let home = bool_from_bit(reader.read::<u8>(1)?);

        Ok(Self {
            sync_word,
            id,
            layer,
            protection_absent,
            profile,
            sampling_frequency_index,
            private_bit,
            channel_configuration,
            original_copy,
            home,
        })
    }

    pub fn write_to<W: io::Write>(&self, writer: &mut BitWriter<W, BigEndian>) -> AacResult<()> {
        writer.write(12, SYNC_WORD)?;
        writer.write(1, bool_to_bit(self.id))?;
        writer.write(2, self.layer)?;
        writer.write(1, bool_to_bit(self.protection_absent))?;
        writer.write(2, self.profile)?;
        writer.write(4, self.sampling_frequency_index)?;
        writer.write(1, bool_to_bit(self.private_bit))?;
        writer.write(3, self.channel_configuration)?;
        writer.write(1, bool_to_bit(self.original_copy))?;
        writer.write(1, bool_to_bit(self.home))?;

        Ok(())
    }
}
