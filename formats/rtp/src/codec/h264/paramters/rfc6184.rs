use crate::codec::h264::paramters::{
    errors::H264SDPError, packetization_mode::PacketizationMode,
    profile_level_id::RtpH264FmtpProfileLevelId,
};
use codec_h264::{
    avc_decoder_configuration_record::{
        AvcDecoderConfigurationRecord, ParameterSetInAvcDecoderConfigurationRecord, SpsExtRelated,
    },
    nalu::NalUnit,
    pps::Pps,
    sps::{Sps, chroma_format_idc::ChromaFormatIdc},
};
use num::ToPrimitive;
use sdp_formats::{attributes::extension::SdpAttributeExtension, errors::SDPError};
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

#[derive(Debug, Clone)]
pub struct SpropParameterSets {
    pub raw: Vec<String>,
    pub sps: Option<Sps>,
    pub pps: Option<Pps>,
}

#[derive(Debug, Clone, Default)]
pub struct RtpH264Fmtp {
    pub fmt: u8,                                             // compatibility only
    pub profile_level_id: Option<RtpH264FmtpProfileLevelId>, // TODO: make this a concrete level id
    pub max_recv_level: Option<[u8; 2]>,
    pub packetization_mode: Option<PacketizationMode>, // 0, 1, 2. default to 0
    pub sprop_deint_buf_req: Option<u64>,              // in [0, 4294967295]
    pub sprop_interleaving_depth: Option<u16>,         // in [0, 32767]
    pub sprop_max_don_diff: Option<u16>,               // in [0, 32767]
    pub sprop_init_buf_time: Option<u64>,              // in [0, 4294967295]
    pub max_mbps: Option<u64>,
    pub max_smbps: Option<u64>,
    pub max_fs: Option<u64>,
    pub max_cpb: Option<u64>,
    pub max_dpb: Option<u64>,
    pub max_br: Option<u64>,
    pub redundant_pic_cap: Option<bool>, // default to 0
    pub deint_buf_cap: Option<u64>,      // in [0, 4294967295]
    pub max_rcmd_nalu_size: Option<u64>, // in [0, 4294967295]
    pub sar_understood: Option<u8>,      // default to 13
    pub sar_supported: Option<u8>,
    pub in_band_parameter_sets: Option<bool>,
    pub use_level_src_parameter_sets: Option<bool>, // default to 0
    pub level_asymmetry_allowed: Option<bool>,      // default to 0
    pub sprop_parameter_sets: Option<SpropParameterSets>,
    pub sprop_level_parameter_sets: Vec<([u8; 3], Vec<String>)>,
    pub unknown: Vec<String>,
}

impl TryFrom<&RtpH264Fmtp> for AvcDecoderConfigurationRecord {
    type Error = H264SDPError;
    fn try_from(value: &RtpH264Fmtp) -> Result<Self, Self::Error> {
        let sps = if let Some(params) = value.sprop_parameter_sets.as_ref()
            && let Some(sps) = params.sps.as_ref()
        {
            vec![sps]
        } else {
            vec![]
        };
        let pps = if let Some(params) = value.sprop_parameter_sets.as_ref()
            && let Some(pps) = params.pps.as_ref()
        {
            vec![pps]
        } else {
            vec![]
        };
        let profile_idc = value.get_profile_idc().ok_or(
            H264SDPError::FmptToAvcDecoderConfigurationRecordError(format!(
                "no profile_idc found: {:?}",
                value
            )),
        )?;
        let sps_ext_related = match profile_idc {
            100 | 110 | 122 | 144 => Some(
                SpsExtRelated::builder()
                    .chroma_format_idc(value.get_chroma_format_idc().ok_or(
                        H264SDPError::FmptToAvcDecoderConfigurationRecordError(format!(
                            "no chroma_format_idc found: {:?}",
                            value
                        )),
                    )?)
                    .bit_depth_chroma_minus8(
                        value
                            .get_bit_depth_chroma_minus8()
                            .ok_or(H264SDPError::FmptToAvcDecoderConfigurationRecordError(
                                format!("no bit_depth_chroma_minus8 found: {:?}", value),
                            ))?
                            .to_u8()
                            .unwrap(),
                    )
                    .bit_depth_luma_minus8(
                        value
                            .get_bit_depth_luma_minus8()
                            .ok_or(H264SDPError::FmptToAvcDecoderConfigurationRecordError(
                                format!("no bit_depth_luma_minus8 found: {:?}", value),
                            ))?
                            .to_u8()
                            .unwrap(),
                    )
                    .build(),
            ),
            _ => None,
        };

        Ok(Self {
            configuration_version: 1,
            avc_profile_indication: profile_idc,
            avc_level_indication: value.get_level_idc().ok_or(
                H264SDPError::FmptToAvcDecoderConfigurationRecordError(format!(
                    "no level_idc found: {:?}",
                    value
                )),
            )?,
            profile_compatibility: value.get_profile_compatibility().ok_or(
                H264SDPError::FmptToAvcDecoderConfigurationRecordError(format!(
                    "no profile_compatibility found: {:?}",
                    value
                )),
            )?,
            length_size_minus_one: 3,
            reserved_3_bits_1: 0b111,
            reserved_6_bits_1: 0b111111,
            num_of_sequence_parameter_sets: sps.len().to_u8().unwrap(),
            sequence_parameter_sets: sps
                .into_iter()
                .map(|p| {
                    let nalu: NalUnit = p.into();
                    ParameterSetInAvcDecoderConfigurationRecord {
                        sequence_parameter_set_length: nalu
                            .get_packet_bytes_count()
                            .to_u16()
                            .unwrap(),
                        parameter_set: p.clone(),
                    }
                })
                .collect(),
            num_of_picture_parameter_sets: pps.len().to_u8().unwrap(),
            picture_parameter_sets: pps
                .into_iter()
                .map(|p| {
                    let nalu: NalUnit = p.into();
                    ParameterSetInAvcDecoderConfigurationRecord {
                        sequence_parameter_set_length: nalu
                            .get_packet_bytes_count()
                            .to_u16()
                            .unwrap(),
                        parameter_set: p.clone(),
                    }
                })
                .collect(),
            sps_ext_related,
        })
    }
}

impl RtpH264Fmtp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_profile_idc(&self) -> Option<u8> {
        if let Some(profile_level_id) = self.profile_level_id {
            return Some(profile_level_id.profile_idc);
        }

        if let Some(parameter_sets) = self.sprop_parameter_sets.as_ref()
            && let Some(sps) = parameter_sets.sps.as_ref()
        {
            return Some(sps.profile_idc);
        }

        None
    }

    /// profile_compatibility is a byte defined exactly the same as the byte
    /// which occurs between the profile_IDC and level_IDC in a sequence parameter set (SPS),
    /// as defined in ISO/IEC 14496-10.
    pub fn get_profile_compatibility(&self) -> Option<u8> {
        if let Some(profile_level_id) = self.profile_level_id {
            let bytes: [u8; 3] = profile_level_id.into();
            return Some(bytes[1]);
        }

        if let Some(parameter_sets) = self.sprop_parameter_sets.as_ref()
            && let Some(sps) = parameter_sets.sps.as_ref()
        {
            let mut b2 = 0u8;
            if sps.constraint_set0_flag {
                b2 |= 1 << 7;
            }
            if sps.constraint_set1_flag {
                b2 |= 1 << 6;
            }
            if sps.constraint_set2_flag {
                b2 |= 1 << 5;
            }
            if sps.constraint_set3_flag {
                b2 |= 1 << 4;
            }
            if sps.constraint_set4_flag {
                b2 |= 1 << 3;
            }
            if sps.constraint_set5_flag {
                b2 |= 1 << 2;
            }
            b2 |= sps.reserved_zero_2bits & 0b11;
            return Some(b2);
        }
        None
    }

    pub fn get_level_idc(&self) -> Option<u8> {
        if let Some(profile_level_id) = self.profile_level_id {
            return Some(profile_level_id.level_idc);
        }
        if let Some(parameters) = self.sprop_parameter_sets.as_ref()
            && let Some(sps) = parameters.sps.as_ref()
        {
            return Some(sps.level_idc);
        }
        None
    }

    pub fn get_chroma_format_idc(&self) -> Option<ChromaFormatIdc> {
        if let Some(params) = self.sprop_parameter_sets.as_ref()
            && let Some(sps) = params.sps.as_ref()
        {
            return sps.get_chroma_format_idc();
        }
        None
    }

    pub fn get_bit_depth_chroma_minus8(&self) -> Option<u64> {
        if let Some(params) = self.sprop_parameter_sets.as_ref()
            && let Some(sps) = params.sps.as_ref()
        {
            return sps.get_bit_depth_chroma_minus8();
        }
        None
    }
    pub fn get_bit_depth_luma_minus8(&self) -> Option<u64> {
        if let Some(params) = self.sprop_parameter_sets.as_ref()
            && let Some(sps) = params.sps.as_ref()
        {
            return sps.get_bit_depth_luma_minus8();
        }
        None
    }
}

impl TryFrom<&sdp_formats::attributes::fmtp::FormatParameters> for RtpH264Fmtp {
    type Error = SDPError;
    fn try_from(
        value: &sdp_formats::attributes::fmtp::FormatParameters,
    ) -> Result<Self, Self::Error> {
        let mut result: Self = value.params.parse().map_err(|err| {
            sdp_formats::errors::SDPError::InvalidAttributeExtension(format!(
                "parse to h264 fmtp failed: {}",
                err
            ))
        })?;
        result.fmt = value.fmt;
        Ok(result)
    }
}

impl From<RtpH264Fmtp> for sdp_formats::attributes::fmtp::FormatParameters {
    fn from(value: RtpH264Fmtp) -> Self {
        Self {
            fmt: value.fmt,
            params: format!("{}", value),
        }
    }
}

impl SdpAttributeExtension for RtpH264Fmtp {
    fn attr_name() -> &'static str {
        "fmtp"
    }

    fn into_attr(self) -> sdp_formats::attributes::SDPAttribute {
        sdp_formats::attributes::SDPAttribute::Fmtp(self.into())
    }

    fn try_from_attr(
        attr: &sdp_formats::attributes::SDPAttribute,
    ) -> Result<Self, sdp_formats::errors::SDPError> {
        match attr {
            sdp_formats::attributes::SDPAttribute::Fmtp(fmtp) => Self::try_from(fmtp),
            _ => Err(sdp_formats::errors::SDPError::InvalidAttributeExtension(
                format!("{} not match h264 fmtp extension", attr),
            )),
        }
    }

    fn value(&self) -> Option<String> {
        Some(format!("{} {}", self.fmt, self))
    }
}
