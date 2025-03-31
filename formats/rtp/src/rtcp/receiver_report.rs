use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io;
use tokio_util::bytes::Bytes;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    fixed_packet::FixedPacket,
    reader::{ReadFrom, ReadRemainingFrom},
    writer::WriteTo,
};

use crate::{
    errors::{RtpError, RtpResult},
    util::padding::{rtp_get_padding_size, rtp_make_padding_bytes, rtp_need_padding},
};

use super::{
    RtcpPacketSizeTrait, common_header::RtcpCommonHeader, payload_types::RtcpPayloadType,
    report_block::ReportBlock,
};

#[derive(Debug, Default, Clone)]
pub struct RtcpReceiverReport {
    pub header: RtcpCommonHeader,
    pub sender_ssrc: u32,
    pub report_blocks: Vec<ReportBlock>,
    pub profile_specific_extension: Option<Bytes>,
}

impl DynamicSizedPacket for RtcpReceiverReport {
    fn get_packet_bytes_count(&self) -> usize {
        let raw_size = self.get_packet_bytes_count_without_padding();
        raw_size + rtp_get_padding_size(raw_size)
    }
}

impl RtcpPacketSizeTrait for RtcpReceiverReport {
    fn get_packet_bytes_count_without_padding(&self) -> usize {
        RtcpCommonHeader::bytes_count() // header
            + 4 // sender ssrc
            + self.report_blocks.len() * ReportBlock::bytes_count() // report blocks
            + self.profile_specific_extension.as_ref().map_or_else(|| 0, |extension| extension.len()) // extension
    }
    fn get_header(&self) -> RtcpCommonHeader {
        let raw_size = self.get_packet_bytes_count_without_padding();
        RtcpCommonHeader {
            version: 2,
            padding: rtp_need_padding(raw_size),
            count: self.report_blocks.len() as u8,
            payload_type: RtcpPayloadType::ReceiverReport,
            length: (self.get_packet_bytes_count() / 4 - 1) as u16,
        }
    }
}

impl<R: io::Read> ReadRemainingFrom<RtcpCommonHeader, R> for RtcpReceiverReport {
    type Error = RtpError;
    fn read_remaining_from(header: RtcpCommonHeader, mut reader: R) -> Result<Self, Self::Error> {
        if header.payload_type != RtcpPayloadType::ReceiverReport {
            return Err(RtpError::WrongPayloadType(format!(
                "expect receiver report payload type but got {:?} instead",
                header.payload_type
            )));
        }

        let sender_ssrc = reader.read_u32::<BigEndian>()?;
        let mut report_blocks = Vec::with_capacity(header.count as usize);
        for _ in 0..header.count {
            report_blocks.push(ReportBlock::read_from(reader.by_ref())?);
        }

        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        let profile_specific_extension = if !buffer.is_empty() {
            Some(Bytes::from(buffer))
        } else {
            None
        };

        Ok(Self {
            header,
            sender_ssrc,
            report_blocks,
            profile_specific_extension,
        })
    }
}

impl<W: io::Write> WriteTo<W> for RtcpReceiverReport {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        let raw_size = self.get_packet_bytes_count_without_padding();
        self.get_header().write_to(writer.by_ref())?;
        writer.write_u32::<BigEndian>(self.sender_ssrc)?;

        self.report_blocks
            .iter()
            .try_for_each(|block| block.write_to(writer.by_ref()))?;

        if let Some(buffer) = &self.profile_specific_extension {
            writer.write_all(buffer)?;
        }

        if let Some(padding) = rtp_make_padding_bytes(raw_size) {
            writer.write_all(&padding)?;
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct RtcpReceiverReportBuilder(RtcpReceiverReport);

impl RtcpReceiverReport {
    pub fn builder() -> RtcpReceiverReportBuilder {
        RtcpReceiverReportBuilder::new()
    }
}

impl RtcpReceiverReportBuilder {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn ssrc(mut self, ssrc: u32) -> Self {
        self.0.sender_ssrc = ssrc;
        self
    }

    pub fn report_block(mut self, block: ReportBlock) -> Self {
        self.0.report_blocks.push(block);
        self
    }

    pub fn report_blocks(mut self, mut blocks: Vec<ReportBlock>) -> Self {
        self.0.report_blocks.append(&mut blocks);
        self
    }

    pub fn extension(mut self, extension_bytes: Bytes) -> Self {
        self.0.profile_specific_extension = Some(extension_bytes);
        self
    }

    pub fn build(mut self) -> RtpResult<RtcpReceiverReport> {
        if self.0.report_blocks.len() > 31 {
            return Err(RtpError::TooManyReportBlocks);
        }
        self.0.header = self.0.get_header();
        Ok(self.0)
    }
}
