use crate::traits::writer::WriteTo;
use std::fmt::Write;

pub fn hex_to_bytes(s: &str) -> Option<Vec<u8>> {
    if s.len().is_multiple_of(2) {
        (0..s.len())
            .step_by(2)
            .map(|i| {
                s.get(i..i + 2)
                    .and_then(|sub| u8::from_str_radix(sub, 16).ok())
            })
            .collect()
    } else {
        None
    }
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

pub fn writable_to_bytes<E, T: WriteTo<Vec<u8>, Error = E>>(writable: &T) -> Result<Vec<u8>, E> {
    let mut bytes = vec![];
    writable.write_to(&mut bytes)?;
    Ok(bytes)
}
