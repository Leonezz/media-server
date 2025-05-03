use num::ToPrimitive;
use tokio_util::bytes::Buf;
use utils::traits::reader::{ReadFrom, ReadRemainingFrom, TryReadFrom};

use std::io::{self, Cursor, Read};

use crate::errors::FLVError;

use super::{FLVTag, FLVTagBodyWithFilter, FLVTagHeader};

impl<R: io::Read> ReadFrom<R> for FLVTag {
    type Error = FLVError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let tag_header = FLVTagHeader::read_from(reader.by_ref())?;

        let tag_body = FLVTagBodyWithFilter::read_remaining_from(&tag_header, reader)?;
        Ok(FLVTag {
            tag_header,
            body_with_filter: tag_body,
        })
    }
}

impl<R: AsRef<[u8]>> TryReadFrom<R> for FLVTag {
    type Error = FLVError;
    fn try_read_from(reader: &mut Cursor<R>) -> Result<Option<Self>, Self::Error> {
        let tag_header = FLVTagHeader::try_read_from(reader.by_ref())?;
        if tag_header.is_none() {
            return Ok(None);
        }
        let tag_header = tag_header.unwrap();
        if reader.remaining() < tag_header.data_size.to_usize().unwrap() {
            return Ok(None);
        }
        Ok(Some(Self {
            body_with_filter: FLVTagBodyWithFilter::read_remaining_from(&tag_header, reader)?,
            tag_header,
        }))
    }
}
