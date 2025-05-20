use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::TTSSequence;

impl<W: BitWrite> BitwiseWriteTo<W> for TTSSequence {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<5, u8>(self.tts_sequence_id)?;
        writer.write::<18, u32>(self.language_code)?;
        writer.write_bit(self.gender_enable)?;
        writer.write_bit(self.age_enable)?;
        writer.write_bit(self.speech_rate_enable)?;
        writer.write_bit(self.prosody_enable)?;
        writer.write_bit(self.video_enable)?;
        writer.write_bit(self.lip_shape_enable)?;
        writer.write_bit(self.trick_mode_enable)?;
        Ok(())
    }
}
