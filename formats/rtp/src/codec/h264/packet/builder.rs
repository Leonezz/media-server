use std::io::{Cursor, Read};

use byteorder::ReadBytesExt;
use h264_codec::nalu::{NALUType, NalUnit, NaluHeader};
use tokio_util::bytes::{Buf, Bytes};
use utils::traits::reader::ReadRemainingFrom;
use utils::traits::{dynamic_sized_packet::DynamicSizedPacket, writer::WriteTo};

use crate::codec::h264::errors::{RtpH264Error, RtpH264Result};
use crate::codec::h264::paramters::packetization_mode::PacketizationMode;
use crate::{
    codec::h264::{
        PayloadStructureType, RtpH264NalUnit,
        aggregation::{AggregationNalUnits, AggregationPacketType, stap::StapAFormat},
        fragmented::{FUAPacket, FUHeader, FragmentationUnitPacketType, FragmentedUnit},
        single_nalu::SingleNalUnit,
    },
    header::RtpHeader,
};

use super::RtpH264Packet;

const DEFAULT_MTU: usize = 1455;

#[derive(Debug)]
pub struct RtpH264PacketBuilder {
    packetization_mode: PacketizationMode,
    header: RtpHeader,
    mtu: usize,
    nal_units: Vec<Bytes>,
    sps_nalu: Option<NalUnit>,
    pps_nalu: Option<NalUnit>,
}

impl Default for RtpH264PacketBuilder {
    fn default() -> Self {
        Self {
            packetization_mode: PacketizationMode::SingleNalu,
            header: Default::default(),
            mtu: DEFAULT_MTU,
            nal_units: Default::default(),
            sps_nalu: Default::default(),
            pps_nalu: Default::default(),
        }
    }
}

impl RtpH264PacketBuilder {
    pub fn packetization_mode(mut self, mode: PacketizationMode) -> Self {
        if mode == PacketizationMode::Interleaved {
            tracing::error!("Interleaved packetization mode is not supported");
            return self;
        }
        self.packetization_mode = mode;
        self
    }

    pub fn header(mut self, header: RtpHeader) -> Self {
        self.header = header;
        self
    }

    pub fn mtu(mut self, mtu: usize) -> Self {
        self.mtu = mtu;
        self
    }

    pub fn nalu(mut self, nalu: Bytes) -> Self {
        self.nal_units.push(nalu);
        self
    }

    pub fn build(mut self) -> RtpH264Result<Vec<RtpH264Packet>> {
        let mut result = Vec::with_capacity(self.nal_units.len());

        self.nal_units
            .clone()
            .into_iter()
            .try_for_each(|nalu_bytes| {
                let nalu = self.packetize_nal_unit(nalu_bytes)?;
                if let Some(valid_nalu) = nalu {
                    result.extend(valid_nalu.into_iter().map(|v| RtpH264Packet {
                        header: self.header.clone(),
                        payload: v,
                    }));
                }
                Ok::<(), RtpH264Error>(())
            })?;

        Ok(result)
    }

    fn packetize_single_nalu(&mut self, nalu: Bytes) -> RtpH264Result<Option<Vec<RtpH264NalUnit>>> {
        if nalu.is_empty() {
            return Ok(None);
        }

        let nalu_bytes_length = nalu.len();
        let mut cursor = Cursor::new(nalu);
        let nalu_header: NaluHeader = cursor.read_u8()?.try_into()?;

        if nalu_bytes_length <= self.mtu {
            return Ok(Some(vec![RtpH264NalUnit::SingleNalu(SingleNalUnit(
                NalUnit::read_remaining_from(nalu_header, cursor.by_ref())?,
            ))]));
        }

        Err(RtpH264Error::InvalidMTU(self.mtu))
    }

    fn packetize_non_interleaved(
        &mut self,
        nalu: Bytes,
    ) -> RtpH264Result<Option<Vec<RtpH264NalUnit>>> {
        if nalu.is_empty() {
            return Ok(None);
        }

        let nalu_bytes_length = nalu.len();
        let mut cursor = Cursor::new(nalu);
        let nalu_header: NaluHeader = cursor.read_u8()?.try_into()?;

        // if nalu_header.nal_unit_type == NALUType::AccessUnitDelimiter
        //     || nalu_header.nal_unit_type == NALUType::FillerData
        // {
        //     // skip these?
        //     return Ok(None);
        // }

        if nalu_header.nal_unit_type == NALUType::SPS {
            self.sps_nalu = Some(NalUnit::read_remaining_from(nalu_header, cursor.by_ref())?);
            return Ok(None);
        }

        if nalu_header.nal_unit_type == NALUType::PPS {
            self.pps_nalu = Some(NalUnit::read_remaining_from(nalu_header, cursor.by_ref())?);
            return Ok(None);
        }

        let mut result = Vec::new();
        if let (Some(sps), Some(pps)) = (self.sps_nalu.clone(), self.pps_nalu.clone()) {
            if sps.get_packet_bytes_count()
                + pps.get_packet_bytes_count()
                + nalu_bytes_length
                + 1 // stap-a nal hdr
                + 2 // sps nalu size
                + 2 // pps nalu size
                + 2 // nalu size
                <= self.mtu
            {
                let stap_a_unit = StapAFormat {
                    header: PayloadStructureType::AggregationPacket(AggregationPacketType::STAPA)
                        .into(),
                    nal_units: vec![
                        sps,
                        pps,
                        NalUnit::read_remaining_from(nalu_header, cursor.by_ref())?,
                    ],
                };

                return Ok(Some(vec![RtpH264NalUnit::Aggregated(
                    AggregationNalUnits::StapA(stap_a_unit),
                )]));
            }

            if sps.get_packet_bytes_count() + pps.get_packet_bytes_count() + 1 + 2 + 2 <= self.mtu {
                let stap_a_unit = StapAFormat {
                    header: PayloadStructureType::AggregationPacket(AggregationPacketType::STAPA)
                        .into(),
                    nal_units: vec![sps, pps],
                };

                result.push(RtpH264NalUnit::Aggregated(AggregationNalUnits::StapA(
                    stap_a_unit,
                )));
            } else {
                if sps.get_packet_bytes_count() <= self.mtu {
                    result.push(RtpH264NalUnit::SingleNalu(SingleNalUnit(sps)));
                } else {
                    let mut sps_bytes = Vec::with_capacity(sps.get_packet_bytes_count());
                    sps.write_to(&mut sps_bytes)?;
                    let fragmented_sps = self.packetize_nal_unit(Bytes::from(sps_bytes))?;
                    if let Some(sps_fragments) = fragmented_sps {
                        result.extend(sps_fragments);
                    }
                }

                if pps.get_packet_bytes_count() <= self.mtu {
                    result.push(RtpH264NalUnit::SingleNalu(SingleNalUnit(pps)));
                } else {
                    let mut pps_bytes = Vec::with_capacity(pps.get_packet_bytes_count());
                    pps.write_to(&mut pps_bytes)?;
                    let fragmented_pps = self.packetize_nal_unit(Bytes::from(pps_bytes))?;
                    if let Some(pps_fragments) = fragmented_pps {
                        result.extend(pps_fragments);
                    }
                }
            }
        }

        if self.sps_nalu.is_some() && self.pps_nalu.is_some() {
            // both must have been processed before
            self.sps_nalu = None;
            self.pps_nalu = None;
        }

        if nalu_bytes_length <= self.mtu {
            result.push(RtpH264NalUnit::SingleNalu(SingleNalUnit(
                NalUnit::read_remaining_from(nalu_header, cursor.by_ref())?,
            )));
            return Ok(Some(result));
        }

        let max_fragment_size = self.mtu as isize - 2; // indicator + fu_header

        if max_fragment_size <= 0 {
            return Err(RtpH264Error::InvalidMTU(self.mtu));
        }

        let mut start_fragment = true;
        while cursor.has_remaining() {
            let current_fragment_size =
                std::cmp::min(max_fragment_size as usize, cursor.remaining());
            let mut fragment_bytes = vec![0; current_fragment_size];
            cursor.read_exact(&mut fragment_bytes)?;

            let is_end = !cursor.has_remaining();
            result.push(RtpH264NalUnit::Fragmented(FragmentedUnit::FuA(FUAPacket {
                indicator: <PayloadStructureType as Into<u8>>::into(
                    PayloadStructureType::FragmentationUnit(FragmentationUnitPacketType::FUA),
                ) | (nalu_header.nal_ref_idc << 5),
                fu_header: FUHeader {
                    start_bit: start_fragment,
                    end_bit: is_end,
                    reserved_bit: false,
                    nalu_type: nalu_header.nal_unit_type.into(),
                },
                payload: Bytes::from(fragment_bytes),
            })));

            start_fragment = false;
        }
        Ok(Some(result))
    }

    fn packetize_nal_unit(&mut self, nalu: Bytes) -> RtpH264Result<Option<Vec<RtpH264NalUnit>>> {
        match self.packetization_mode {
            PacketizationMode::SingleNalu => self.packetize_single_nalu(nalu),
            PacketizationMode::NonInterleaved => self.packetize_non_interleaved(nalu),
            // it is not reasonable to support interleaved packetization mode for a media server
            PacketizationMode::Interleaved => Err(RtpH264Error::UnsupportedPacketizationMode(
                self.packetization_mode,
            )),
        }
    }
}
