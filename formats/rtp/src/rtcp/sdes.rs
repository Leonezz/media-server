use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use packet_traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    fixed_packet::FixedPacket,
    reader::{ReadFrom, ReadRemainingFrom},
    writer::WriteTo,
};
use std::io::{self};

use crate::errors::RtpError;

use super::{common_header::RtcpCommonHeader, payload_types::RtcpPayloadType};

///! @see: RFC 3550 6.5 SDES: Source Description RTCP Packet
///         0                   1                   2                   3
///         0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// header |V=2|P|    SC   |  PT=SDES=202  |             length            |
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// chunk  |                             SSRC/CSRC_1                       |
///     1  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                             SDES items                        |
///        |                                ...                            |
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// chunk  |                             SSRC/CSRC_2                       |
///     2  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                             SDES items                        |
///        |                                ...                            |
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
///

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SDESItemType {
    CNAME = 1,
    NAME = 2,
    EMAIL = 3,
    PHONE = 4,
    LOC = 5,
    TOOL = 6,
    NOTE = 7,
    PRIV = 8,
}

impl Into<u8> for SDESItemType {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for SDESItemType {
    type Error = RtpError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::CNAME),
            2 => Ok(Self::NAME),
            3 => Ok(Self::EMAIL),
            4 => Ok(Self::PHONE),
            5 => Ok(Self::LOC),
            6 => Ok(Self::TOOL),
            7 => Ok(Self::NOTE),
            8 => Ok(Self::PRIV),
            _ => Err(RtpError::UnknownSdesType(value)),
        }
    }
}

#[derive(Debug)]
pub struct SDESBody {
    length: u8,
    value: String,
}

impl DynamicSizedPacket for SDESBody {
    fn get_packet_bytes_count(&self) -> usize {
        1 + self.value.len()
    }
}

impl<R: io::Read> ReadFrom<R> for SDESBody {
    type Error = RtpError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let length = reader.read_u8()?;
        let mut str_bytes = vec![0 as u8; length as usize];
        reader.read_exact(&mut str_bytes)?;
        Ok(Self {
            length,
            value: String::from_utf8(str_bytes)?,
        })
    }
}

impl<W: io::Write> WriteTo<W> for SDESBody {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_u8(self.length)?;
        writer.write_all(self.value.as_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SDESItem {
    item_type: SDESItemType,
    item_body: SDESBody,
}

impl DynamicSizedPacket for SDESItem {
    fn get_packet_bytes_count(&self) -> usize {
        1 + self.item_body.get_packet_bytes_count()
    }
}

impl<R: io::Read> ReadFrom<R> for SDESItem {
    type Error = RtpError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let item_type: SDESItemType = reader.read_u8()?.try_into()?;
        if item_type == SDESItemType::PRIV {
            todo!()
        }

        let item_body = SDESBody::read_from(reader.by_ref())?;
        Ok(Self {
            item_type,
            item_body,
        })
    }
}

impl<W: io::Write> WriteTo<W> for SDESItem {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_u8(self.item_type.into())?;
        self.item_body.write_to(writer.by_ref())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SDESChunk {
    ssrc: u32,
    items: Vec<SDESItem>,
}

impl DynamicSizedPacket for SDESChunk {
    fn get_packet_bytes_count(&self) -> usize {
        4 + self
            .items
            .iter()
            .fold(0, |sum, v| v.get_packet_bytes_count() + sum)
    }
}

impl<R: io::Read> ReadFrom<R> for SDESChunk {
    type Error = RtpError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let ssrc = reader.read_u32::<BigEndian>()?;
        let bytes_read = 0;
        let mut items = Vec::new();
        loop {
            let item_type = reader.read_u8()?;
            if item_type != 0 {
                items.push(SDESItem::read_from(reader.by_ref())?);
            } else {
                if bytes_read % 4 == 0 {
                    break;
                }
                let bytes_to_align_word = 4 - (bytes_read % 4);
                for _ in 0..bytes_to_align_word {
                    // skip padding bytes
                    let _ = reader.read_u8()?;
                }
            }
        }

        Ok(Self { ssrc, items })
    }
}

impl<W: io::Write> WriteTo<W> for SDESChunk {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_u32::<BigEndian>(self.ssrc)?;
        self.items
            .iter()
            .try_for_each(|item| item.write_to(writer.by_ref()))?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct RtcpSourceDescriptionPacket {
    header: RtcpCommonHeader,
    chunks: Vec<SDESChunk>,
}

impl DynamicSizedPacket for RtcpSourceDescriptionPacket {
    fn get_packet_bytes_count(&self) -> usize {
        RtcpCommonHeader::bytes_count()
            + self
                .chunks
                .iter()
                .fold(0, |sum, v| v.get_packet_bytes_count() + sum)
    }
}

impl<R: io::Read> ReadRemainingFrom<RtcpCommonHeader, R> for RtcpSourceDescriptionPacket {
    type Error = RtpError;
    fn read_remaining_from(header: RtcpCommonHeader, mut reader: R) -> Result<Self, Self::Error> {
        if header.payload_type != RtcpPayloadType::SourceDescription {
            return Err(RtpError::WrongPayloadType(format!(
                "expect sdes payload type got {:?} instead",
                header.payload_type
            )));
        }

        let mut chunks = Vec::with_capacity(header.count as usize);
        for _ in 0..header.count {
            chunks.push(SDESChunk::read_from(reader.by_ref())?);
        }

        Ok(Self { header, chunks })
    }
}

impl<W: io::Write> WriteTo<W> for RtcpSourceDescriptionPacket {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        self.header.write_to(writer.by_ref())?;
        self.chunks
            .iter()
            .try_for_each(|chunk| chunk.write_to(writer.by_ref()))?;

        Ok(())
    }
}
