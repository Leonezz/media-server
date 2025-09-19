use std::io;

use crate::errors::FLVError;
use byteorder::{BigEndian, ReadBytesExt};
use tokio_util::bytes::Buf;
use utils::traits::{
    fixed_packet::FixedPacket,
    reader::{ReadFrom, TryReadFrom},
};

use super::{FLVTagHeader, FLVTagType};

impl<R: io::Read> ReadFrom<R> for FLVTagHeader {
    type Error = FLVError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let first_byte = reader.read_u8()?;
        let filter_enabled = ((first_byte >> 5) & 0b1) != 0;
        let tag_type: FLVTagType = (first_byte & 0b11111).try_into()?;
        let data_size = reader.read_u24::<BigEndian>()?;
        let timestamp = reader.read_u24::<BigEndian>()?;
        let timestamp_extended = reader.read_u8()?;
        let timestamp = (((timestamp_extended as u32) << 24) | timestamp) & 0x7FFF_FFFF;
        let _stream_id = reader.read_u24::<BigEndian>()?;

        Ok(FLVTagHeader {
            tag_type,
            data_size,
            timestamp,
            filter_enabled,
        })
    }
}

impl<R: AsRef<[u8]>> TryReadFrom<R> for FLVTagHeader {
    type Error = FLVError;
    fn try_read_from(reader: &mut io::Cursor<R>) -> Result<Option<Self>, Self::Error> {
        if reader.remaining() < FLVTagHeader::bytes_count() {
            return Ok(None);
        }
        Ok(Some(Self::read_from(reader)?))
    }
}
