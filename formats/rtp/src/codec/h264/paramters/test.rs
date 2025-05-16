#[cfg(test)]
mod tests {
    use crate::codec::h264::paramters::{
        H264SDPFormatParameters, packetization_mode::PacketizationMode,
    };

    #[test]
    fn test_simple() {
        let parameters = "profile-level-id=42e016;max-mbps=108000;max-fs=3600";
        let parsed: H264SDPFormatParameters = parameters.parse().unwrap();
        assert_eq!(parsed.profile_level_id, Some([0x42, 0xe0, 0x16]));
        assert_eq!(parsed.max_mbps, Some(108000));
        assert_eq!(parsed.max_fs, Some(3600));

        let serialized = parsed.to_string();
        assert_eq!(serialized, parameters);
    }

    #[test]
    fn test_simple2() {
        let parameters = "profile-level-id=42A01E; packetization-mode=2; sprop-parameter-sets=Z2QAHqzZQNg95vARAAADAAEAAAMAMA8WLZY=,aO+Pyw==; sprop-interleaving-depth=45; sprop-deint-buf-req=64000; sprop-init-buf-time=102478; deint-buf-cap=128000";
        let parsed: H264SDPFormatParameters = parameters.parse().unwrap();
        assert_eq!(parsed.profile_level_id, Some([0x42, 0xa0, 0x1e]));
        assert_eq!(
            parsed.packetization_mode,
            Some(PacketizationMode::Interleaved)
        );
        assert_eq!(
            parsed.sprop_parameter_sets.unwrap().raw,
            vec![
                "Z2QAHqzZQNg95vARAAADAAEAAAMAMA8WLZY=".to_string(),
                "aO+Pyw==".to_string()
            ]
        );
        assert_eq!(parsed.sprop_interleaving_depth, Some(45));
        assert_eq!(parsed.sprop_deint_buf_req, Some(64000));
        assert_eq!(parsed.deint_buf_cap, Some(128000));
        assert_eq!(parsed.sprop_init_buf_time, Some(102478));
        assert_eq!(parsed.max_mbps, None);
    }
}
