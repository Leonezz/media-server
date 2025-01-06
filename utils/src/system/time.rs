use std::time::{self, SystemTimeError, UNIX_EPOCH};


pub fn get_timestamp_ms() -> Result<u64, SystemTimeError> {
    Ok(time::SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis() as u64)
}

pub fn get_timestamp_ns() -> Result<u128, SystemTimeError> {
    Ok(time::SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos())
}
