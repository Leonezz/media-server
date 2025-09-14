use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Cursor};
use tokio_util::bytes::{Buf, Bytes};
use utils::{
    system::time::get_timestamp_ms,
    traits::{
        dynamic_sized_packet::DynamicSizedPacket,
        reader::{ReadFrom, TryReadFrom},
        writer::WriteTo,
    },
};

use crate::errors::{RtpError, RtpResult};

// @see: RFC 3550 5.1 RTP Fixed Header Fields
/// this is not likely to useful
///
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |V=2|P|X|  CC   |M|      PT     |        sequence number        |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                            timestamp                          |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |            synchronization source (SSRC) identifier           |
/// +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// |             contributing source (CSRC) identifiers            |
/// |                               ....                            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug, Clone)]
pub struct RtpHeader {
    pub version: u8,
    pub padding: bool,
    pub extension: bool,
    pub csrc_count: u8,
    pub marker: bool,
    pub payload_type: u8,
    pub sequence_number: u16,
    pub timestamp: u32,
    pub ssrc: u32,
    pub csrc_list: Vec<u32>,
    pub header_extension: Option<RtpHeaderExtension>,
}

impl Default for RtpHeader {
    fn default() -> Self {
        Self {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: false,
            payload_type: 0,
            sequence_number: 0,
            timestamp: 0,
            ssrc: 0,
            csrc_list: Vec::new(),
            header_extension: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RtpHeaderExtension {
    profile_defined: u16,
    length: u16,
    bytes: Bytes,
}

impl DynamicSizedPacket for RtpHeaderExtension {
    fn get_packet_bytes_count(&self) -> usize {
        2 // profile defined
          + 2 // length
          + self.bytes.len() // payload
    }
}

impl DynamicSizedPacket for RtpHeader {
    fn get_packet_bytes_count(&self) -> usize {
        4 // first line header
          + 4 // timestamp
          + 4 // ssrc
          + self.csrc_list.len() * 4 // csrc
          + if let Some(ex) = &self.header_extension {
            ex.get_packet_bytes_count()
          } else {
            0
          }
    }
}

impl<R: io::Read> ReadFrom<R> for RtpHeader {
    type Error = RtpError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let first_byte = reader.read_u8()?;
        let version = (first_byte >> 6) & 0b11;
        let padding = ((first_byte >> 5) & 0b1) == 0b1;
        let extension = ((first_byte >> 4) & 0b1) == 0b1;
        let csrc_count = first_byte & 0b1111;

        let second_byte = reader.read_u8()?;
        let marker = ((second_byte >> 7) & 0b1) == 0b1;
        let payload_type = first_byte & 0b0111_1111;

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
                Some(RtpHeaderExtension::read_from(reader)?)
            },
        })
    }
}

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
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let profile_defined = reader.read_u16::<BigEndian>()?;
        let length = reader.read_u16::<BigEndian>()?;
        let mut bytes = vec![0; length as usize];
        reader.read_exact(&mut bytes)?;

        Ok(Self {
            profile_defined,
            length,
            bytes: Bytes::from(bytes),
        })
    }
}

impl<W: io::Write> WriteTo<W> for RtpHeader {
    type Error = RtpError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let first_byte = ((self.version & 0b11) << 6)
            | ((self.padding as u8) << 5)
            | ((self.extension as u8) << 4)
            | (self.csrc_count & 0b1111);
        writer.write_u8(first_byte)?;
        writer.write_u8(((self.marker as u8) << 7) | (self.payload_type & 0b0111_1111))?;
        writer.write_u16::<BigEndian>(self.sequence_number)?;
        writer.write_u32::<BigEndian>(self.timestamp)?;
        writer.write_u32::<BigEndian>(self.ssrc)?;
        for csrc in &self.csrc_list {
            writer.write_u32::<BigEndian>(*csrc)?;
        }

        if let Some(header_extension) = &self.header_extension {
            header_extension.write_to(writer)?;
        }

        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for RtpHeaderExtension {
    type Error = RtpError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u16::<BigEndian>(self.profile_defined)?;
        writer.write_u16::<BigEndian>(self.length)?;
        writer.write_all(&self.bytes)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct RtpHeaderBuilder {
    pub header: RtpHeader,
}

impl RtpHeaderBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn version(&mut self, version: u8) -> &mut Self {
        self.header.version = version;
        self
    }

    pub fn csrc(&mut self, csrc: u32) -> RtpResult<&mut Self> {
        if self.header.csrc_list.len() > 30 {
            return Err(RtpError::TooManyCSRC);
        }
        self.header.csrc_list.push(csrc);
        self.header.csrc_count = self.header.csrc_list.len() as u8;
        Ok(self)
    }

    pub fn marker(&mut self, marker: bool) -> &mut Self {
        self.header.marker = marker;
        self
    }

    pub fn payload_type(&mut self, payload_type: u8) -> &mut Self {
        self.header.payload_type = payload_type;
        self
    }

    pub fn sequence_number(&mut self, number: u16) -> &mut Self {
        self.header.sequence_number = number;
        self
    }

    pub fn timestamp(&mut self, timestamp: u32) -> &mut Self {
        self.header.timestamp = timestamp;
        self
    }

    pub fn timestamp_now(&mut self) -> &mut Self {
        self.timestamp(get_timestamp_ms().unwrap_or(0) as u32)
    }

    pub fn ssrc(&mut self, ssrc: u32) -> &mut Self {
        self.header.ssrc = ssrc;
        self
    }

    pub fn extension(&mut self, extension: RtpHeaderExtension) -> &mut Self {
        self.header.header_extension = Some(extension);
        self
    }

    pub fn build(&self) -> RtpHeader {
        self.header.clone()
    }
}
