use byteorder::{BigEndian, ReadBytesExt};
use std::io;
use tokio_util::bytes::BytesMut;

use crate::errors::RtpResult;

use super::{RtpHeader, RtpHeaderExtension};

impl RtpHeader {
    pub fn read_from<R>(mut reader: R) -> RtpResult<Self>
    where
        R: io::Read,
    {
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

        let mut csrc_list = Vec::with_capacity(csrc_count as usize);
        for _ in 0..csrc_count {
            csrc_list.push(reader.read_u32::<BigEndian>()?);
        }

        Ok(Self {
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
                Some(RtpHeaderExtension::read_from(reader.by_ref())?)
            },
        })
    }
}

impl RtpHeaderExtension {
    pub fn read_from<R>(mut reader: R) -> RtpResult<Self>
    where
        R: io::Read,
    {
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
