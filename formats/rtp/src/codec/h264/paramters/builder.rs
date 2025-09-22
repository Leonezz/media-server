use base64::Engine;
use codec_common::video::H264VideoConfig;
use codec_h264::nalu::NalUnit;

use crate::codec::h264::paramters::{
    packetization_mode::PacketizationMode,
    profile_level_id::RtpH264FmtpProfileLevelId,
    rfc6184::{RtpH264Fmtp, SpropParameterSets},
};

#[derive(Default)]
pub struct RtpH264FmtpBuilder(RtpH264Fmtp);

impl RtpH264FmtpBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn sps(mut self, sps: codec_h264::sps::Sps) -> Self {
        if self.0.sprop_parameter_sets.is_none() {
            self.0.sprop_parameter_sets = Some(SpropParameterSets {
                raw: vec![],
                sps: None,
                pps: None,
            })
        }

        let nalu: NalUnit = (&sps).into();
        let bytes = utils::bytes::writable_to_bytes(&nalu).unwrap();
        let base64_str = base64::prelude::BASE64_STANDARD.encode(bytes);
        self.0
            .sprop_parameter_sets
            .as_mut()
            .unwrap()
            .raw
            .push(base64_str);
        self.0.profile_level_id = Some(RtpH264FmtpProfileLevelId::from(&sps));
        // TODO: sprop_level_parameter_sets ?
        self.0.sprop_parameter_sets.as_mut().unwrap().sps = Some(sps);
        self
    }

    pub fn pps(mut self, pps: codec_h264::pps::Pps) -> Self {
        if self.0.sprop_parameter_sets.is_none() {
            self.0.sprop_parameter_sets = Some(SpropParameterSets {
                raw: vec![],
                sps: None,
                pps: None,
            });
        }

        let nalu: NalUnit = (&pps).into();
        let bytes = utils::bytes::writable_to_bytes(&nalu).unwrap();
        let base64_str = base64::prelude::BASE64_STANDARD.encode(bytes);
        self.0
            .sprop_parameter_sets
            .as_mut()
            .unwrap()
            .raw
            .push(base64_str);
        self.0.sprop_parameter_sets.as_mut().unwrap().pps = Some(pps);
        self
    }

    pub fn max_recv_level(mut self, value: [u8; 2]) -> Self {
        self.0.max_recv_level = Some(value);
        self
    }

    pub fn packetization_mode(mut self, mode: PacketizationMode) -> Self {
        self.0.packetization_mode = Some(mode);
        self
    }

    pub fn sprop_deint_buf_req(mut self, req: u64) -> Self {
        self.0.sprop_deint_buf_req = Some(req);
        self
    }

    pub fn sprop_interleaving_depth(mut self, depth: u16) -> Self {
        self.0.sprop_interleaving_depth = Some(depth);
        self
    }

    pub fn sprop_max_don_diff(mut self, diff: u16) -> Self {
        self.0.sprop_max_don_diff = Some(diff);
        self
    }

    pub fn build(self) -> RtpH264Fmtp {
        self.0
    }
}

impl From<&H264VideoConfig> for RtpH264FmtpBuilder {
    fn from(value: &H264VideoConfig) -> Self {
        let mut builder = RtpH264FmtpBuilder::new();
        if let Some(sps) = &value.sps {
            builder = builder.sps(sps.clone());
        }
        if let Some(pps) = &value.pps {
            builder = builder.pps(pps.clone());
        }
        builder
    }
}
