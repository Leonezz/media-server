pub mod builder;
pub mod sequencer;

use std::io;

use builder::RtpH264PacketBuilder;
use utils::traits::{dynamic_sized_packet::DynamicSizedPacket, reader::ReadFrom, writer::WriteTo};

use crate::{
    errors::RtpError,
    header::RtpHeader,
    util::{RtpPacketTrait, padding::rtp_need_padding},
};

use super::RtpH264NalUnit;

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

impl RtpPacketTrait for RtpH264Packet {
    fn get_packet_bytes_count_without_padding(&self) -> usize {
        self.get_packet_bytes_count()
    }

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
    type Error = RtpError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let header = RtpHeader::read_from(reader.by_ref())?;
        let payload = RtpH264NalUnit::read_from(reader.by_ref())?;
        Ok(Self { header, payload })
    }
}

impl<W: io::Write> WriteTo<W> for RtpH264Packet {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        self.header.write_to(writer.by_ref())?;
        self.payload.write_to(writer.by_ref())?;
        Ok(())
    }
}
