use std::io;

use crate::errors::CodecCommonError;
use bitstream_io::BigEndian;
use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;
use utils::traits::writer::WriteTo;

use super::AudioConfig;

impl<W: io::Write> WriteTo<W> for AudioConfig {
    type Error = CodecCommonError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::AAC(config) => {
                let mut writer = bitstream_io::BitWriter::endian(writer, BigEndian);
                config.write_to(writer.by_ref()).map_err(|err| {
                    CodecCommonError::WriteAudioConfigFailed(
                        Box::new(self.clone()),
                        format!("{}", err),
                    )
                })?;
                writer.byte_align()?;
                Ok(())
            }
        }
    }
}
