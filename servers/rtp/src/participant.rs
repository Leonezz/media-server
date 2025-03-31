use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    rtcp_observer::RtcpObserver, rtp_observer::RtpObserver, sequence_number::SequenceNumber,
};
use num::ToPrimitive;
use rtp_formats::rtcp::{
    RtcpPacket, compound_packet::RtcpCompoundPacket, report_block::ReportBlock,
    simple_ntp::SimpleNtp,
};

use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

#[derive(Debug, Clone)]
pub struct RtpParticipant {
    ssrc: u32,
    cname: Option<String>,
    joined_at: SystemTime,
    rtp_clockrate: u64,

    max_rtp_sequence_number: SequenceNumber,
    rtp_first_sequence_number: SequenceNumber,
    rtp_bad_sequence_number: u64,
    rtp_packets_sent: u64,
    rtp_bytes_sent: u64,
    rtp_packets_probation: u16,
    rtp_packets_sent_prior: u64,
    rtp_packets_expected_prior: u64,

    last_sr_timestamp_ntp: Option<SimpleNtp>,
    last_sr_timestamp: Option<SystemTime>,

    last_rtp_sent_timestamp: Option<SystemTime>,
    last_rtp_sent_timestamp_rtp: Option<u64>,
    last_rtp_interarrvial_jitter: u64,
    last_rtp_sent_rtcp_report_round: u64,
    last_rtcp_sent_timestamp: Option<SystemTime>,
    rtcp_report_round: u64,

    is_sender: bool,
    bye_sent_timestamp: Option<SystemTime>,
}

const MAX_DROPOUT: u16 = 3000;
const MAX_MISORDERR: u16 = 100;
const MIN_SEQUENTIAL: u16 = 2;

impl RtcpObserver for RtpParticipant {
    fn on_rtcp_compound_packet_sent(&mut self, packet: &RtcpCompoundPacket, timestamp: SystemTime) {
        self.last_rtcp_sent_timestamp = Some(timestamp);
        self.rtcp_report_round += 1;
        if self.rtcp_report_round - self.last_rtp_sent_rtcp_report_round > 1 {
            self.is_sender = false;
        }

        packet.packets().iter().for_each(|v| match v {
            RtcpPacket::SenderReport(sender_report) => {
                self.last_sr_timestamp_ntp = Some(sender_report.sender_info.ntp_timestamp);
                self.last_sr_timestamp = Some(timestamp);
                self.is_sender = true;
            }
            RtcpPacket::Bye(_) => {
                self.bye_sent_timestamp = Some(timestamp);
            }
            RtcpPacket::SourceDescription(sdes) => self.cname = sdes.get_cname_of(self.ssrc),
            _ => {}
        });
    }

    fn on_rtcp_compound_packet_received(
        &mut self,
        _packet: &RtcpCompoundPacket,
        _timestamp: SystemTime,
    ) {
        let expected_rtp_packets = self.max_rtp_sequence_number - self.rtp_first_sequence_number;
        self.rtp_packets_expected_prior = expected_rtp_packets.value();
        self.rtp_packets_sent_prior = self.rtp_packets_sent;
    }
}

impl RtpObserver for RtpParticipant {
    fn on_rtp_packet_received(
        &mut self,
        _packet: &rtp_formats::packet::RtpTrivialPacket,
        _timestamp: SystemTime,
    ) {
    }

    fn on_rtp_packet_sent(
        &mut self,
        packet: &rtp_formats::packet::RtpTrivialPacket,
        timestamp: SystemTime,
    ) {
        self.is_sender = true;
        self.last_rtp_sent_rtcp_report_round = self.rtcp_report_round;

        self.update_sequence_number(packet.header.sequence_number);
        if self.rtp_packets_probation == 0 {
            self.rtp_bytes_sent
                .checked_add_signed(packet.get_packet_bytes_count().to_i64().unwrap())
                .unwrap();

            let r: i64 = timestamp
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .to_i64()
                .and_then(|v| v.checked_mul(self.rtp_clockrate.to_i64().unwrap()))
                .and_then(|v| v.checked_div(1000))
                .unwrap();

            let prev_r: i64 = self
                .last_rtp_sent_timestamp
                .map(|v| v.duration_since(UNIX_EPOCH).unwrap().as_millis())
                .and_then(|v| v.to_i64())
                .and_then(|v| v.checked_mul(self.rtp_clockrate.to_i64().unwrap()))
                .and_then(|v| v.checked_div(1000))
                .unwrap();

            let d: i64 = r
                .checked_sub(packet.header.timestamp.to_i64().unwrap())
                .and_then(|v| {
                    v.checked_sub(
                        prev_r
                            .checked_sub(
                                self.last_rtp_sent_timestamp_rtp
                                    .unwrap_or(0)
                                    .to_i64()
                                    .unwrap(),
                            )
                            .unwrap(),
                    )
                })
                .unwrap();

            let jitter_delta = d
                .abs()
                .checked_sub(self.last_rtp_interarrvial_jitter.to_i64().unwrap())
                .and_then(|v| v.checked_div(16))
                .unwrap();
            self.last_rtp_interarrvial_jitter = self
                .last_rtp_interarrvial_jitter
                .to_i64()
                .and_then(|v| v.checked_add(jitter_delta))
                .and_then(|v| v.to_u64())
                .unwrap();
        }
        self.last_rtp_sent_timestamp_rtp = Some(packet.header.timestamp.to_u64().unwrap());
        self.last_rtp_sent_timestamp = Some(timestamp);
    }
}

impl RtpParticipant {
    pub fn new(ssrc: u32, cname: Option<String>, rtp_clockrate: u64) -> Self {
        Self {
            ssrc,
            cname,
            joined_at: SystemTime::now(),
            rtp_clockrate,
            is_sender: false,
            max_rtp_sequence_number: Default::default(),
            rtp_first_sequence_number: Default::default(),
            rtp_bad_sequence_number: u16::MAX as u64 + 2,
            rtp_packets_sent: 0,
            rtp_packets_expected_prior: 0,
            rtp_packets_sent_prior: 0,
            rtp_bytes_sent: 0,
            rtp_packets_probation: MIN_SEQUENTIAL,

            last_sr_timestamp_ntp: Default::default(),
            last_sr_timestamp: None,

            last_rtp_sent_timestamp: None,
            last_rtp_sent_timestamp_rtp: None,
            last_rtp_interarrvial_jitter: 0,
            last_rtp_sent_rtcp_report_round: 0,
            last_rtcp_sent_timestamp: None,
            rtcp_report_round: 0,
            bye_sent_timestamp: None,
        }
    }

    pub fn generate_report_block(&self, current_timestamp: SystemTime) -> ReportBlock {
        ReportBlock::builder()
            .ssrc(self.ssrc)
            .fraction_lost(self.rtp_fraction_lost_report())
            .cumulative_packet_lost(self.rtp_cumulative_packets_lost_report().to_i32().unwrap())
            .highest_sequence_number_received(self.max_rtp_sequence_number.number())
            .highest_sequence_number_cycles(self.max_rtp_sequence_number.round())
            .interarrival_jitter(self.last_rtp_interarrvial_jitter.to_u32().unwrap())
            .last_sr(self.last_sr_timestamp_ntp.unwrap_or_default())
            .delay_since_last_sr(
                self.last_sr_timestamp
                    .map(|v| {
                        current_timestamp.duration_since(v).unwrap().as_nanos() * 65536
                            / 1_000_000_000
                    })
                    .unwrap_or(0) as u32,
            )
            .build()
    }

    pub fn reset_sequence_number(&mut self, sequence_number: u16) {
        self.rtp_first_sequence_number = sequence_number.into();
        self.max_rtp_sequence_number = self.rtp_first_sequence_number;
        self.rtp_packets_sent = 0;
        self.rtp_bad_sequence_number = u16::MAX as u64 + 2;
    }

    pub fn reset(&mut self, ssrc: u32, cname: Option<String>, rtp_clockrate: u64) {
        *self = Self::new(ssrc, cname, rtp_clockrate)
    }

    pub fn ssrc(&self) -> u32 {
        self.ssrc
    }

    pub fn cname(&self) -> Option<&String> {
        self.cname.as_ref()
    }

    pub fn is_sender(&self) -> bool {
        self.is_sender
    }

    pub fn bye_sent(&self) -> bool {
        self.bye_sent_timestamp.is_some()
    }

    fn update_sequence_number(&mut self, sequence_number: u16) -> bool {
        let delta = sequence_number - self.max_rtp_sequence_number.number();
        // probation provides a small gap between the first packet arrive and this participant got statisticed
        if self.rtp_packets_probation > 0 {
            if sequence_number == self.max_rtp_sequence_number.number() + 1_u16 {
                self.rtp_packets_probation -= 1;
                self.max_rtp_sequence_number.set_number(sequence_number);
                if self.rtp_packets_probation == 0 {
                    self.reset_sequence_number(sequence_number);
                    self.rtp_packets_sent += 1;
                    return true;
                }
            } else {
                self.rtp_packets_probation = MIN_SEQUENTIAL - 1;
                self.max_rtp_sequence_number.set_number(sequence_number);
            }
            return false;
        } else if delta < MAX_DROPOUT {
            if sequence_number < self.max_rtp_sequence_number.number() {
                // sequence number wrapped
                self.max_rtp_sequence_number.add_round(1);
            }
            self.max_rtp_sequence_number.set_number(sequence_number);
        } else if delta <= u16::MAX - MAX_MISORDERR + 1 {
            if sequence_number as u64 == self.rtp_bad_sequence_number {
                // two sequential packets with very large sequence number gap,
                // maybe the peer reset the sequence number without telling everyone
                self.reset_sequence_number(sequence_number);
            } else {
                self.rtp_bad_sequence_number = (sequence_number as u64 + 1) & (u16::MAX as u64);
                return false;
            }
        } else {
            // duplicate or reordered packet
        }
        self.rtp_packets_sent += 1;
        true
    }

    pub fn rtp_fraction_lost_report(&self) -> f64 {
        self.rtp_packets_sent_prior as f64 / self.rtp_packets_expected_prior as f64
    }

    pub fn rtp_cumulative_packets_lost_report(&self) -> i64 {
        let expected_rtp_packets = self.max_rtp_sequence_number - self.rtp_first_sequence_number;
        expected_rtp_packets.value() as i64 - self.rtp_packets_sent as i64
    }

    pub fn get_latest_packet_sent_timestamp(&self) -> Option<SystemTime> {
        if self.last_rtcp_sent_timestamp.is_none() && self.last_rtp_sent_timestamp.is_none() {
            return None;
        }

        if self.last_rtcp_sent_timestamp.is_none() {
            return self.last_rtp_sent_timestamp;
        }

        Some(
            self.last_rtcp_sent_timestamp
                .unwrap()
                .max(self.last_rtp_sent_timestamp.unwrap_or(UNIX_EPOCH)),
        )
    }

    pub fn get_joined_timestamp(&self) -> SystemTime {
        self.joined_at
    }
}
