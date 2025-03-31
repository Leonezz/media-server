use std::io::{self};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

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
    report_block::ReportBlock, simple_ntp::SimpleNtp,
};

// @see: RFC 3550 6.4.1 SR: Sender Report RTCP Packet
///         0                   1                   2                   3
///         0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// header |V=2|P|   RC    |   PT=SR=200   |             length            |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                           SSRC of sender                      |
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// sender |             NTP timestamp, most significant word              |
/// info   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |             NTP timestamp, least significant word             |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                         RTP timestamp                         |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                      sender’s packet count                    |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                      sender’s octet count                     |
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// report |                 SSRC_1 (SSRC of first source)                 |
/// block  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///     1  | fraction lost |        cumulative number of packets lost      |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |          extended highest sequence number received            |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                      interarrival jitter                      |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                        last SR (LSR)                          |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                 delay since last SR (DLSR)                    |
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// report |              SSRC_2 (SSRC of second source)                   |
/// block  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///     2  :                            ...                                :
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
///        |                profile-specific extensions                    |
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

#[derive(Debug, Default, Clone)]
pub struct SenderInfo {
    pub ntp_timestamp: SimpleNtp,
    pub rtp_timestamp: u32,
    pub sender_packet_count: u32,
    pub sender_octet_count: u32,
}

impl FixedPacket for SenderInfo {
    fn bytes_count() -> usize {
        20
    }
}

impl<R: io::Read> ReadFrom<R> for SenderInfo {
    type Error = RtpError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let ntp_timestamp = reader.read_u64::<BigEndian>()?;
        let rtp_timestamp = reader.read_u32::<BigEndian>()?;
        let sender_packet_count = reader.read_u32::<BigEndian>()?;
        let sender_octet_count = reader.read_u32::<BigEndian>()?;
        Ok(Self {
            ntp_timestamp: ntp_timestamp.into(),
            rtp_timestamp,
            sender_packet_count,
            sender_octet_count,
        })
    }
}

impl<W: io::Write> WriteTo<W> for SenderInfo {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_u64::<BigEndian>(self.ntp_timestamp.into())?;
        writer.write_u32::<BigEndian>(self.rtp_timestamp)?;
        writer.write_u32::<BigEndian>(self.sender_packet_count)?;
        writer.write_u32::<BigEndian>(self.sender_octet_count)?;
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct RtcpSenderReport {
    pub header: RtcpCommonHeader,
    pub sender_ssrc: u32,
    pub sender_info: SenderInfo,
    pub report_blocks: Vec<ReportBlock>,
    pub profile_specific_extension: Option<Bytes>,
}

impl RtcpSenderReport {
    pub fn builder() -> RtcpSenderReportBuilder {
        RtcpSenderReportBuilder::new()
    }
}

impl DynamicSizedPacket for RtcpSenderReport {
    fn get_packet_bytes_count(&self) -> usize {
        let raw_bytes_count = self.get_packet_bytes_count_without_padding();
        raw_bytes_count + rtp_get_padding_size(raw_bytes_count)
    }
}

impl RtcpPacketSizeTrait for RtcpSenderReport {
    fn get_packet_bytes_count_without_padding(&self) -> usize {
        RtcpCommonHeader::bytes_count() // header
            + 4 // ssrc
            + SenderInfo::bytes_count() // sender info
            + self.report_blocks.len() * ReportBlock::bytes_count() // blocks
            + self.profile_specific_extension.as_ref().map_or_else(|| 0, |v| v.len()) // extension
    }
    fn get_header(&self) -> RtcpCommonHeader {
        let raw_size = self.get_packet_bytes_count_without_padding();
        RtcpCommonHeader {
            version: 2,
            padding: rtp_need_padding(raw_size),
            count: self.report_blocks.len() as u8,
            payload_type: RtcpPayloadType::SenderReport,
            length: (self.get_packet_bytes_count() / 4 - 1) as u16,
        }
    }
}

impl<R: io::Read> ReadRemainingFrom<RtcpCommonHeader, R> for RtcpSenderReport {
    type Error = RtpError;
    fn read_remaining_from(header: RtcpCommonHeader, mut reader: R) -> Result<Self, Self::Error> {
        if header.payload_type != RtcpPayloadType::SenderReport {
            return Err(RtpError::WrongPayloadType(format!(
                "expect sender report payload type, got {:?} instead",
                header.payload_type
            )));
        }

        let sender_ssrc = reader.read_u32::<BigEndian>()?;
        let sender_info = SenderInfo::read_from(reader.by_ref())?;

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
            sender_info,
            report_blocks,
            profile_specific_extension,
        })
    }
}

impl<W: io::Write> WriteTo<W> for RtcpSenderReport {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        self.get_header().write_to(writer.by_ref())?;
        writer.write_u32::<BigEndian>(self.sender_ssrc)?;
        self.sender_info.write_to(writer.by_ref())?;
        self.report_blocks
            .iter()
            .try_for_each(|block| block.write_to(writer.by_ref()))?;

        if let Some(buffer) = &self.profile_specific_extension {
            writer.write_all(buffer)?;
        }

        if let Some(padding) = rtp_make_padding_bytes(self.get_packet_bytes_count_without_padding())
        {
            writer.write_all(&padding)?;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct RtcpSenderReportBuilder(RtcpSenderReport);

impl RtcpSenderReportBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn ssrc(mut self, ssrc: u32) -> Self {
        self.0.sender_ssrc = ssrc;
        self
    }

    pub fn ntp(mut self, ntp: SimpleNtp) -> Self {
        self.0.sender_info.ntp_timestamp = ntp;
        self
    }

    pub fn rtp_timestamp(mut self, rtp_timestamp: u32) -> Self {
        self.0.sender_info.rtp_timestamp = rtp_timestamp;
        self
    }

    pub fn sender_packet_count(mut self, packet_count: u32) -> Self {
        self.0.sender_info.sender_packet_count = packet_count;
        self
    }

    pub fn sender_octet_count(mut self, octet_count: u32) -> Self {
        self.0.sender_info.sender_octet_count = octet_count;
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

    pub fn build(mut self) -> RtpResult<RtcpSenderReport> {
        if self.0.report_blocks.len() > 31 {
            return Err(RtpError::TooManyReportBlocks);
        }
        self.0.header = self.0.get_header();
        Ok(self.0)
    }
}
