use utils::traits::fixed_packet::FixedBitwisePacket;

pub mod reader;
pub mod writer;
/// @see: 6.4.1 MPEG-4 audio TTSSpecificConfig
pub type TTSSpecificConfig = TTSSequence;

/// Table 6.1 — Syntax of TTS_Sequence()
#[derive(Debug, Clone)]
pub struct TTSSequence {
    pub tts_sequence_id: u8,      // 5 bits
    pub language_code: u32,       // 18 bits
    pub gender_enable: bool,      // 1 bit
    pub age_enable: bool,         // 1 bit
    pub speech_rate_enable: bool, // 1 bit
    pub prosody_enable: bool,     // 1 bit
    pub video_enable: bool,       // 1 bit
    pub lip_shape_enable: bool,   // 1 bit
    pub trick_mode_enable: bool,  // 1 bit
}

impl FixedBitwisePacket for TTSSequence {
    fn bits_count() -> usize {
        5 + // TTS_Sequence_ID
        18 + // Language_Code
        1 + // Gender_Enable
        1 + // Age_Enable
        1 + // Speech_Rate_Enable
        1 + // Prosody_Enable
        1 + // Video_Enable
        1 + // Lip_Shape_Enable
        1 // Trick_Mode_Enable
    }
}
