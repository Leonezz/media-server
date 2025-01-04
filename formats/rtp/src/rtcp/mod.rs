use std::io::{self, Cursor, Read};

use app::RtcpAppPacket;
use bye::RtcpByePacket;
use common_header::RtcpCommonHeader;
use packet_traits::{
    reader::{ReadRemainingFrom, TryReadRemainingFrom},
    writer::WriteTo,
};
use payload_types::RtcpPayloadType;
use report::{RtcpReceiverReport, RtcpSenderReport};
use sdes::RtcpSourceDescriptionPacket;
use tokio_util::bytes::Buf;

use crate::errors::RtpError;

pub mod app;
pub mod bye;
pub mod common_header;
pub mod payload_types;
pub mod report;
pub mod sdes;
pub mod simple_ntp;

#[derive(Debug)]
pub enum RtcpPacket {
    SenderReport(RtcpSenderReport),
    ReceiverReport(RtcpReceiverReport),
    SourceDescription(RtcpSourceDescriptionPacket),
    Bye(RtcpByePacket),
    App(RtcpAppPacket),
}

impl<R: AsRef<[u8]>> TryReadRemainingFrom<RtcpCommonHeader, R> for RtcpPacket {
    type Error = RtpError;
    fn try_read_remaining_from(
        header: RtcpCommonHeader,
        reader: &mut std::io::Cursor<R>,
    ) -> Result<Option<Self>, Self::Error> {
        let bytes_remaining = (header.length as usize) * 4;
        if reader.remaining() < bytes_remaining {
            return Ok(None);
        }

        let mut remaining_bytes = Vec::new();
        remaining_bytes.resize(bytes_remaining, 0 as u8);
        reader.read_exact(&mut remaining_bytes)?;

        // ignore padding bytes
        if header.padding && !remaining_bytes.is_empty() {
            let padding_bytes = remaining_bytes.last().unwrap().clone();
            remaining_bytes.truncate(padding_bytes as usize);
        }

        let cursor = Cursor::new(&remaining_bytes);

        match header.payload_type {
            RtcpPayloadType::SenderReport => Ok(Some(Self::SenderReport(
                // there must be enough bytes
                RtcpSenderReport::read_remaining_from(header, cursor)?,
            ))),
            RtcpPayloadType::ReceiverReport => Ok(Some(Self::ReceiverReport(
                RtcpReceiverReport::read_remaining_from(header, cursor)?,
            ))),
            RtcpPayloadType::SourceDescription => Ok(Some(Self::SourceDescription(
                RtcpSourceDescriptionPacket::read_remaining_from(header, cursor)?,
            ))),
            RtcpPayloadType::Bye => Ok(Some(Self::Bye(RtcpByePacket::read_remaining_from(
                header, cursor,
            )?))),
            RtcpPayloadType::App => Ok(Some(Self::App(RtcpAppPacket::read_remaining_from(
                header, cursor,
            )?))),
        }
    }
}

impl<W: io::Write> WriteTo<W> for RtcpPacket {
    type Error = RtpError;
    fn write_to(&self, writer: W) -> Result<(), Self::Error> {
        match self {
            RtcpPacket::SenderReport(packet) => packet.write_to(writer),
            RtcpPacket::ReceiverReport(packet) => packet.write_to(writer),
            RtcpPacket::SourceDescription(packet) => packet.write_to(writer),
            RtcpPacket::Bye(packet) => packet.write_to(writer),
            RtcpPacket::App(packet) => packet.write_to(writer),
        }
    }
}
