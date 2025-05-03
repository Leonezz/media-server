pub mod builder;
pub mod framed;
pub mod sequencer;
use std::io::{self, Read};

use builder::RtpTrivialPacketBuilder;

use tokio_util::bytes::{Buf, Bytes};
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket, reader::TryReadFrom, writer::WriteTo,
};

use crate::{
    errors::RtpError,
    header::RtpHeader,
    util::{
        RtpPacketTrait, RtpPaddedPacketTrait,
        padding::{rtp_get_padding_size, rtp_make_padding_bytes, rtp_need_padding},
    },
};

#[derive(Debug, Clone)]
pub struct RtpTrivialPacket {
    pub header: RtpHeader,
    pub payload: Bytes,
}

impl RtpTrivialPacket {
    pub fn builder() -> RtpTrivialPacketBuilder {
        Default::default()
    }

    pub fn new(header: RtpHeader, payload: Bytes) -> Self {
        let mut result = Self { header, payload };
        let raw_size = result.get_packet_bytes_count_without_padding();
        result.header.padding = rtp_need_padding(raw_size);
        result
    }
}

impl DynamicSizedPacket for RtpTrivialPacket {
    fn get_packet_bytes_count(&self) -> usize {
        let raw_size = self.get_packet_bytes_count_without_padding();
        raw_size + rtp_get_padding_size(raw_size)
    }
}

impl RtpPaddedPacketTrait for RtpTrivialPacket {
    fn get_packet_bytes_count_without_padding(&self) -> usize {
        self.header.get_packet_bytes_count() + self.payload.len()
    }
}

impl RtpPacketTrait for RtpTrivialPacket {
    fn get_header(&self) -> RtpHeader {
        let raw_size = self.get_packet_bytes_count_without_padding();
        RtpHeader {
            version: 2,
            padding: rtp_need_padding(raw_size),
            extension: self.header.extension,
            csrc_count: self.header.csrc_list.len() as u8,
            marker: self.header.marker,
            payload_type: self.header.payload_type,
            sequence_number: self.header.sequence_number,
            timestamp: self.header.timestamp,
            ssrc: self.header.ssrc,
            csrc_list: self.header.csrc_list.clone(),
            header_extension: self.header.header_extension.clone(),
        }
    }
}

impl<R: AsRef<[u8]>> TryReadFrom<R> for RtpTrivialPacket {
    type Error = RtpError;
    fn try_read_from(reader: &mut std::io::Cursor<R>) -> Result<Option<Self>, Self::Error> {
        let header = RtpHeader::try_read_from(reader.by_ref())?;
        if header.is_none() {
            return Ok(None);
        }

        if !reader.has_remaining() {
            return Err(RtpError::EmptyPayload);
        }
        let payload_size = reader.remaining();
        let payload = reader.copy_to_bytes(payload_size);

        let header = header.unwrap();
        if header.padding {
            let padding_size = *payload.last().unwrap() as usize;
            if padding_size > payload_size {
                return Err(RtpError::BadPaddingSize(padding_size));
            }

            Ok(Some(Self {
                header,
                payload: payload.slice(..payload_size - padding_size),
            }))
        } else {
            Ok(Some(Self { header, payload }))
        }
    }
}

impl<W: io::Write> WriteTo<W> for RtpTrivialPacket {
    type Error = RtpError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let raw_size = self.get_packet_bytes_count_without_padding();
        self.get_header().write_to(writer.by_ref())?;
        writer.write_all(&self.payload)?;
        if let Some(padding) = rtp_make_padding_bytes(raw_size) {
            writer.write_all(&padding)?;
        }
        Ok(())
    }
}
