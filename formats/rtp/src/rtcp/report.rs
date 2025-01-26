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
    errors::RtpError,
    util::padding::{rtp_get_padding_size, rtp_make_padding_bytes, rtp_need_padding},
};

use super::{
    RtcpPacketTrait,
    common_header::RtcpCommonHeader,
    payload_types::RtcpPayloadType,
    simple_ntp::{SimpleNtp, SimpleShortNtp},
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

#[derive(Debug)]
pub struct SenderInfo {
    ntp_timestamp: SimpleNtp,
    rtp_timestamp: u32,
    sender_packet_count: u32,
    sender_octet_count: u32,
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

#[derive(Debug)]
pub struct ReportBlock {
    ssrc: u32,
    fraction_lost: u8,
    cumulative_packet_lost: u32,
    highest_sequence_number_received: u16,
    sequence_number_cycles: u16,
    interarrival_jitter: u32,
    last_sender_report_timestamp: SimpleShortNtp,
    /// The delay, expressed in units of 1/65536 seconds,
    /// between receiving the last SR packet from source SSRC n
    /// and sending this reception report block.
    /// If no SR packet has been received yet from SSRC n,
    /// the DLSR field is set to zero.
    delay_since_last_sender_report: u32,
}

impl FixedPacket for ReportBlock {
    fn bytes_count() -> usize {
        24
    }
}

impl<R: io::Read> ReadFrom<R> for ReportBlock {
    type Error = RtpError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let ssrc = reader.read_u32::<BigEndian>()?;
        let fraction_lost = reader.read_u8()?;
        let cumulative_packet_lost = reader.read_u24::<BigEndian>()?;
        let sequence_number_cycles = reader.read_u16::<BigEndian>()?;
        let highest_sequence_number_received = reader.read_u16::<BigEndian>()?;
        let interarrival_jitter = reader.read_u32::<BigEndian>()?;
        let last_sender_report_timestamp = reader.read_u32::<BigEndian>()?;
        let delay_since_last_sender_report = reader.read_u32::<BigEndian>()?;
        Ok(Self {
            ssrc,
            fraction_lost,
            cumulative_packet_lost,
            highest_sequence_number_received,
            sequence_number_cycles,
            interarrival_jitter,
            last_sender_report_timestamp: last_sender_report_timestamp.into(),
            delay_since_last_sender_report,
        })
    }
}

impl<W: io::Write> WriteTo<W> for ReportBlock {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write_u8(self.fraction_lost)?;
        writer.write_u24::<BigEndian>(self.cumulative_packet_lost)?;
        writer.write_u16::<BigEndian>(self.sequence_number_cycles)?;
        writer.write_u16::<BigEndian>(self.highest_sequence_number_received)?;
        writer.write_u32::<BigEndian>(self.interarrival_jitter)?;
        writer.write_u32::<BigEndian>(self.last_sender_report_timestamp.into())?;
        writer.write_u32::<BigEndian>(self.delay_since_last_sender_report)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct RtcpSenderReport {
    _header: RtcpCommonHeader,
    sender_ssrc: u32,
    sender_info: SenderInfo,
    report_blocks: Vec<ReportBlock>,
    profile_specific_extension: Option<Bytes>,
}

impl DynamicSizedPacket for RtcpSenderReport {
    fn get_packet_bytes_count(&self) -> usize {
        let raw_bytes_count = self.get_packet_bytes_count_without_padding();
        raw_bytes_count + rtp_get_padding_size(raw_bytes_count)
    }
}

impl RtcpPacketTrait for RtcpSenderReport {
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
            _header: header,
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

#[derive(Debug)]
pub struct RtcpReceiverReport {
    _header: RtcpCommonHeader,
    sender_ssrc: u32,
    report_blocks: Vec<ReportBlock>,
    profile_specific_extension: Option<Bytes>,
}

impl DynamicSizedPacket for RtcpReceiverReport {
    fn get_packet_bytes_count(&self) -> usize {
        let raw_size = self.get_packet_bytes_count_without_padding();
        raw_size + rtp_get_padding_size(raw_size)
    }
}

impl RtcpPacketTrait for RtcpReceiverReport {
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
            _header: header,
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
