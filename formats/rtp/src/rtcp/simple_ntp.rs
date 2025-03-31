use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, Default)]
pub struct SimpleNtp {
    seconds: u32,
    fraction: u32,
}

impl From<u64> for SimpleNtp {
    fn from(value: u64) -> Self {
        Self {
            seconds: ((value >> 32) & 0xFFFF_FFFF) as u32,
            fraction: (value & 0xFFFF_FFFF) as u32,
        }
    }
}

impl From<SimpleNtp> for u64 {
    fn from(value: SimpleNtp) -> Self {
        ((value.seconds as u64) << 32) | (value.fraction as u64)
    }
}

impl From<SystemTime> for SimpleNtp {
    fn from(value: SystemTime) -> Self {
        let duration = value
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_nanos() as u64;
        let mut seconds = duration / 1_000_000_000;
        seconds += 0x83AA7E80; //offset in seconds between unix epoch and ntp epoch
        let mut fraction = duration % 1_000_000_000;
        fraction <<= 32;
        fraction /= 1_000_000_000;
        seconds <<= 32;
        Self {
            seconds: seconds as u32,
            fraction: fraction as u32,
        }
    }
}

impl From<SimpleNtp> for SystemTime {
    fn from(value: SimpleNtp) -> Self {
        let value: u64 = value.into();
        let mut seconds = value >> 32;
        let mut fraction = value & 0xFFFF_FFFF;
        fraction *= 1_000_000_000;
        fraction >>= 32;
        seconds -= 0x83AA7E80;
        let duration = seconds * 1_000_000_000 + fraction;

        UNIX_EPOCH
            .checked_add(Duration::new(
                duration / 1_000_000_000,
                (duration % 1_000_000_000) as u32,
            ))
            .unwrap_or(UNIX_EPOCH)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SimpleShortNtp {
    seconds: u16,
    fraction: u16,
}

impl From<u32> for SimpleShortNtp {
    fn from(value: u32) -> Self {
        Self {
            seconds: ((value >> 16) & 0xFFFF) as u16,
            fraction: (value & 0xFFFF) as u16,
        }
    }
}

impl From<SimpleNtp> for SimpleShortNtp {
    fn from(value: SimpleNtp) -> Self {
        Self {
            seconds: value.seconds as u16,
            fraction: value.fraction as u16,
        }
    }
}

impl From<SimpleShortNtp> for u32 {
    fn from(value: SimpleShortNtp) -> Self {
        ((value.seconds as u32) << 16) | (value.fraction as u32)
    }
}

impl From<SimpleShortNtp> for SimpleNtp {
    fn from(value: SimpleShortNtp) -> Self {
        SimpleNtp {
            seconds: value.seconds as u32,
            fraction: value.fraction as u32,
        }
    }
}

impl From<SystemTime> for SimpleShortNtp {
    fn from(value: SystemTime) -> Self {
        let ntp: SimpleNtp = value.into();
        let bits: u64 = ntp.into();
        let short_bits = ((bits >> 16) & 0xFFFF_FFFF) as u32;
        Self::from(short_bits)
    }
}

impl From<SimpleShortNtp> for SystemTime {
    fn from(value: SimpleShortNtp) -> Self {
        let ntp: SimpleNtp = value.into();
        ntp.into()
    }
}
