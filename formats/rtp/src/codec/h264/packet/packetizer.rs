use crate::codec::h264::aggregation::{AggregatedHeader, AggregatedPayload};
use crate::codec::h264::errors::{RtpH264Error, RtpH264Result};
use crate::codec::h264::fragmented::FuIndicator;
use crate::codec::h264::packet::RtpH264Packet;
use crate::codec::h264::paramters::packetization_mode::PacketizationMode;
use crate::errors::RtpError;
use crate::header::RtpHeaderBuilder;
use crate::packet::RtpTrivialPacket;
use crate::packet::packetizer::{RtpTrivialPacketPacketizer, wallclock_to_rtp_timestamp};
use crate::payload_types::rtp_payload_type::{get_video_rtp_payload_type, video_get_rtp_clockrate};
use crate::{
    codec::h264::{
        RtpH264NalUnit,
        aggregation::{AggregationNalUnits, AggregationPacketType, stap::StapAFormat},
        fragmented::{FUAPacket, FUHeader, FragmentationUnitPacketType, FragmentedUnit},
        single_nalu::SingleNalUnit,
    },
    header::RtpHeader,
};
use codec_common::video::VideoCodecCommon;
use codec_h264::nalu::NalUnit;
use codec_h264::nalu_type::NALUType;
use num::ToPrimitive;
use std::cmp;
use std::io::Read;
use tokio_util::bytes::{Buf, Bytes};
use utils::random::{random_u16, random_u32};
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;
use utils::traits::writer::WriteTo;

const DEFAULT_MTU: usize = 1455;

#[derive(Debug)]
pub struct RtpH264PacketPacketizer {
    packetization_mode: PacketizationMode,
    header: RtpHeader,
    mtu: usize,
    nal_units: Vec<NalUnit>,
    sps_nalu: Option<NalUnit>,
    pps_nalu: Option<NalUnit>,
    first_frame_timestamp: Option<u64>,
    last_frame_timestamp: Option<u64>,
    rtp_timestamp_base: u64,
    rtp_clockrate: u64,
}

impl Default for RtpH264PacketPacketizer {
    fn default() -> Self {
        Self {
            packetization_mode: PacketizationMode::SingleNalu,
            header: RtpHeaderBuilder::new()
                .version(2)
                .payload_type(get_video_rtp_payload_type(VideoCodecCommon::AVC).unwrap())
                .sequence_number(random_u16())
                .build(),
            mtu: DEFAULT_MTU,
            nal_units: Default::default(),
            sps_nalu: Default::default(),
            pps_nalu: Default::default(),
            first_frame_timestamp: None,
            last_frame_timestamp: None,
            rtp_clockrate: video_get_rtp_clockrate(VideoCodecCommon::AVC).unwrap(),
            rtp_timestamp_base: random_u32().to_u64().unwrap(),
        }
    }
}

impl RtpH264PacketPacketizer {
    pub fn new(mtu: usize, mode: PacketizationMode, ssrc: u32) -> Self {
        let mut default = Self::default();
        default.header.ssrc = ssrc;
        Self {
            packetization_mode: mode,
            mtu,
            rtp_clockrate: video_get_rtp_clockrate(VideoCodecCommon::AVC).unwrap(),
            ..default
        }
    }

    pub fn packetization_mode(&mut self, mode: PacketizationMode) -> &mut Self {
        if mode == PacketizationMode::Interleaved {
            tracing::error!("Interleaved packetization mode is not supported");
            return self;
        }
        self.packetization_mode = mode;
        self
    }

    pub fn mtu(&mut self, mtu: usize) -> &mut Self {
        self.mtu = mtu;
        self
    }

    pub fn nalu(&mut self, nalu: NalUnit) -> &mut Self {
        self.nal_units.push(nalu);
        self
    }

    fn packetize_sps_pps(&mut self) -> RtpH264Result<Vec<RtpH264NalUnit>> {
        if self.sps_nalu.is_none() || self.pps_nalu.is_none() {
            return Ok(vec![]);
        }
        let (sps, pps) = (self.sps_nalu.take().unwrap(), self.pps_nalu.take().unwrap());
        if !matches!(self.packetization_mode, PacketizationMode::SingleNalu)
            && let Ok(res) = self.packetize_aggregated_nalu(vec![sps.clone(), pps.clone()])
        {
            return Ok(vec![res]);
        }

        let mut result = vec![];
        for item in [sps, pps].into_iter() {
            if let Ok(res) = self.packetize_single_nalu(item.clone()) {
                result.push(res);
            } else if matches!(self.packetization_mode, PacketizationMode::SingleNalu) {
                return Err(RtpH264Error::InvalidPacketizationMode(format!(
                    "cannot packetize nalu {:?} of size {} in single nalu mode",
                    item.header,
                    item.get_packet_bytes_count()
                )));
            } else {
                result.extend(self.packetize_fragmented_nalu(item)?);
            }
        }
        Ok(result)
    }

    fn packetize_fragmented_nalu(&self, nalu: NalUnit) -> RtpH264Result<Vec<RtpH264NalUnit>> {
        let max_fragment_size =
            self.mtu as isize - 2 - self.header.get_packet_bytes_count() as isize; // indicator + fu_header

        if max_fragment_size <= 0 {
            return Err(RtpH264Error::InvalidMTU(self.mtu));
        }

        let mut result = vec![];
        let mut nalu_bytes = vec![];
        // always use write_to to serialize nalu
        nalu.write_to(&mut nalu_bytes)?;
        let mut cursor = std::io::Cursor::new(&nalu_bytes[1..]);
        let mut start_fragment = true;
        while cursor.has_remaining() {
            let current_fragment_size =
                std::cmp::min(max_fragment_size as usize, cursor.remaining());
            let mut fragment_bytes = vec![0; current_fragment_size];
            cursor.read_exact(&mut fragment_bytes)?;

            let is_end = !cursor.has_remaining();
            result.push(RtpH264NalUnit::Fragmented(FragmentedUnit::FuA(FUAPacket {
                indicator: FuIndicator {
                    forbidden_zero_bit: false,
                    nal_ref_idc: nalu.header.nal_ref_idc,
                    fu_type: FragmentationUnitPacketType::FUA,
                },
                fu_header: FUHeader {
                    start_bit: start_fragment,
                    end_bit: is_end,
                    reserved_bit: false,
                    nalu_type: nalu.header.nal_unit_type.into(),
                },
                payload: Bytes::from(fragment_bytes),
            })));

            start_fragment = false;
        }
        Ok(result)
    }

    fn packetize_single_nalu(&self, nalu: NalUnit) -> RtpH264Result<RtpH264NalUnit> {
        let nalu_bytes_length = nalu.get_packet_bytes_count();
        if nalu_bytes_length + self.header.get_packet_bytes_count() <= self.mtu {
            return Ok(RtpH264NalUnit::SingleNalu(SingleNalUnit(nalu)));
        }

        Err(RtpH264Error::InvalidMTU(self.mtu))
    }

    fn aggreated_check_mtu_with_length(&self, nalu_total_size: usize, nalu_cnt: usize) -> bool {
        nalu_total_size
            + self.header.get_packet_bytes_count()
            + 1 // stap-a nal hdr
            + nalu_cnt * 2
            <= self.mtu
    }

    fn aggreated_check_mtu(&self, nalus: &[NalUnit]) -> bool {
        let nalu_bytes_length = nalus
            .iter()
            .fold(0, |prev, item| prev + item.get_packet_bytes_count());
        self.aggreated_check_mtu_with_length(nalu_bytes_length, nalus.len())
    }

    fn packetize_aggregated_nalu(&self, nalus: Vec<NalUnit>) -> RtpH264Result<RtpH264NalUnit> {
        if !self.aggreated_check_mtu(&nalus) {
            return Err(RtpH264Error::InvalidMTU(self.mtu));
        }

        let nal_ref_idc = nalus
            .iter()
            .fold(0, |prev, item| cmp::max(prev, item.header.nal_ref_idc));
        Ok(RtpH264NalUnit::Aggregated(AggregationNalUnits {
            header: AggregatedHeader {
                forbidden_zero_bit: false,
                nal_ref_idc,
                aggregate_type: AggregationPacketType::STAPA,
            },
            payload: AggregatedPayload::StapA(StapAFormat { nal_units: nalus }),
        }))
    }

    fn packetize_non_interleaved(&mut self) -> RtpH264Result<Vec<RtpH264NalUnit>> {
        if self.nal_units.is_empty() {
            return Ok(vec![]);
        }
        let mut result = vec![];
        result.extend(self.packetize_sps_pps()?);
        // if self.nal_units.len() == 1 {
        //     let nalu = self.nal_units.remove(0);
        //     if let Ok(res) = self.packetize_single_nalu(nalu.clone()) {
        //         result.push(res);
        //     } else {
        //         result.extend(self.packetize_fragmented_nalu(nalu)?);
        //     }
        //     return Ok(result);
        // }

        let mut fragments = vec![];
        let (mut start_idx, mut total_size) = (0, 0);
        for (idx, nalu) in self.nal_units.iter().enumerate() {
            let nalu_size = nalu.get_packet_bytes_count();
            if !self.aggreated_check_mtu_with_length(nalu_size, 1) {
                if !total_size == 0 {
                    fragments.push((start_idx, idx - start_idx));
                }
                fragments.push((idx, 1));
                start_idx = idx + 1;
                total_size = 0;
                continue;
            }
            total_size += nalu_size;
            let nalu_cnt = idx - start_idx + 1;
            if self.aggreated_check_mtu_with_length(total_size, nalu_cnt) {
                continue;
            }
            fragments.push((start_idx, nalu_cnt - 1));
            start_idx = idx;
            total_size = nalu_size;
        }
        if total_size != 0 {
            fragments.push((start_idx, self.nal_units.len() - start_idx));
        }

        for (start, len) in fragments {
            let mut nalus_slice: Vec<NalUnit> = self.nal_units[start..start + len].to_vec();
            if nalus_slice.len() == 1 {
                let nalu = nalus_slice.remove(0);
                if let Ok(res) = self.packetize_single_nalu(nalu.clone()) {
                    result.push(res);
                } else {
                    result.extend(self.packetize_fragmented_nalu(nalu)?);
                }
            } else {
                result.push(self.packetize_aggregated_nalu(nalus_slice)?);
            }
        }

        Ok(result)
    }

    fn make_packets(&mut self) -> RtpH264Result<Vec<RtpH264NalUnit>> {
        let mut result = vec![];
        match self.packetization_mode {
            PacketizationMode::SingleNalu => {
                result.extend(self.packetize_sps_pps()?);
                let nalus: Vec<_> = self.nal_units.drain(..).collect();
                for nalu in nalus {
                    result.push(self.packetize_single_nalu(nalu)?);
                }
            }
            PacketizationMode::NonInterleaved => {
                result.extend(self.packetize_non_interleaved()?);
            }
            // it is not reasonable to support interleaved packetization mode for a media server
            PacketizationMode::Interleaved => {
                return Err(RtpH264Error::UnsupportedPacketizationMode(
                    self.packetization_mode,
                ));
            }
        };
        self.nal_units.clear();
        Ok(result)
    }
}

impl RtpTrivialPacketPacketizer for RtpH264PacketPacketizer {
    fn build(&mut self) -> Result<Vec<RtpTrivialPacket>, RtpError> {
        let packets = self
            .make_packets()
            .map_err(|e| RtpError::H264PacketizationFailed(format!("{}", e)))?;
        let packets_cnt = packets.len();

        let mut header = self.rtp_header().clone();
        header.timestamp = wallclock_to_rtp_timestamp(
            self.last_frame_timestamp.unwrap(),
            self.first_frame_timestamp.unwrap(),
            self.rtp_timestamp_base,
            self.rtp_clockrate,
        ) as u32;
        let mut result = vec![];
        for (idx, item) in packets.into_iter().enumerate() {
            let marker = idx == packets_cnt - 1;
            let trivial_packet: RtpTrivialPacket = RtpH264Packet {
                header: RtpHeader {
                    marker,
                    ..header.clone()
                },
                payload: item,
            }
            .try_into()
            .map_err(|err| {
                RtpError::H264PacketizationFailed(format!(
                    "convert to trivial packet failed: {}",
                    err
                ))
            })?;
            header.sequence_number = header.sequence_number.wrapping_add(1);
            result.push(trivial_packet);
        }
        self.header.sequence_number = header.sequence_number;
        Ok(result)
    }

    fn packetize(
        &mut self,
        item: crate::packet::packetizer::RtpPacketizerItem,
    ) -> Result<(), RtpError> {
        match item {
            crate::packet::packetizer::RtpPacketizerItem::Video(video_item) => match video_item {
                crate::packet::packetizer::RtpPacketizerVideoItem::H264(h264_item) => {
                    for nalu in h264_item.nalus {
                        if matches!(nalu.header.nal_unit_type, NALUType::SPS) {
                            self.sps_nalu = Some(nalu);
                        } else if matches!(nalu.header.nal_unit_type, NALUType::PPS) {
                            self.pps_nalu = Some(nalu);
                        } else {
                            self.nal_units.push(nalu);
                        }
                    }
                    Ok(())
                }
            },
            _ => {
                debug_assert!(false, "only h264 video item is supported");
                Err(RtpError::H264PacketizationFailed(
                    "only h264 video item is supported".to_string(),
                ))
            }
        }
    }

    fn set_rtp_header(&mut self, mut header: RtpHeader) {
        header.sequence_number = self.header.sequence_number;
        self.header = header;
    }

    fn get_rtp_clockrate(&self) -> u64 {
        self.rtp_clockrate
    }

    fn set_frame_timestamp(&mut self, timestamp: u64) {
        self.last_frame_timestamp = Some(timestamp);
        if self.first_frame_timestamp.is_none() {
            self.first_frame_timestamp = Some(timestamp)
        }
    }

    fn rtp_header(&self) -> &RtpHeader {
        &self.header
    }
}
