use crate::{
    errors::{RtpSessionError, RtpSessionResult},
    participant::RtpParticipant,
    rtcp_observer::RtcpObserver,
    rtp_observer::RtpObserver,
};
use num::ToPrimitive;
use rtp_formats::rtcp::{
    RtcpPacket, RtcpPacketTrait, bye::RtcpByePacket, compound_packet::RtcpCompoundPacket,
    receiver_report::RtcpReceiverReport, sdes::RtcpSourceDescriptionPacket,
    sender_report::RtcpSenderReport,
};
use std::{
    collections::HashMap,
    ops::Mul,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use utils::{
    random::{random_u32, uniform_random_f64},
    traits::dynamic_sized_packet::DynamicSizedPacket,
};

pub trait RtpSessionObserver: RtpObserver + RtcpObserver + Send + Sync {}

pub(crate) struct RtcpContext {
    ssrc: u32,
    tp: SystemTime,
    tn: SystemTime,
    pmembers: u64,
    participants: HashMap<u32, RtpParticipant>,
    rtcp_bw: u64,
    avg_rtcp_size: u64,
    initial: bool,
    about_to_send_bye: bool,
    rtp_clockrate: u64,
    session_observers: Vec<Box<dyn RtpSessionObserver>>,
}

impl RtcpObserver for RtcpContext {
    fn on_rtcp_compound_packet_received(
        &mut self,
        packet: &rtp_formats::rtcp::compound_packet::RtcpCompoundPacket,
        timestamp: SystemTime,
    ) {
        packet.packets().iter().for_each(|item| {
            if let Some(ssrc) = item.sender_ssrc() {
                self.add_participant(ssrc, None);
                self.participants
                    .entry(ssrc)
                    .and_modify(|p| p.on_rtcp_compound_packet_sent(packet, timestamp));
            }

            item.csrc_list().iter().for_each(|csrc| {
                self.add_participant(*csrc, None);
                self.participants
                    .entry(*csrc)
                    .and_modify(|p| p.on_rtcp_compound_packet_sent(packet, timestamp));
            });

            if let RtcpPacket::Bye(bye) = item {
                bye.ssrc_list
                    .iter()
                    .for_each(|v| self.on_bye_packet_received(*v, bye.leave_reason.clone()));
            }
        });
        self.update_avg_rtcp_size(packet.get_packet_bytes_count().to_u64().unwrap());

        self.session_observers
            .iter_mut()
            .for_each(|item| item.on_rtcp_compound_packet_received(packet, timestamp));
    }

    fn on_rtcp_compound_packet_sent(
        &mut self,
        packet: &rtp_formats::rtcp::compound_packet::RtcpCompoundPacket,
        timestamp: SystemTime,
    ) {
        let t_d = self.compute_deterministic_interval_ms();
        let t = Self::compute_interval_ms(t_d) as u64;
        if self.tn <= timestamp {
            self.tp = timestamp;
            self.tn = timestamp.checked_add(Duration::from_millis(t)).unwrap();
        } else {
            self.tn = self.tp.checked_add(Duration::from_millis(t)).unwrap();
        }
        self.initial = false;
        self.update_avg_rtcp_size(packet.get_packet_bytes_count().to_u64().unwrap());
        self.participants
            .entry(self.ssrc)
            .and_modify(|p| p.on_rtcp_compound_packet_received(packet, timestamp));

        self.session_observers
            .iter_mut()
            .for_each(|item| item.on_rtcp_compound_packet_sent(packet, timestamp));
    }
}

impl RtpObserver for RtcpContext {
    fn on_rtp_packet_received(
        &mut self,
        packet: &rtp_formats::packet::RtpTrivialPacket,
        timestamp: SystemTime,
    ) {
        self.add_participant(packet.header.ssrc, None);
        packet.header.csrc_list.iter().for_each(|csrc| {
            self.add_participant(*csrc, None);
        });
        self.participants.entry(packet.header.ssrc).and_modify(|p| {
            p.on_rtp_packet_sent(packet, timestamp);
        });
        packet.header.csrc_list.iter().for_each(|csrc| {
            self.participants
                .entry(*csrc)
                .and_modify(|p| p.on_rtp_packet_sent(packet, timestamp));
        });

        self.session_observers
            .iter_mut()
            .for_each(|item| item.on_rtp_packet_received(packet, timestamp));
    }

    fn on_rtp_packet_sent(
        &mut self,
        packet: &rtp_formats::packet::RtpTrivialPacket,
        timestamp: SystemTime,
    ) {
        self.participants
            .entry(self.ssrc)
            .and_modify(|p| p.on_rtp_packet_sent(packet, timestamp));

        self.session_observers
            .iter_mut()
            .for_each(|item| item.on_rtp_packet_sent(packet, timestamp));
    }
}

impl RtcpContext {
    pub fn new(
        session_bandwidth: u64,
        rtp_clockrate: u64,
        cname: Option<String>,
        ssrc: u32,
    ) -> Self {
        let mut ctx = RtcpContext {
            ssrc,
            tp: UNIX_EPOCH,
            tn: UNIX_EPOCH,
            pmembers: 0,
            participants: HashMap::new(),
            rtcp_bw: 0,
            avg_rtcp_size: 0,
            initial: true,
            about_to_send_bye: false,
            rtp_clockrate,
            session_observers: Vec::new(),
        };

        ctx.reset(None, cname, session_bandwidth, rtp_clockrate);
        ctx
    }

    pub fn with_observer(&mut self, observer: Box<dyn RtpSessionObserver>) {
        self.session_observers.push(observer);
    }

    pub fn reset(
        &mut self,
        ssrc: Option<u32>,
        cname: Option<String>,
        session_bandwidth: u64,
        rtp_clockrate: u64,
    ) {
        self.ssrc = ssrc.unwrap_or(random_u32());
        self.tp = UNIX_EPOCH;
        self.pmembers = 1;
        self.rtcp_bw = session_bandwidth
            .checked_mul(5)
            .and_then(|v| v.checked_div(100))
            .unwrap();
        self.avg_rtcp_size = 0; // TODO(zhuwenq): calculate it
        self.initial = true;
        self.about_to_send_bye = false;
        self.participants.clear();
        self.participants.insert(
            self.ssrc,
            RtpParticipant::new(self.ssrc, cname, rtp_clockrate),
        );

        let t_d = self.compute_deterministic_interval_ms();
        self.tn = SystemTime::now()
            .checked_add(Duration::from_millis(
                Self::compute_interval_ms(t_d).to_u64().unwrap(),
            ))
            .unwrap();
        self.rtp_clockrate = rtp_clockrate;
    }

    fn senders_count(&self) -> u64 {
        self.participants
            .values()
            .filter(|p| p.is_sender() && !p.bye_sent())
            .count() as u64
    }

    fn members_count(&self) -> u64 {
        self.participants.values().filter(|p| !p.bye_sent()).count() as u64
    }

    fn add_participant(&mut self, ssrc: u32, cname: Option<String>) {
        self.participants.entry(ssrc).or_insert(RtpParticipant::new(
            ssrc,
            cname,
            self.rtp_clockrate,
        ));
        self.pmembers = self.members_count();
    }

    fn compute_deterministic_interval_ms(&self) -> f64 {
        let c: f64;
        let n: f64;
        let senders = self.senders_count() as f64;
        let members = self.members_count() as f64;
        if senders / members > 0.25 {
            if self.participants.get(&self.ssrc).unwrap().is_sender() {
                c = (self.avg_rtcp_size as f64) * 4.0 / (self.rtcp_bw as f64);
                n = senders;
            } else {
                c = (self.avg_rtcp_size as f64) * 4.0 / ((self.rtcp_bw as f64) * 3.0);
                n = members - senders;
            }
        } else {
            c = (self.avg_rtcp_size as f64) / (self.rtcp_bw as f64);
            n = members;
        }

        let t_min: f64 = if self.initial { 2500.0 } else { 5000.0 };
        let t_d: f64 = t_min.max(c * n);
        t_d
    }

    fn compute_interval_ms(t_d: f64) -> f64 {
        uniform_random_f64(0.5 * t_d, 1.5 * t_d)
    }

    fn update_avg_rtcp_size(&mut self, packet_size: u64) {
        self.avg_rtcp_size =
            ((packet_size as f64) / 16.0 + (15.0 * (self.avg_rtcp_size as f64)) / 16.0) as u64;
    }

    pub fn timed_out(&self, current_timestamp: SystemTime) -> bool {
        current_timestamp > self.tn
    }

    pub fn check_timeout(&mut self) {
        let tc = SystemTime::now();
        let t_d = self.compute_deterministic_interval_ms();
        let t = Self::compute_interval_ms(t_d);
        let is_sender = self.participants.get(&self.ssrc).unwrap().is_sender();
        self.participants.retain(|ssrc, p| {
            if ssrc.eq(&self.ssrc) {
                return true;
            }
            if p.is_sender()
                && tc
                    .duration_since(
                        p.get_latest_packet_sent_timestamp()
                            .unwrap_or(p.get_joined_timestamp()),
                    )
                    .unwrap()
                    .as_millis()
                    .gt(&t.to_u128().unwrap().checked_mul(2).unwrap())
            {
                return false;
            }

            const M: u64 = 5;
            if is_sender
                && tc
                    .duration_since(
                        p.get_latest_packet_sent_timestamp()
                            .unwrap_or(p.get_joined_timestamp()),
                    )
                    .unwrap()
                    .as_millis()
                    .gt(&t.to_u128().unwrap().checked_mul(M.into()).unwrap())
            {
                return false;
            }

            true
        });
    }

    fn on_bye_packet_received(&mut self, _ssrc: u32, _reason: Option<String>) {
        let tc = SystemTime::now();

        let members_count = self.members_count();
        self.pmembers = members_count;

        // reverse reconsideration
        self.tn = tc
            .checked_add(Duration::from_millis(
                (self
                    .tn
                    .duration_since(tc)
                    .unwrap_or_default()
                    .as_millis()
                    .to_f64()
                    .unwrap()
                    * members_count.to_f64().unwrap()
                    / self.pmembers.to_f64().unwrap())
                .to_u64()
                .unwrap(),
            ))
            .unwrap();
        self.tp = tc
            .checked_sub(Duration::from_millis(
                (tc.duration_since(self.tp)
                    .unwrap_or_default()
                    .as_millis()
                    .to_f64()
                    .unwrap()
                    * members_count.to_f64().unwrap()
                    / self.pmembers.to_f64().unwrap())
                .to_u64()
                .unwrap(),
            ))
            .unwrap();
    }

    fn generate_report_blocks(
        &self,
        current_timestamp: SystemTime,
    ) -> Vec<rtp_formats::rtcp::report_block::ReportBlock> {
        let count = self.participants.len().min(31);
        let iter = self.participants.values().take(count);
        iter.map(|v| v.generate_report_block(current_timestamp))
            .collect()
    }

    fn generate_sender_report(
        &self,
        rtp_timestamp: u32,
        current_timestamp: SystemTime,
    ) -> RtpSessionResult<RtcpSenderReport> {
        rtp_formats::rtcp::sender_report::RtcpSenderReport::builder()
            .ssrc(self.ssrc)
            .ntp(current_timestamp.into())
            .rtp_timestamp(rtp_timestamp) // TODO: replace with rtp timestamp
            .report_blocks(self.generate_report_blocks(current_timestamp))
            .build()
            .map_err(RtpSessionError::RtpFormatError)
    }

    fn generate_receiver_report(
        &self,
        current_timestamp: SystemTime,
    ) -> RtpSessionResult<RtcpReceiverReport> {
        rtp_formats::rtcp::receiver_report::RtcpReceiverReport::builder()
            .report_blocks(self.generate_report_blocks(current_timestamp))
            .build()
            .map_err(RtpSessionError::RtpFormatError)
    }

    fn generate_sdes(&self) -> RtpSessionResult<RtcpSourceDescriptionPacket> {
        let cname = self
            .participants
            .get(&self.ssrc)
            .unwrap_or_else(|| {
                panic!(
                    "missing self in participants, something must be wrong, self ssrc: {}",
                    self.ssrc
                )
            })
            .cname();
        let builder = RtcpSourceDescriptionPacket::builder();
        if let Some(cname) = cname {
            builder
                .cname(self.ssrc, cname.clone())
                .unwrap()
                .build()
                .map_err(RtpSessionError::RtpFormatError)
        } else {
            builder.build().map_err(RtpSessionError::RtpFormatError)
        }
    }

    pub fn generate_rtcp_compound_packet(
        &self,
        current_timestamp: SystemTime,
        bye: bool,
        bye_reason: Option<String>,
        with_packets: Vec<RtcpPacket>,
    ) -> RtpSessionResult<RtcpCompoundPacket> {
        let mut builder = RtcpCompoundPacket::builder();
        let participant_self = self.participants.get(&self.ssrc).unwrap_or_else(|| {
            panic!(
                "missing self in participants, something must be wrong, self ssrc: {}",
                self.ssrc
            )
        });
        if participant_self.is_sender() {
            let first_rtp_sent_timestamp = participant_self.first_rtp_sent_timestamp();
            let first_rtp_sent_timestamp_rtp = participant_self.first_rtp_sent_timestamp_rtp();
            if let Some(first_rtp_sent_timestamp) = first_rtp_sent_timestamp
                && let Some(first_rtp_sent_timestamp_rtp) = first_rtp_sent_timestamp_rtp
            {
                let rtp_timestamp = first_rtp_sent_timestamp_rtp
                    .checked_add(
                        (current_timestamp
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64()
                            - first_rtp_sent_timestamp
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs_f64())
                        .mul(self.rtp_clockrate.to_f64().unwrap())
                        .to_u32()
                        .unwrap(),
                    )
                    .unwrap();
                builder = builder.packet(RtcpPacket::SenderReport(
                    self.generate_sender_report(rtp_timestamp, current_timestamp)?,
                ));
            }
        } else {
            builder = builder.packet(RtcpPacket::ReceiverReport(
                self.generate_receiver_report(current_timestamp)?,
            ));
        }

        builder = builder.packet(RtcpPacket::SourceDescription(self.generate_sdes()?));
        builder = builder.packets(with_packets);
        if bye {
            let mut bye_packet_builder = RtcpByePacket::builder().ssrc(self.ssrc);
            if let Some(r) = bye_reason {
                bye_packet_builder = bye_packet_builder.reason(r);
            }
            builder = builder.packet(RtcpPacket::Bye(bye_packet_builder.build()?))
        }
        builder.build().map_err(RtpSessionError::RtpFormatError)
    }
}
