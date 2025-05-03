use byteorder::WriteBytesExt;
use std::io;
use utils::traits::writer::WriteTo;

use crate::errors::FLVError;

use super::{LegacyAudioTagHeader, SoundFormat, SoundRate, SoundSize, SoundType};

impl<W: io::Write> WriteTo<W> for LegacyAudioTagHeader {
    type Error = FLVError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let mut byte: u8 = 0;
        byte |= <SoundFormat as Into<u8>>::into(self.sound_format);
        byte <<= 2;
        byte |= <SoundRate as Into<u8>>::into(self.sound_rate);
        byte <<= 1;
        byte |= <SoundSize as Into<u8>>::into(self.sound_size);
        byte <<= 1;
        byte |= <SoundType as Into<u8>>::into(self.sound_type);
        if self.sound_format == SoundFormat::AAC && self.aac_packet_type.is_none() {
            return Err(FLVError::InconsistentHeader(
                "audio format header with sound_type 10 should have aac packet type, but got none"
                    .to_owned(),
            ));
        }

        writer.write_u8(byte)?;
        if let Some(packet_type) = self.aac_packet_type {
            writer.write_u8(packet_type.into())?;
        }
        Ok(())
    }
}
