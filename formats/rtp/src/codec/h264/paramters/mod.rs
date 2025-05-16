pub mod errors;
pub mod packetization_mode;
#[cfg(test)]
mod test;

use std::{fmt, str::FromStr};

use base64::Engine;
use codec_h264::{
    nalu::NalUnit,
    nalu_type::NALUType,
    pps::Pps,
    rbsp::RbspReader,
    sps::{Sps, chroma_format_idc::ChromaFormatIdc},
};
use errors::H264SDPError;
use itertools::Itertools;
use packetization_mode::PacketizationMode;
use utils::traits::reader::{BitwiseReadFrom, BitwiseReadReaminingFrom, ReadFrom};

#[derive(Debug, Clone)]
pub struct SpropParameterSets {
    pub raw: Vec<String>,
    pub sps: Option<Sps>,
    pub pps: Option<Pps>,
}

#[derive(Debug, Clone, Default)]
pub struct H264SDPFormatParameters {
    pub profile_level_id: Option<[u8; 3]>, // TODO: make this a concrete level id
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

impl H264SDPFormatParameters {
    pub fn new() -> Self {
        Self::default()
    }
}

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

impl FromStr for H264SDPFormatParameters {
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
                    result.profile_level_id = Some(parse_profile_level_id(value)?)
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
                        let nalu = NalUnit::read_from(&mut bytes.as_slice()).map_err(|err| H264SDPError::InvalidSpropLevelParameterSets(
                            format!("sprop-parameter-sets value parse as nalu failed: {}, err={}", item, err)
                        ))?;
                        let mut reader = RbspReader::new(&nalu.body[..]);
                        match nalu.header.nal_unit_type {
                            NALUType::SPS => {
                                result.sprop_parameter_sets.as_mut().unwrap().sps = Some(Sps::read_from(&mut reader).map_err(|err| {
                                    H264SDPError::InvalidSpropParameterSets(format!(
                                        "sprop-parameter-sets value parse as sps failed: {}, err={}",
                                        item, err
                                    ))
                                })?);
                            },
                            NALUType::PPS => {
                                result.sprop_parameter_sets.as_mut().unwrap().pps = Some(
                                    Pps::read_remaining_from(
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
                                    })?
                                )
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

impl fmt::Display for H264SDPFormatParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result: Vec<String> = Vec::new();
        if let Some(profile_level_id) = self.profile_level_id {
            result.push(format!(
                "profile-level-id={:02x}{:02x}{:02x}",
                profile_level_id[0], profile_level_id[1], profile_level_id[2]
            ));
        }
        if let Some(max_recv_level) = self.max_recv_level {
            result.push(format!(
                "max-recv-level={:02x}{:02x}",
                max_recv_level[0], max_recv_level[1]
            ));
        }
        if let Some(packetization_mode) = self.packetization_mode {
            result.push(format!("packetization-mode={}", packetization_mode));
        }
        if let Some(sprop_deint_buf_req) = self.sprop_deint_buf_req {
            result.push(format!("sprop-deint-buf-req={}", sprop_deint_buf_req));
        }
        if let Some(sprop_interleaving_depth) = self.sprop_interleaving_depth {
            result.push(format!(
                "sprop-interleaving-depth={}",
                sprop_interleaving_depth
            ));
        }
        if let Some(sprop_max_don_diff) = self.sprop_max_don_diff {
            result.push(format!("sprop-max-don-diff={}", sprop_max_don_diff));
        }
        if let Some(sprop_init_buf_time) = self.sprop_init_buf_time {
            result.push(format!("sprop-init-buf-time={}", sprop_init_buf_time));
        }
        if let Some(max_mbps) = self.max_mbps {
            result.push(format!("max-mbps={}", max_mbps));
        }
        if let Some(max_smbps) = self.max_smbps {
            result.push(format!("max-smbps={}", max_smbps));
        }
        if let Some(max_fs) = self.max_fs {
            result.push(format!("max-fs={}", max_fs));
        }
        if let Some(max_cpb) = self.max_cpb {
            result.push(format!("max-cpb={}", max_cpb));
        }
        if let Some(max_dpb) = self.max_dpb {
            result.push(format!("max-dpb={}", max_dpb));
        }
        if let Some(max_br) = self.max_br {
            result.push(format!("max-br={}", max_br));
        }
        if let Some(redundant_pic_cap) = self.redundant_pic_cap {
            result.push(format!("redundant-pic-cap={}", redundant_pic_cap as u8));
        }
        if let Some(deint_buf_cap) = self.deint_buf_cap {
            result.push(format!("deint-buf-cap={}", deint_buf_cap));
        }
        if let Some(max_rcmd_nalu_size) = self.max_rcmd_nalu_size {
            result.push(format!("max-rcmd-nalu-size={}", max_rcmd_nalu_size));
        }
        if let Some(sar_understood) = self.sar_understood {
            result.push(format!("sar-understood={}", sar_understood));
        }
        if let Some(sar_supported) = self.sar_supported {
            result.push(format!("sar-supported={}", sar_supported));
        }
        if let Some(in_band_parameter_sets) = self.in_band_parameter_sets {
            result.push(format!(
                "in-band-parameter-sets={}",
                in_band_parameter_sets as u8
            ));
        }
        if let Some(use_level_src_parameter_sets) = self.use_level_src_parameter_sets {
            result.push(format!(
                "use-level-src-parameter-sets={}",
                use_level_src_parameter_sets as u8
            ));
        }
        if let Some(level_asymmetry_allowed) = self.level_asymmetry_allowed {
            result.push(format!(
                "level-asymmetry-allowed={}",
                level_asymmetry_allowed as u8
            ));
        }
        if let Some(sprop_parameter_sets) = &self.sprop_parameter_sets {
            let sprop_parameter_sets = sprop_parameter_sets
                .raw
                .iter()
                .join(",");
            result.push(format!("sprop-parameter-sets={}", sprop_parameter_sets));
        }

        if !self.sprop_level_parameter_sets.is_empty() {
            let sprop_level_parameter_sets = self
                .sprop_level_parameter_sets
                .iter()
                .map(|(plid, psls)| {
                    format!(
                        "{:02x}{:02x}{:02x}:{}",
                        plid[0],
                        plid[1],
                        plid[2],
                        psls.join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join(":");
            result.push(format!(
                "sprop-level-parameter-sets={}",
                sprop_level_parameter_sets
            ));
        }
        if !self.unknown.is_empty() {
            result.extend(self.unknown.clone());
        }
        write!(f, "{}", result.join(";"))
    }
}
