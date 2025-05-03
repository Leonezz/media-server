pub mod builder;
pub mod sequencer;

use std::io;

use builder::RtpH264PacketBuilder;
use tokio_util::bytes::Buf;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    reader::{ReadFrom, ReadRemainingFrom},
    writer::WriteTo,
};

use crate::{
    header::RtpHeader,
    packet::RtpTrivialPacket,
    util::{RtpPacketTrait, RtpPaddedPacketTrait, padding::rtp_need_padding},
};

use super::{RtpH264NalUnit, errors::RtpH264Error};

#[derive(Debug)]
pub struct RtpH264Packet {
    pub header: RtpHeader,
    pub payload: RtpH264NalUnit,
}

impl RtpH264Packet {
    pub fn builder() -> RtpH264PacketBuilder {
        Default::default()
    }
}

impl DynamicSizedPacket for RtpH264Packet {
    fn get_packet_bytes_count(&self) -> usize {
        self.header.get_packet_bytes_count() + self.payload.get_packet_bytes_count()
    }
}

impl RtpPaddedPacketTrait for RtpH264Packet {
    fn get_packet_bytes_count_without_padding(&self) -> usize {
        self.get_packet_bytes_count()
    }
}

impl RtpPacketTrait for RtpH264Packet {
    fn get_header(&self) -> RtpHeader {
        let raw_size = self.get_packet_bytes_count_without_padding();
        RtpHeader {
            version: 2,
            padding: rtp_need_padding(raw_size),
            ..self.header.clone()
        }
    }
}

impl<R: io::Read> ReadFrom<R> for RtpH264Packet {
    type Error = RtpH264Error;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let header = RtpHeader::read_from(reader.by_ref())?;
        Self::read_remaining_from(header, reader)
    }
}

impl<R: io::Read> ReadRemainingFrom<RtpHeader, R> for RtpH264Packet {
    type Error = RtpH264Error;
    fn read_remaining_from(header: RtpHeader, mut reader: R) -> Result<Self, Self::Error> {
        let payload = RtpH264NalUnit::read_from(reader.by_ref()).inspect_err(|err| {
            tracing::error!(
                "read rtp h264 failed, rtp_header: {:?}, err: {}",
                header,
                err
            );
            panic!()
        })?;
        Ok(Self { header, payload })
    }
}

impl TryFrom<RtpTrivialPacket> for RtpH264Packet {
    type Error = RtpH264Error;
    fn try_from(value: RtpTrivialPacket) -> Result<Self, Self::Error> {
        Self::read_remaining_from(value.header, value.payload.reader())
    }
}

impl<W: io::Write> WriteTo<W> for RtpH264Packet {
    type Error = RtpH264Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        self.header.write_to(writer.by_ref())?;
        self.payload.write_to(writer.by_ref())?;
        Ok(())
    }
}
