#[derive(Debug, Clone, Copy)]
pub struct RtpH264FmtpProfileLevelId {
    pub profile_idc: u8,
    pub constraint_set0_flag: bool,
    pub constraint_set1_flag: bool,
    pub constraint_set2_flag: bool,
    pub constraint_set3_flag: bool,
    pub constraint_set4_flag: bool,
    pub constraint_set5_flag: bool,
    pub reserved_zero_2bits: u8, // 2 bits
    pub level_idc: u8,
}

impl From<&codec_h264::sps::Sps> for RtpH264FmtpProfileLevelId {
    fn from(value: &codec_h264::sps::Sps) -> Self {
        Self {
            profile_idc: value.profile_idc,
            constraint_set0_flag: value.constraint_set0_flag,
            constraint_set1_flag: value.constraint_set1_flag,
            constraint_set2_flag: value.constraint_set2_flag,
            constraint_set3_flag: value.constraint_set3_flag,
            constraint_set4_flag: value.constraint_set4_flag,
            constraint_set5_flag: value.constraint_set5_flag,
            reserved_zero_2bits: value.reserved_zero_2bits,
            level_idc: value.level_idc,
        }
    }
}

impl From<[u8; 3]> for RtpH264FmtpProfileLevelId {
    fn from(value: [u8; 3]) -> Self {
        let profile_level_id = value[0];
        let constraint_set0_flag = (value[1] >> 7) & 0b1 == 0b1;
        let constraint_set1_flag = (value[1] >> 6) & 0b1 == 0b1;
        let constraint_set2_flag = (value[1] >> 5) & 0b1 == 0b1;
        let constraint_set3_flag = (value[1] >> 4) & 0b1 == 0b1;
        let constraint_set4_flag = (value[1] >> 3) & 0b1 == 0b1;
        let constraint_set5_flag = (value[1] >> 2) & 0b1 == 0b1;
        let reserved_zero_2bits = value[1] & 0b11;
        let level_idc = value[2];
        Self {
            profile_idc: profile_level_id,
            constraint_set0_flag,
            constraint_set1_flag,
            constraint_set2_flag,
            constraint_set3_flag,
            constraint_set4_flag,
            constraint_set5_flag,
            reserved_zero_2bits,
            level_idc,
        }
    }
}

impl From<RtpH264FmtpProfileLevelId> for [u8; 3] {
    fn from(value: RtpH264FmtpProfileLevelId) -> Self {
        let b1 = value.profile_idc;
        let mut b2 = 0u8;
        if value.constraint_set0_flag {
            b2 |= 1 << 7;
        }
        if value.constraint_set1_flag {
            b2 |= 1 << 6;
        }
        if value.constraint_set2_flag {
            b2 |= 1 << 5;
        }
        if value.constraint_set3_flag {
            b2 |= 1 << 4;
        }
        if value.constraint_set4_flag {
            b2 |= 1 << 3;
        }
        if value.constraint_set5_flag {
            b2 |= 1 << 2;
        }
        b2 |= value.reserved_zero_2bits & 0b11;
        let b3 = value.level_idc;
        [b1, b2, b3]
    }
}
