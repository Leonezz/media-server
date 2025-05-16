use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use num::ToPrimitive;
use std::io::{self};
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    fixed_packet::FixedPacket,
    reader::{ReadFrom, ReadRemainingFrom},
    writer::WriteTo,
};

use crate::{
    errors::{RtpError, RtpResult},
    util::{
        RtpPaddedPacketTrait,
        padding::{rtp_get_padding_size, rtp_make_padding_bytes, rtp_need_padding},
    },
};

use super::{RtcpPacketSizeTrait, common_header::RtcpCommonHeader, payload_types::RtcpPayloadType};

// @see: RFC 3550 6.5 SDES: Source Description RTCP Packet
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

impl From<SDESItemType> for u8 {
    fn from(value: SDESItemType) -> Self {
        value as u8
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

#[derive(Debug, Clone)]
pub struct SDESBody {
    length: u8,
    value: String,
}

impl TryFrom<String> for SDESBody {
    type Error = RtpError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > 255 {
            return Err(RtpError::SDESValueTooLarge(value));
        }
        Ok(Self {
            length: value.len().to_u8().unwrap(),
            value,
        })
    }
}

impl DynamicSizedPacket for SDESBody {
    fn get_packet_bytes_count(&self) -> usize {
        1 + self.value.len()
    }
}

impl<R: io::Read> ReadFrom<R> for SDESBody {
    type Error = RtpError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let length = reader.read_u8()?;
        let mut str_bytes = vec![0_u8; length as usize];
        reader.read_exact(&mut str_bytes)?;
        Ok(Self {
            length,
            value: String::from_utf8(str_bytes)?,
        })
    }
}

impl<W: io::Write> WriteTo<W> for SDESBody {
    type Error = RtpError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.length)?;
        writer.write_all(self.value.as_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
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
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
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
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.item_type.into())?;
        self.item_body.write_to(writer)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SDESChunk {
    pub ssrc: u32,
    pub items: Vec<SDESItem>,
}

impl DynamicSizedPacket for SDESChunk {
    fn get_packet_bytes_count(&self) -> usize {
        let len = self.get_packet_bytes_count_without_padding();
        if len % 4 == 0 {
            len + 4
        } else {
            rtp_get_padding_size(len) + len
        }
    }
}

impl RtpPaddedPacketTrait for SDESChunk {
    fn get_packet_bytes_count_without_padding(&self) -> usize {
        4 + self
            .items
            .iter()
            .fold(0, |sum, v| v.get_packet_bytes_count() + sum)
    }
}

impl<R: io::Read> ReadFrom<R> for SDESChunk {
    type Error = RtpError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let ssrc = reader.read_u32::<BigEndian>()?;
        let bytes_read = 0;
        let mut items = Vec::new();
        loop {
            let item_type = reader.read_u8()?;
            if item_type != 0 {
                items.push(SDESItem::read_from(reader)?);
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
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u32::<BigEndian>(self.ssrc)?;
        self.items
            .iter()
            .try_for_each(|item| item.write_to(writer))?;
        let raw_len = self.get_packet_bytes_count_without_padding();
        let padding_size = rtp_get_padding_size(raw_len);
        if padding_size == 0 {
            writer.write_u32::<BigEndian>(0)?;
        } else {
            writer.write_all(&vec![0_u8; padding_size])?;
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct RtcpSourceDescriptionPacket {
    pub header: RtcpCommonHeader,
    pub chunks: Vec<SDESChunk>,
}

impl DynamicSizedPacket for RtcpSourceDescriptionPacket {
    fn get_packet_bytes_count(&self) -> usize {
        let raw_size = self.get_packet_bytes_count_without_padding();
        raw_size + rtp_get_padding_size(raw_size)
    }
}

impl RtcpPacketSizeTrait for RtcpSourceDescriptionPacket {
    fn get_packet_bytes_count_without_padding(&self) -> usize {
        RtcpCommonHeader::bytes_count()
            + self
                .chunks
                .iter()
                .fold(0, |sum, v| v.get_packet_bytes_count() + sum)
    }
    fn get_header(&self) -> RtcpCommonHeader {
        let raw_size = self.get_packet_bytes_count_without_padding();
        RtcpCommonHeader {
            version: 2,
            padding: rtp_need_padding(raw_size),
            count: self.chunks.len() as u8,
            payload_type: RtcpPayloadType::SourceDescription,
            length: (self.get_packet_bytes_count() / 4 - 1) as u16,
        }
    }
}

impl<R: io::Read> ReadRemainingFrom<RtcpCommonHeader, R> for RtcpSourceDescriptionPacket {
    type Error = RtpError;
    fn read_remaining_from(header: RtcpCommonHeader, reader: &mut R) -> Result<Self, Self::Error> {
        if header.payload_type != RtcpPayloadType::SourceDescription {
            return Err(RtpError::WrongPayloadType(format!(
                "expect sdes payload type got {:?} instead",
                header.payload_type
            )));
        }

        let mut chunks = Vec::with_capacity(header.count as usize);
        for _ in 0..header.count {
            chunks.push(SDESChunk::read_from(reader)?);
        }

        Ok(Self { header, chunks })
    }
}

impl<W: io::Write> WriteTo<W> for RtcpSourceDescriptionPacket {
    type Error = RtpError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let raw_size = self.get_packet_bytes_count_without_padding();
        self.get_header().write_to(writer)?;
        self.chunks
            .iter()
            .try_for_each(|chunk| chunk.write_to(writer))?;

        if let Some(padding) = rtp_make_padding_bytes(raw_size) {
            writer.write_all(&padding)?;
        }
        Ok(())
    }
}

impl RtcpSourceDescriptionPacket {
    pub fn builder() -> RtcpSourceDescriptionPacketBuilder {
        RtcpSourceDescriptionPacketBuilder::new()
    }
    pub fn get_cname(&self) -> Option<String> {
        self.chunks.iter().find_map(|v| {
            v.items.iter().find_map(|item| {
                if !matches!(item.item_type, SDESItemType::CNAME) {
                    return None;
                }
                Some(item.item_body.value.clone())
            })
        })
    }
    pub fn get_cname_of(&self, ssrc: u32) -> Option<String> {
        self.chunks.iter().find_map(|v| {
            if v.ssrc != ssrc {
                return None;
            }
            v.items.iter().find_map(|item| {
                if !matches!(item.item_type, SDESItemType::CNAME) {
                    return None;
                }

                Some(item.item_body.value.clone())
            })
        })
    }
}

#[derive(Debug, Default)]
pub struct RtcpSourceDescriptionPacketBuilder(RtcpSourceDescriptionPacket);

impl RtcpSourceDescriptionPacketBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn build(mut self) -> RtpResult<RtcpSourceDescriptionPacket> {
        if self.0.chunks.len() > 31 {
            return Err(RtpError::SDESTooManyChunks);
        }
        self.0.header = self.0.get_header();
        Ok(self.0)
    }

    pub fn item(mut self, ssrc: u32, item: SDESItem) -> Self {
        if let Some(chunk) = self.0.chunks.iter_mut().find(|v| v.ssrc == ssrc) {
            chunk.items.push(item);
        } else {
            self.0.chunks.push(SDESChunk {
                ssrc,
                items: vec![item],
            });
        }
        self
    }

    fn item_from_parts(self, ssrc: u32, item_type: SDESItemType, value: String) -> RtpResult<Self> {
        let item_body = SDESBody::try_from(value);
        item_body.map(|v| {
            self.item(
                ssrc,
                SDESItem {
                    item_type,
                    item_body: v,
                },
            )
        })
    }

    pub fn cname(self, ssrc: u32, cname: String) -> RtpResult<Self> {
        self.item_from_parts(ssrc, SDESItemType::CNAME, cname)
    }

    pub fn name(self, ssrc: u32, name: String) -> RtpResult<Self> {
        self.item_from_parts(ssrc, SDESItemType::NAME, name)
    }

    pub fn email(self, ssrc: u32, email: String) -> RtpResult<Self> {
        self.item_from_parts(ssrc, SDESItemType::EMAIL, email)
    }

    pub fn phone(self, ssrc: u32, phone: String) -> RtpResult<Self> {
        self.item_from_parts(ssrc, SDESItemType::PHONE, phone)
    }

    pub fn loc(self, ssrc: u32, loc: String) -> RtpResult<Self> {
        self.item_from_parts(ssrc, SDESItemType::LOC, loc)
    }

    pub fn tool(self, ssrc: u32, tool: String) -> RtpResult<Self> {
        self.item_from_parts(ssrc, SDESItemType::TOOL, tool)
    }

    pub fn note(self, ssrc: u32, note: String) -> RtpResult<Self> {
        self.item_from_parts(ssrc, SDESItemType::TOOL, note)
    }

    pub fn private(self, ssrc: u32, value: String) -> RtpResult<Self> {
        self.item_from_parts(ssrc, SDESItemType::PRIV, value)
    }
}
