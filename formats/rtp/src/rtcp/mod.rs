use std::io::{self, Cursor, Read};

use app::RtcpAppPacket;
use bye::RtcpByePacket;
use common_header::RtcpCommonHeader;
use payload_types::RtcpPayloadType;
use receiver_report::RtcpReceiverReport;
use report_block::ReportBlock;
use sdes::RtcpSourceDescriptionPacket;
use sender_report::{RtcpSenderReport, SenderInfo};
use tokio_util::bytes::Buf;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    reader::{ReadRemainingFrom, TryReadRemainingFrom},
    writer::WriteTo,
};

use crate::{errors::RtpError, util::padding::rtp_get_padding_size};

pub mod app;
pub mod bye;
pub mod common_header;
pub mod compound_packet;
pub mod framed;
pub mod payload_types;
pub mod receiver_report;
pub mod report_block;
pub mod sdes;
pub mod sender_report;
pub mod simple_ntp;

pub trait RtcpPacketSizeTrait: DynamicSizedPacket {
    fn get_packet_bytes_count_without_padding(&self) -> usize;
    fn get_header(&self) -> RtcpCommonHeader;
}

pub trait RtcpPacketTrait {
    fn sender_ssrc(&self) -> Option<u32>;
    fn csrc_list(&self) -> Vec<u32>;
    fn payload_type(&self) -> RtcpPayloadType;
    fn sender_info(&self) -> Option<SenderInfo>;
    fn report_blocks(&self) -> Option<Vec<ReportBlock>>;
    fn sde_chunks(&self) -> Option<Vec<sdes::SDESChunk>>;
}

#[derive(Debug, Clone)]
pub enum RtcpPacket {
    SenderReport(RtcpSenderReport),
    ReceiverReport(RtcpReceiverReport),
    SourceDescription(RtcpSourceDescriptionPacket),
    Bye(RtcpByePacket),
    App(RtcpAppPacket),
}

impl RtcpPacketTrait for RtcpPacket {
    fn payload_type(&self) -> RtcpPayloadType {
        match self {
            RtcpPacket::SenderReport(_) => RtcpPayloadType::SenderReport,
            RtcpPacket::ReceiverReport(_) => RtcpPayloadType::ReceiverReport,
            RtcpPacket::SourceDescription(_) => RtcpPayloadType::SourceDescription,
            RtcpPacket::Bye(_) => RtcpPayloadType::Bye,
            RtcpPacket::App(_) => RtcpPayloadType::App,
        }
    }

    fn sender_ssrc(&self) -> Option<u32> {
        match self {
            RtcpPacket::ReceiverReport(packet) => Some(packet.sender_ssrc),
            RtcpPacket::SenderReport(packet) => Some(packet.sender_ssrc),
            RtcpPacket::SourceDescription(_) => None,
            RtcpPacket::Bye(_) => None,
            RtcpPacket::App(packet) => Some(packet.ssrc),
        }
    }

    fn csrc_list(&self) -> Vec<u32> {
        match self {
            RtcpPacket::ReceiverReport(packet) => {
                packet.report_blocks.iter().map(|item| item.ssrc).collect()
            }
            RtcpPacket::SenderReport(packet) => {
                packet.report_blocks.iter().map(|item| item.ssrc).collect()
            }
            RtcpPacket::SourceDescription(packet) => {
                packet.chunks.iter().map(|item| item.ssrc).collect()
            }
            RtcpPacket::Bye(packet) => packet.ssrc_list.clone(),
            RtcpPacket::App(_) => vec![],
        }
    }

    fn report_blocks(&self) -> Option<Vec<ReportBlock>> {
        match self {
            RtcpPacket::ReceiverReport(packet) => Some(packet.report_blocks.clone()),
            RtcpPacket::SenderReport(packet) => Some(packet.report_blocks.clone()),
            _ => None,
        }
    }

    fn sender_info(&self) -> Option<SenderInfo> {
        match self {
            RtcpPacket::SenderReport(packet) => Some(packet.sender_info.clone()),
            _ => None,
        }
    }

    fn sde_chunks(&self) -> Option<Vec<sdes::SDESChunk>> {
        match self {
            RtcpPacket::SourceDescription(packet) => Some(packet.chunks.clone()),
            _ => None,
        }
    }
}

impl RtcpPacketSizeTrait for RtcpPacket {
    fn get_packet_bytes_count_without_padding(&self) -> usize {
        match self {
            RtcpPacket::SenderReport(packet) => packet.get_packet_bytes_count_without_padding(),
            RtcpPacket::ReceiverReport(packet) => packet.get_packet_bytes_count_without_padding(),
            RtcpPacket::SourceDescription(packet) => {
                packet.get_packet_bytes_count_without_padding()
            }
            RtcpPacket::Bye(packet) => packet.get_packet_bytes_count_without_padding(),
            RtcpPacket::App(packet) => packet.get_packet_bytes_count_without_padding(),
        }
    }
    fn get_header(&self) -> RtcpCommonHeader {
        match self {
            RtcpPacket::SenderReport(packet) => packet.get_header(),
            RtcpPacket::ReceiverReport(packet) => packet.get_header(),
            RtcpPacket::SourceDescription(packet) => packet.get_header(),
            RtcpPacket::Bye(packet) => packet.get_header(),
            RtcpPacket::App(packet) => packet.get_header(),
        }
    }
}

impl DynamicSizedPacket for RtcpPacket {
    fn get_packet_bytes_count(&self) -> usize {
        let raw_size = self.get_packet_bytes_count_without_padding();
        raw_size + rtp_get_padding_size(raw_size)
    }
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

        let mut remaining_bytes = vec![0_u8; bytes_remaining];
        reader.read_exact(&mut remaining_bytes)?;

        // ignore padding bytes
        if header.padding && !remaining_bytes.is_empty() {
            let padding_bytes = *remaining_bytes.last().unwrap();
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
