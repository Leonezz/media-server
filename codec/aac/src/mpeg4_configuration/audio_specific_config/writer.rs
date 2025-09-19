use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::{
    AudioSpecificConfig, EpConfig, SpecificConfig, SyncExtensionAudioObjectType5,
    SyncExtensionAudioObjectType22, audio_object_type::AudioObjectType,
};

impl<W: BitWrite> BitwiseWriteTo<W> for AudioObjectType {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let value: u8 = (*self).into();
        if value < 31 {
            writer.write::<5, u8>(value)?;
            return Ok(());
        }
        writer.write::<5, u8>(31)?;
        writer.write::<6, u8>(value - 32)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for AudioSpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        self.audio_object_type.write_to(writer)?;
        writer.write::<4, u8>(self.sampling_frequency_index.into())?;
        if let Some(frequency) = self.sampling_frequency {
            writer.write::<24, u32>(frequency)?;
        }
        writer.write::<4, u8>(self.channel_configuration)?;
        if let Some(index) = self.extension_sampling_frequency_index {
            writer.write::<4, u8>(index.into())?;
        }
        if let Some(frequency) = self.extension_sampling_frequency {
            writer.write::<24, u32>(frequency)?;
        }
        if let Some(configuration) = self.extension_channel_configuration {
            writer.write::<4, u8>(configuration)?;
        }
        self.specific_config.write_to(writer)?;
        if let Some(ep_config) = self.ep_config.as_ref() {
            ep_config.write_to(writer)?;
        }
        if let Some(sync_extension_type) = self.sync_extension_type {
            writer.write::<11, u16>(sync_extension_type)?;
        }
        if let Some(sync_extension_audio_object_type) = self.sync_extension_audio_object_type {
            sync_extension_audio_object_type.write_to(writer)?;
        }
        if let Some(sync_type5) = self.sync_extension_audio_object_type5.as_ref() {
            sync_type5.write_to(writer)?;
        }
        if let Some(sync_type22) = self.sync_extension_audio_object_type22.as_ref() {
            sync_type22.write_to(writer)?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for EpConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<2, u8>(self.ep_config)?;
        if let Some(config) = self.error_protection_specific_config.as_ref() {
            config.write_to(writer)?;
        }
        if let Some(mapping) = self.direct_mapping {
            writer.write_bit(mapping)?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for SyncExtensionAudioObjectType5 {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.sbr_present_flag)?;
        if let Some(index) = self.extension_sampling_frequency_index {
            writer.write::<4, u8>(index.into())?;
        }
        if let Some(freq) = self.extension_sampling_frequency {
            writer.write::<24, u32>(freq)?;
        }
        if let Some(sync_ext_type) = self.sync_extension_type {
            writer.write::<11, u16>(sync_ext_type)?;
        }
        if let Some(ps) = self.ps_present_flag {
            writer.write_bit(ps)?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for SyncExtensionAudioObjectType22 {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.sbr_present_flag)?;
        if let Some(index) = self.extension_sampling_frequency_index {
            writer.write::<4, u8>(index.into())?;
        }
        if let Some(freq) = self.extension_sampling_frequency {
            writer.write::<24, u32>(freq)?;
        }
        writer.write::<4, u8>(self.extension_channel_configuration)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for SpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::Ga(ga) => ga.write_to(writer),
            Self::Celp(celp) => celp.write_to(writer),
            Self::Hvxc(hvxc) => hvxc.write_to(writer),
            Self::TTS(tts) => tts.write_to(writer),
            Self::StructuredAudio(structured_audio) => structured_audio.write_to(writer),
            Self::ErrorResilientCelp(error_celp) => error_celp.write_to(writer),
            Self::ErrorResilientHvxc(error_hvxc) => error_hvxc.write_to(writer),
            Self::Parametric() => {
                unimplemented!("no spec on this element")
            }
            Self::SSC(ssc) => ssc.write_to(writer),
            Self::Spatial {} => {
                unimplemented!("no spec on this element")
            }
            Self::Mpeg1_2 { extension } => writer.write_bit(*extension).map_err(|err| err.into()),
            Self::DST(dst) => dst.write_to(writer),
            Self::ALS { fill_bits, config } => {
                writer.write::<5, u8>(*fill_bits)?;
                config.write_to(writer)
            }
            Self::SLS(sls) => sls.write_to(writer),
            Self::ELD(eld) => eld.write_to(writer),
            Self::SymbolicMusic() => {
                unimplemented!("no spec on this element")
            }
            Self::Reserved => Ok(()),
        }
    }
}
