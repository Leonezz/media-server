use bitstream_io::BitRead;
use num::ToPrimitive;
use utils::traits::reader::{BitwiseReadFrom, BitwiseReadReaminingFrom};

use crate::{
    errors::AACCodecError,
    mpeg4_configuration::{
        eld_specific_config::{EldExtData, LdSbr},
        sbr_header::SbrHeader,
    },
};

use super::{ELDEXT_TERM, ELDSpecificConfig, LdSbrHeader};

impl<R: BitRead> BitwiseReadReaminingFrom<u8, R> for ELDSpecificConfig {
    type Error = AACCodecError;
    fn read_remaining_from(channel_configuration: u8, reader: &mut R) -> Result<Self, Self::Error> {
        let frame_length_flag = reader.read_bit()?;
        let aac_section_data_resilience_flag = reader.read_bit()?;
        let aac_scalefactor_data_resilience_flag = reader.read_bit()?;
        let aac_spectral_data_resilience_flag = reader.read_bit()?;
        let ld_sbr_present_flag = reader.read_bit()?;
        let ld_sbr = if ld_sbr_present_flag {
            Some(LdSbr::read_remaining_from(channel_configuration, reader)?)
        } else {
            None
        };
        let mut eld_ext_data = vec![];
        loop {
            let eld_ext_type = reader.read::<4, u8>()?;
            if eld_ext_type == ELDEXT_TERM {
                break;
            }
            eld_ext_data.push(EldExtData::read_remaining_from(eld_ext_type, reader)?);
        }
        Ok(Self {
            frame_length_flag,
            aac_section_data_resilience_flag,
            aac_scalefactor_data_resilience_flag,
            aac_spectral_data_resilience_flag,
            ld_sbr_present_flag,
            ld_sbr,
            eld_ext_data,
        })
    }
}

impl<R: BitRead> BitwiseReadReaminingFrom<u8, R> for EldExtData {
    type Error = AACCodecError;
    fn read_remaining_from(eld_ext_type: u8, reader: &mut R) -> Result<Self, Self::Error> {
        let mut eld_ext_len = reader.read::<4, u32>()?;
        if eld_ext_len == 15 {
            eld_ext_len += reader.read::<8, u32>()?;
        }
        if eld_ext_len == 255 {
            eld_ext_len += reader.read::<16, u32>()?;
        }
        let mut other_byte = vec![0; eld_ext_len.to_usize().unwrap()];
        reader.read_bytes(&mut other_byte)?;
        Ok(Self {
            eld_ext_type,
            other_byte,
        })
    }
}

impl<R: BitRead> BitwiseReadReaminingFrom<u8, R> for LdSbr {
    type Error = AACCodecError;
    fn read_remaining_from(channel_configuration: u8, reader: &mut R) -> Result<Self, Self::Error> {
        let ld_sbr_sampling_rate = reader.read_bit()?;
        let ld_sbr_crc_flag = reader.read_bit()?;
        let ld_sbr_header = LdSbrHeader::read_remaining_from(channel_configuration, reader)?;
        Ok(Self {
            ld_sbr_sampling_rate,
            ld_sbr_crc_flag,
            ld_sbr_header,
        })
    }
}

impl<R: BitRead> BitwiseReadReaminingFrom<u8, R> for LdSbrHeader {
    type Error = AACCodecError;
    fn read_remaining_from(channel_configuration: u8, reader: &mut R) -> Result<Self, Self::Error> {
        let num_sbr_header = match channel_configuration {
            1 | 2 => 1,
            3 => 2,
            4..=6 => 3,
            7 => 4,
            _ => 0,
        };
        let mut sbr_headers = vec![];
        for _ in 0..num_sbr_header {
            sbr_headers.push(SbrHeader::read_from(reader)?);
        }
        Ok(Self {
            num_sbr_header,
            sbr_headers,
        })
    }
}
