use std::str::FromStr;
use base64::Engine;
use codec_bitstream::reader::BitstreamReader;
use codec_h264::{nalu::NalUnit, nalu_type::NALUType, pps::Pps, sps::{chroma_format_idc::ChromaFormatIdc, Sps}};
use itertools::Itertools;
use utils::traits::reader::{BitwiseReadFrom, BitwiseReadReaminingFrom, ReadFrom};
use crate::codec::h264::paramters::{errors::H264SDPError, rfc6184::{RtpH264Fmtp, SpropParameterSets}};

fn parse_profile_level_id(value: &str) -> Result<[u8; 3], H264SDPError> {
    if value.len() != 6 {
        return Err(H264SDPError::InvalidProfileLevelId(format!(
            "profile level id is not of 6 bytes: {}",
            value
        )));
    }
    let mut result = [0u8; 3];
    for i in 0..3 {
        result[i] = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).map_err(|_| {
            H264SDPError::InvalidProfileLevelId(format!("invalid hex value: {}", value))
        })?;
    }
    Ok(result)
}

fn parse_max_recv_level(value: &str) -> Result<[u8; 2], H264SDPError> {
    if value.len() != 4 {
        return Err(H264SDPError::InvalidMaxRecvLevel(format!(
            "max recv level is not of 4 bytes: {}",
            value
        )));
    }
    let mut result = [0u8; 2];
    for i in 0..2 {
        result[i] = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).map_err(|_| {
            H264SDPError::InvalidMaxRecvLevel(format!("invalid hex value: {}", value))
        })?;
    }
    Ok(result)
}

impl FromStr for RtpH264Fmtp {
    type Err = H264SDPError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = Self::default();
        for item in s.split(";") {
            let (key, value) = item
                .trim()
                .split_once("=")
                .ok_or(H264SDPError::InvalidFormat(format!(
                    "no key value pair found: {}",
                    item
                )))?;
            match key {
                "profile-level-id" => {
                    result.profile_level_id = Some(parse_profile_level_id(value)?.into())
                }
                "max-recv-level" => result.max_recv_level = Some(parse_max_recv_level(value)?),
                "packetization-mode" => {
                    if value != "0" && value != "1" && value != "2" {
                        return Err(H264SDPError::InvalidPacketizationMode(format!(
                            "invalid packetization mode: {}",
                            value
                        )));
                    }
                    result.packetization_mode = Some(value.parse().unwrap());
                }
                "sprop-deint-buf-req" => {
                    let value = value.parse::<u64>().map_err(|_| {
                        H264SDPError::InvalidSpropDeintBufReq(format!(
                            "invalid sprop-deint-buf-req: {}",
                            value
                        ))
                    })?;
                    if value > 4294967295 {
                        return Err(H264SDPError::InvalidSpropDeintBufReq(format!(
                            "sprop-deint-buf-req out of range: {}",
                            value
                        )));
                    }
                    result.sprop_deint_buf_req = Some(value);
                }
                "sprop-interleaving-depth" => {
                    let value = value.parse::<u16>().map_err(|_| {
                        H264SDPError::InvalidSpropInterleavingDepth(format!(
                            "invalid sprop-interleaving-depth: {}",
                            value
                        ))
                    })?;
                    if value > 32767 {
                        return Err(H264SDPError::InvalidSpropInterleavingDepth(format!(
                            "sprop-interleaving-depth out of range: {}",
                            value
                        )));
                    }
                    result.sprop_interleaving_depth = Some(value);
                }
                "sprop-max-don-diff" => {
                    let value = value.parse::<u16>().map_err(|_| {
                        H264SDPError::InvalidSpropMaxDonDiff(format!(
                            "invalid sprop-max-don-diff: {}",
                            value
                        ))
                    })?;
                    if value > 32767 {
                        return Err(H264SDPError::InvalidSpropMaxDonDiff(format!(
                            "sprop-max-don-diff out of range: {}",
                            value
                        )));
                    }
                    result.sprop_max_don_diff = Some(value);
                }
                "sprop-init-buf-time" => {
                    let value = value.parse::<u64>().map_err(|_| {
                        H264SDPError::InvalidSpropInitBufTime(format!(
                            "invalid sprop-init-buf-time: {}",
                            value
                        ))
                    })?;
                    if value > 4294967295 {
                        return Err(H264SDPError::InvalidSpropInitBufTime(format!(
                            "sprop-init-buf-time out of range: {}",
                            value
                        )));
                    }
                    result.sprop_init_buf_time = Some(value);
                }
                "max-mbps" => {
                    result.max_mbps = Some(value.parse::<u64>().map_err(|_| {
                        H264SDPError::InvalidMaxMbps(format!("invalid max-mbps: {}", value))
                    })?);
                }
                "max-smbps" => {
                    result.max_smbps = Some(value.parse::<u64>().map_err(|_| {
                        H264SDPError::InvalidMaxSmbps(format!("invalid max-smbps: {}", value))
                    })?);
                }
                "max-fs" => {
                    result.max_fs = Some(value.parse::<u64>().map_err(|_| {
                        H264SDPError::InvalidMaxFs(format!("invalid max-fs: {}", value))
                    })?);
                }
                "max-cpb" => {
                    result.max_cpb = Some(value.parse::<u64>().map_err(|_| {
                        H264SDPError::InvalidMaxCpb(format!("invalid max-cpb: {}", value))
                    })?);
                }
                "max-dpb" => {
                    result.max_dpb = Some(value.parse::<u64>().map_err(|_| {
                        H264SDPError::InvalidMaxDpb(format!("invalid max-dpb: {}", value))
                    })?);
                }
                "max-br" => {
                    result.max_br = Some(value.parse::<u64>().map_err(|_| {
                        H264SDPError::InvalidMaxBr(format!("invalid max-br: {}", value))
                    })?);
                }
                "redundant-pic-cap" => {
                    result.redundant_pic_cap = Some(
                        value.parse::<u8>().map_err(|_| {
                            H264SDPError::InvalidRedundantPicCap(format!(
                                "invalid redundant-pic-cap: {}",
                                value
                            ))
                        })? != 0,
                    );
                }
                "deint-buf-cap" => {
                    let value = value.parse::<u64>().map_err(|_| {
                        H264SDPError::InvalidDeintBufCap(format!(
                            "invalid deint-buf-cap: {}",
                            value
                        ))
                    })?;
                    if value > 4294967295 {
                        return Err(H264SDPError::InvalidDeintBufCap(format!(
                            "deint-buf-cap out of range: {}",
                            value
                        )));
                    }
                    result.deint_buf_cap = Some(value);
                }
                "max-rcmd-nalu-size" => {
                    let value = value.parse::<u64>().map_err(|_| {
                        H264SDPError::InvalidMaxRcmdNaluSize(format!(
                            "invalid max-rcmd-nalu-size: {}",
                            value
                        ))
                    })?;
                    if value > 4294967295 {
                        return Err(H264SDPError::InvalidMaxRcmdNaluSize(format!(
                            "max-rcmd-nalu-size out of range: {}",
                            value
                        )));
                    }
                    result.max_rcmd_nalu_size = Some(value);
                }
                "sar-understood" => {
                    result.sar_understood = Some(value.parse::<u8>().map_err(|_| {
                        H264SDPError::InvalidSarUnderstood(format!(
                            "invalid sar-understood: {}",
                            value
                        ))
                    })?);
                }
                "sar-supported" => {
                    result.sar_supported = Some(value.parse::<u8>().map_err(|_| {
                        H264SDPError::InvalidSarSupported(format!(
                            "invalid sar-supported: {}",
                            value
                        ))
                    })?);
                }
                "in-band-parameter-sets" => {
                    result.in_band_parameter_sets = Some(
                        value.parse::<u8>().map_err(|_| {
                            H264SDPError::InvalidInBandParameterSets(format!(
                                "invalid in-band-parameter-sets: {}",
                                value
                            ))
                        })? != 0,
                    );
                }
                "use-level-src-parameter-sets" => {
                    result.use_level_src_parameter_sets = Some(
                        value.parse::<u8>().map_err(|_| {
                            H264SDPError::InvalidUseLevelSrcParameterSets(format!(
                                "invalid use-level-src-parameter-sets: {}",
                                value
                            ))
                        })? != 0,
                    );
                }
                "level-asymmetry-allowed" => {
                    result.level_asymmetry_allowed = Some(
                        value.parse::<u8>().map_err(|_| {
                            H264SDPError::InvalidLevelAsymmetryAllowed(format!(
                                "invalid level-asymmetry-allowed: {}",
                                value
                            ))
                        })? != 0,
                    );
                }
                "sprop-parameter-sets" => {
                    let raw: Vec<_> = value.split(',').map(|s| s.to_owned()).collect();
                    result.sprop_parameter_sets = Some(SpropParameterSets {
                        raw: vec![],
                        sps: None,
                        pps: None,
                    });
                    raw.iter().try_for_each(|item| {
                        let bytes = base64::prelude::BASE64_STANDARD.decode(item.as_bytes()).map_err(|err| H264SDPError::InvalidSpropParameterSets(
                            format!("sprop-parameter-sets value decode as base64 failed: {}, err={}", item, err)
                        ))?;

                        tracing::debug!("sprop-parameter-sets bytes: {:x?}", &bytes);

                        let nalu = NalUnit::read_from(&mut bytes.as_slice()).map_err(|err| H264SDPError::InvalidSpropLevelParameterSets(
                            format!("sprop-parameter-sets value parse as nalu failed: {}, err={}", item, err)
                        ))?;
                        let mut reader = BitstreamReader::new(&nalu.body);
                        match nalu.header.nal_unit_type {
                            NALUType::SPS => {
                                let sps = Sps::read_from(&mut reader).map_err(|err| {
                                    H264SDPError::InvalidSpropParameterSets(format!(
                                        "sprop-parameter-sets value parse as sps failed: {}, err={}",
                                        item, err
                                    ))
                                })?;
                                result.sprop_parameter_sets.as_mut().unwrap().sps = Some(sps);
                            },
                            NALUType::PPS => {
                                let pps = Pps::read_remaining_from(
                                        result.sprop_parameter_sets.as_ref().unwrap().sps
                                        .as_ref()
                                        .map_or(ChromaFormatIdc::Chroma420, 
                                            |sps| sps.profile_idc_related.as_ref()
                                            .map_or(ChromaFormatIdc::Chroma420, 
                                                |p| p.chroma_format_idc)),
                                        &mut reader,
                                    ).map_err(|err| {
                                        H264SDPError::InvalidSpropParameterSets(format!(
                                            "sprop-parameter-sets value parse as pps failed: {}, err={}",
                                            item, err
                                        ))
                                    })?;
                                result.sprop_parameter_sets.as_mut().unwrap().pps = Some(pps);
                            },
                            t => {
                                return Err(H264SDPError::InvalidSpropParameterSets(
                                    format!("sprop-parameter-sets value is not SPS or PPS: {}, nalu type: {:?}", item, t)
                                ));
                            }
                        }
                        Ok(())
                    })?;
                    result.sprop_parameter_sets.as_mut().unwrap().raw = raw;
                }
                "sprop-level-parameter-sets" => {
                    let split = value.split(":").collect::<Vec<_>>();
                    if split.len() % 2 != 0 {
                        return Err(H264SDPError::InvalidSpropLevelParameterSets(format!(
                            "invalid sprop-level-parameter-sets: {}",
                            value
                        )));
                    }
                    for (plid, psls) in split.iter().tuples() {
                        let plid = parse_profile_level_id(plid)?;
                        let psls = psls.split(',').map(|s| s.to_owned()).collect();
                        result.sprop_level_parameter_sets.push((plid, psls));
                    }
                }
                _ => {
                    tracing::warn!("unknown h264 sdp parameter: {}", item);
                    result.unknown.push(item.to_owned());
                }
            }
        }

        Ok(result)
    }
}
