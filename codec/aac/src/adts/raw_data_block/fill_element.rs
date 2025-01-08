use std::io;

use bitstream_io::{BigEndian, BitRead, BitReader};
use tokio_util::bytes::Bytes;

use crate::errors::AacResult;

pub mod extension_type {
    pub const EXT_FILL: u8 = 0b0000;
    pub const EXT_FILL_DATA: u8 = 0b0001;
    pub const EXT_DYNAMIC_RANGE: u8 = 0b1011;
    pub const EXT_SBR_DATA: u8 = 0b1101;
    pub const EXT_SBR_DATA_CRC: u8 = 0b1110;
}

#[repr(u8)]
#[derive(Debug)]
pub enum ExtensionType {
    Fill = extension_type::EXT_FILL,
    FillData = extension_type::EXT_FILL_DATA,
    DynamicRange = extension_type::EXT_DYNAMIC_RANGE,
    SBREnhancement = extension_type::EXT_SBR_DATA,
    SBREnhancementWithCRC = extension_type::EXT_SBR_DATA_CRC,
    Reserved,
}

impl Into<u8> for ExtensionType {
    fn into(self) -> u8 {
        self as u8
    }
}

impl From<u8> for ExtensionType {
    fn from(value: u8) -> Self {
        match value {
            extension_type::EXT_FILL => Self::Fill,
            extension_type::EXT_FILL_DATA => Self::FillData,
            extension_type::EXT_DYNAMIC_RANGE => Self::DynamicRange,
            extension_type::EXT_SBR_DATA => Self::SBREnhancement,
            extension_type::EXT_SBR_DATA_CRC => Self::SBREnhancementWithCRC,
            _ => Self::Reserved,
        }
    }
}

// skip all but reserved data
#[derive(Debug)]
pub enum ExtensionData {
    ExtFill,
    ExtFillData,
    ExtDynamicRange,
    ExtSBREnhancement,
    ExtSBREnhancementWithCRC,
    Reserved(Bytes),
}

impl ExtensionData {
    pub fn read_from<R: io::Read>(
        count: u16,
        reader: &mut BitReader<R, BigEndian>,
    ) -> AacResult<(Self, usize)> {
        let extension_type: ExtensionType = reader.read::<u8>(4)?.into();
        match extension_type {
            ExtensionType::Fill => {
                reader.skip(count as u32 * 8 - 4)?;
                Ok((Self::ExtFill, count as usize))
            }
            ExtensionType::FillData => {
                reader.skip(count as u32 * 8 - 4)?;
                Ok((Self::ExtFillData, count as usize))
            }
            ExtensionType::DynamicRange => {
                reader.skip(count as u32 * 8 - 4)?;
                Ok((Self::ExtDynamicRange, count as usize))
            }
            ExtensionType::SBREnhancement => {
                reader.skip(count as u32 * 8 - 4)?;
                Ok((Self::ExtSBREnhancement, count as usize))
            }
            ExtensionType::SBREnhancementWithCRC => {
                reader.skip(count as u32 * 8 - 4)?;
                Ok((Self::ExtSBREnhancementWithCRC, count as usize))
            }
            ExtensionType::Reserved => {
                let (bytes, cnt) = Self::read_bytes_from(count, reader)?;
                Ok((Self::Reserved(bytes), cnt))
            }
        }
    }

    fn read_bytes_from<R: io::Read>(
        count: u16,
        reader: &mut BitReader<R, BigEndian>,
    ) -> AacResult<(Bytes, usize)> {
        let mut bytes = Vec::with_capacity(count as usize);
        bytes.resize((count - 1) as usize, 0);
        reader.read_bytes(&mut bytes)?;
        // the last byte has only 4 bits because the extension_type takes 4 at the beginning
        let last_byte = reader.read::<u8>(4)?;
        bytes.push(last_byte);
        Ok((Bytes::from(bytes), count as usize))
    }
}

#[derive(Debug)]
pub struct AacFillElement {
    pub count: u16,
    pub payload: Vec<ExtensionData>,
}

impl AacFillElement {
    pub fn read_from<R: io::Read>(reader: &mut BitReader<R, BigEndian>) -> AacResult<Self> {
        let mut count = {
            let mut count = reader.read::<u16>(4)?;
            if count == 15 {
                let esc_count = reader.read::<u16>(8)?;
                count = count + esc_count - 1;
            }
            count
        };

        let mut payload = Vec::with_capacity(count as usize);

        while count > 0 {
            let (extension, bytes_cnt) = ExtensionData::read_from(count, reader)?;
            payload.push(extension);
            count -= bytes_cnt as u16;
        }

        Ok(Self { count, payload })
    }
}
