use std::io;

use bitstream_io::BitRead;
use codec_bitstream::reader::BitstreamReader;
use tokio_util::bytes::BytesMut;

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

/// for every 0 0 1 in rbsp, it should be transformed to 0 0 3 1
pub fn count_rbsp_bytes(value: &[u8]) -> usize {
    if value.len() < 3 {
        return value.len();
    }
    let mut extra = 0;
    for i in 2..value.len() {
        if value[i - 2..=i] == [0, 0, 1] {
            extra += 1;
        }
    }
    extra + value.len() + 1 // 1 for trailing bits
}

pub fn raw_bytes_to_rbsp(value: &[u8]) -> Vec<u8> {
    if value.len() < 3 {
        return Vec::from(value);
    }
    let mut result = Vec::with_capacity(value.len());
    result.extend_from_slice(&value[0..2]);
    for i in 2..value.len() {
        if value[i - 2..=i] == [0, 0, 1] {
            result.push(0x03);
        }
        result.push(value[i]);
    }
    result
}
