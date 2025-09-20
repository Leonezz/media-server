use std::io;

use bitstream_io::BitRead;
use codec_bitstream::reader::BitstreamReader;

use crate::errors::H264CodecError;

pub trait RbspReadExt {
    type Error;
    fn more_rbsp_data(&mut self) -> Result<bool, Self::Error>;
}

impl<'a> RbspReadExt for BitstreamReader<'a> {
    type Error = H264CodecError;
    fn more_rbsp_data(&mut self) -> Result<bool, Self::Error> {
        let remaining = self.remaining_bits();
        if remaining > 8 {
            return Ok(true);
        }
        if remaining == 0 {
            return Ok(false);
        }
        let mut temp_reader = self.reader().clone();
        temp_reader.skip(1)?;
        match temp_reader.read_unary::<1>() {
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
            Err(e) => Err(e.into()),
            Ok(_) => Ok(true),
        }
    }
}

/// for every 0 0 in rbsp, it should be transformed to 0 0 3
pub fn count_rbsp_bytes(value: &[u8]) -> usize {
    if value.len() < 3 {
        return value.len();
    }
    let mut extra: usize = 0;
    let mut i = 2;
    while i < value.len() {
        if value[i - 2..i] == [0, 0] && value[i] < 4 {
            extra += 1;
            i += 2;
        } else {
            i += 1;
        }
    }
    extra.checked_add(value.len()).unwrap()
}

pub fn rbsp_to_sodb(value: &[u8]) -> Vec<u8> {
    if value.len() < 3 {
        return Vec::from(value);
    }
    let mut result = Vec::with_capacity(value.len());
    result.extend_from_slice(&value[0..2]);
    for v in &value[2..] {
        let result_len = result.len();
        if *v < 4 && result[result_len - 2..result_len] == [0, 0] {
            result.push(0x03);
        }
        result.push(*v);
    }
    result
}

pub fn need_escape(value: &[u8]) -> bool {
    value.windows(3).any(|v| v[2] < 4 && v[0..2] == [0, 0])
}

pub fn rbsp_extract(sodb: &[u8]) -> Vec<u8> {
    if sodb.len() < 3 {
        return Vec::from(sodb);
    }
    let mut result = Vec::with_capacity(sodb.len());
    result.extend_from_slice(&sodb[0..2]);
    for i in 2..sodb.len() {
        if sodb[i - 2..=i] == [0, 0, 3] {
            continue;
        }
        result.push(sodb[i]);
    }
    result
}

pub fn need_extract(rbsp: &[u8]) -> bool {
    rbsp.windows(3).any(|v| v == [0, 0, 3])
}
