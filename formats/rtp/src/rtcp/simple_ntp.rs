use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy)]
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

impl Into<u64> for SimpleNtp {
    fn into(self) -> u64 {
        ((self.seconds as u64) << 32) | (self.fraction as u64)
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

impl Into<SystemTime> for SimpleNtp {
    fn into(self) -> SystemTime {
        let value: u64 = self.into();
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

#[derive(Debug, Clone, Copy)]
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

impl Into<u32> for SimpleShortNtp {
    fn into(self) -> u32 {
        ((self.seconds as u32) << 16) | (self.fraction as u32)
    }
}

impl Into<SimpleNtp> for SimpleShortNtp {
    fn into(self) -> SimpleNtp {
        SimpleNtp {
            seconds: self.seconds as u32,
            fraction: self.fraction as u32,
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

impl Into<SystemTime> for SimpleShortNtp {
    fn into(self) -> SystemTime {
        let ntp: SimpleNtp = self.into();
        ntp.into()
    }
}
