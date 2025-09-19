use bitstream_io::BitRead;
use utils::traits::reader::BitwiseReadFrom;

use crate::errors::AACCodecError;

use super::TTSSequence;

impl<R: BitRead> BitwiseReadFrom<R> for TTSSequence {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let tts_sequence_id = reader.read::<5, u8>()?;
        let language_code = reader.read::<18, u32>()?;
        let gender_enable = reader.read_bit()?;
        let age_enable = reader.read_bit()?;
        let speech_rate_enable = reader.read_bit()?;
        let prosody_enable = reader.read_bit()?;
        let video_enable = reader.read_bit()?;
        let lip_shape_enable = reader.read_bit()?;
        let trick_mode_enable = reader.read_bit()?;
        Ok(Self {
            tts_sequence_id,
            language_code,
            gender_enable,
            age_enable,
            speech_rate_enable,
            prosody_enable,
            video_enable,
            lip_shape_enable,
            trick_mode_enable,
        })
    }
}
