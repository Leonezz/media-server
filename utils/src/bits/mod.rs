#[inline]
pub const fn bool_from_bit(value: u8) -> bool {
    return (value & 0b1) == 0b1;
}

#[inline]
pub const fn bool_to_bit(value: bool) -> u8 {
    match value {
        true => 0b1,
        false => 0b0,
    }
}
