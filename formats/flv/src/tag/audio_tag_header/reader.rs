use byteorder::ReadBytesExt;
use std::io;

use crate::errors::FLVResult;

use super::{AACPacketType, AudioTagHeader, SoundFormat, SoundRate, SoundSize, SoundType};

#[derive(Debug)]
pub struct Reader<R> {
    inner: R,
}

impl<R> Reader<R>
where
    R: io::Read,
{
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn read(&mut self) -> FLVResult<AudioTagHeader> {
        let first_byte = self.inner.read_u8()?;
        let sound_format: SoundFormat = ((first_byte >> 4) & 0b1111).try_into()?;
        let sound_rate: SoundRate = ((first_byte >> 2) & 0b11).try_into()?;
        let sound_size: SoundSize = ((first_byte >> 1) & 0b1).into();
        let sound_type: SoundType = ((first_byte >> 0) & 0b1).into();
        let mut aac_packet_type: Option<AACPacketType> = None;
        if sound_format == SoundFormat::AAC {
            let aac_type_byte = self.inner.read_u8()?;
            aac_packet_type = Some(aac_type_byte.into());
        }
        Ok(AudioTagHeader {
            sound_format,
            sound_rate,
            sound_size,
            sound_type,
            aac_packet_type,
        })
    }
}
