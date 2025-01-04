use byteorder::{BigEndian, ReadBytesExt};
use packet_traits::reader::{ReadFrom, TryReadFrom};
use std::io::{self, Cursor};
use tokio_util::bytes::{Buf, BytesMut};

use crate::errors::{RtpError, RtpResult};

use super::{RtpHeader, RtpHeaderExtension};

impl<R: AsRef<[u8]>> TryReadFrom<R> for RtpHeader {
    type Error = RtpError;
    fn try_read_from(reader: &mut Cursor<R>) -> Result<Option<Self>, Self::Error> {
        if reader.remaining() < 12 {
            return Ok(None);
        }
        let first_byte = reader.read_u8()?;
        let version = (first_byte >> 6) & 0b11;
        let padding = ((first_byte >> 5) & 0b1) == 0b1;
        let extension = ((first_byte >> 4) & 0b1) == 0b1;
        let csrc_count = first_byte & 0b1111;

        let second_byte = reader.read_u8()?;
        let marker = ((second_byte >> 7) & 0b1) == 0b1;
        let payload_type = second_byte & 0b0111_1111;

        let sequence_number = reader.read_u16::<BigEndian>()?;
        let timestamp = reader.read_u32::<BigEndian>()?;
        let ssrc = reader.read_u32::<BigEndian>()?;

        if reader.remaining() < (csrc_count * 4) as usize {
            return Ok(None);
        }

        let mut csrc_list = Vec::with_capacity(csrc_count as usize);
        for _ in 0..csrc_count {
            csrc_list.push(reader.read_u32::<BigEndian>()?);
        }

        Ok(Some(Self {
            version,
            padding,
            extension,
            csrc_count,
            marker,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
            csrc_list,
            header_extension: if !extension {
                None
            } else {
                Some(RtpHeaderExtension::read_from(reader)?)
            },
        }))
    }
}

impl<R: io::Read> ReadFrom<R> for RtpHeaderExtension {
    type Error = RtpError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let profile_defined = reader.read_u16::<BigEndian>()?;
        let length = reader.read_u16::<BigEndian>()?;
        let mut bytes = Vec::with_capacity(length as usize);
        bytes.resize(length as usize, 0);
        reader.read_exact(&mut bytes)?;

        Ok(Self {
            profile_defined,
            length,
            bytes: BytesMut::from(&bytes[..]),
        })
    }
}
