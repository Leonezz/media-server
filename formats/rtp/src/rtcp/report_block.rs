use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io;
use utils::traits::{fixed_packet::FixedPacket, reader::ReadFrom, writer::WriteTo};

use crate::errors::RtpError;

use super::simple_ntp::SimpleShortNtp;

#[derive(Debug, Default, Clone)]
pub struct ReportBlock {
    pub ssrc: u32,
    pub fraction_lost: f64,
    pub cumulative_packet_lost: i32,
    pub highest_sequence_number_received: u16,
    pub sequence_number_cycles: u16,
    pub interarrival_jitter: u32,
    pub last_sender_report_timestamp: SimpleShortNtp,
    /// The delay, expressed in units of 1/65536 seconds,
    /// between receiving the last SR packet from source SSRC n
    /// and sending this reception report block.
    /// If no SR packet has been received yet from SSRC n,
    /// the DLSR field is set to zero.
    pub delay_since_last_sender_report: u32,
}

impl FixedPacket for ReportBlock {
    fn bytes_count() -> usize {
        24
    }
}

impl<R: io::Read> ReadFrom<R> for ReportBlock {
    type Error = RtpError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let ssrc = reader.read_u32::<BigEndian>()?;
        let fraction_lost = reader.read_u8()?;
        let cumulative_packet_lost = reader.read_i24::<BigEndian>()?;
        let sequence_number_cycles = reader.read_u16::<BigEndian>()?;
        let highest_sequence_number_received = reader.read_u16::<BigEndian>()?;
        let interarrival_jitter = reader.read_u32::<BigEndian>()?;
        let last_sender_report_timestamp = reader.read_u32::<BigEndian>()?;
        let delay_since_last_sender_report = reader.read_u32::<BigEndian>()?;
        Ok(Self {
            ssrc,
            fraction_lost: fraction_lost as f64 / 256.0,
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
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write_u8((self.fraction_lost * 256.0) as u8)?;
        writer.write_i24::<BigEndian>(self.cumulative_packet_lost)?;
        writer.write_u16::<BigEndian>(self.sequence_number_cycles)?;
        writer.write_u16::<BigEndian>(self.highest_sequence_number_received)?;
        writer.write_u32::<BigEndian>(self.interarrival_jitter)?;
        writer.write_u32::<BigEndian>(self.last_sender_report_timestamp.into())?;
        writer.write_u32::<BigEndian>(self.delay_since_last_sender_report)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct RtcpReportBlockBuilder(ReportBlock);

impl ReportBlock {
    pub fn builder() -> RtcpReportBlockBuilder {
        Default::default()
    }
}

impl RtcpReportBlockBuilder {
    pub fn ssrc(mut self, ssrc: u32) -> Self {
        self.0.ssrc = ssrc;
        self
    }

    pub fn fraction_lost(mut self, fraction_lost: f64) -> Self {
        self.0.fraction_lost = fraction_lost;
        self
    }

    pub fn cumulative_packet_lost(mut self, cumulative_packet_lost: i32) -> Self {
        self.0.cumulative_packet_lost = cumulative_packet_lost;
        self
    }

    pub fn highest_sequence_number_received(mut self, sequence_number: u16) -> Self {
        self.0.highest_sequence_number_received = sequence_number;
        self
    }

    pub fn highest_sequence_number_cycles(mut self, cycles: u16) -> Self {
        self.0.sequence_number_cycles = cycles;
        self
    }

    pub fn interarrival_jitter(mut self, jitter: u32) -> Self {
        self.0.interarrival_jitter = jitter;
        self
    }

    pub fn last_sr<T: Into<SimpleShortNtp>>(mut self, lsr: T) -> Self {
        self.0.last_sender_report_timestamp = lsr.into();
        self
    }

    pub fn delay_since_last_sr(mut self, dlsr: u32) -> Self {
        self.0.delay_since_last_sender_report = dlsr;
        self
    }

    pub fn build(self) -> ReportBlock {
        self.0
    }
}
