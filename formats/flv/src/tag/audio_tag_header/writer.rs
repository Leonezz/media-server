use byteorder::WriteBytesExt;
use std::io;

use crate::errors::{FLVError, FLVResult};

use super::{AudioTagHeader, SoundFormat, SoundRate, SoundSize, SoundType};

#[derive(Debug)]
pub struct Writer<W> {
    inner: W,
}

impl<W> Writer<W>
where
    W: io::Write,
{
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    pub fn write(&mut self, header: &AudioTagHeader) -> FLVResult<()> {
        let mut byte: u8 = 0;
        byte |= <SoundFormat as Into<u8>>::into(header.sound_format);
        byte <<= 2;
        byte |= <SoundRate as Into<u8>>::into(header.sound_rate);
        byte <<= 1;
        byte |= <SoundSize as Into<u8>>::into(header.sound_size);
        byte <<= 1;
        byte |= <SoundType as Into<u8>>::into(header.sound_type);
        if header.sound_format == SoundFormat::AAC && header.aac_packet_type.is_none() {
            return Err(FLVError::InconsistentHeader(
                "audio format header with sound_type 10 should have aac packet type, but got none"
                    .to_owned(),
            ));
        }

        self.inner.write_u8(byte)?;
        if let Some(packet_type) = header.aac_packet_type {
            self.inner.write_u8(packet_type.into())?;
        }
        Ok(())
    }
}
