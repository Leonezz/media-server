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
