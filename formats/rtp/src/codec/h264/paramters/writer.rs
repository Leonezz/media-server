use std::fmt;

use itertools::Itertools;

use crate::codec::h264::paramters::rfc6184::RtpH264Fmtp;

impl fmt::Display for RtpH264Fmtp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result: Vec<String> = Vec::new();
        if let Some(profile_level_id) = self.profile_level_id {
            let bytes: [u8; 3] = profile_level_id.into();
            result.push(format!(
                "profile-level-id={:02x}{:02x}{:02x}",
                bytes[0], bytes[1], bytes[2]
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
            let sprop_parameter_sets = sprop_parameter_sets.raw.iter().join(",");
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
