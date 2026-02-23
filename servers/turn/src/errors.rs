use std::net::SocketAddr;

use rootcause::Report;
use stun_formats::{attributes::rfc8489::ErrorCodeAttribute, builder::MessageBuilder};
use thiserror::Error;
use turn_formats::error_codes;

#[derive(Debug, Error)]
pub enum TurnSessionError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported protocol: {0:?}")]
    UnsupportedProtocol(iana_formats::protocol_numbers::Protocol),
    #[error("unknown protocol: {0}")]
    UnknownProtocol(u8),
    #[error("unsupported address family: {0}")]
    UnsupportedAddressFamily(iana_formats::addrress_family::AddressFamily),
    #[error("peer address family not match: {0}")]
    PeerAddressFamilyNotMatch(iana_formats::addrress_family::AddressFamily),
    #[error("stun session error: {0}")]
    StunSessionError(#[from] stun_server::errors::StunSessionError),
    #[error("stun format error: {0}")]
    StunFormatError(#[from] stun_formats::errors::StunMessageError),
    #[error("turn format error: {0}")]
    TurnFormatError(#[from] turn_formats::errors::TurnMessageError),
    #[error("got a message that is not a request where a request is expected: {0}")]
    ExpectRequest(stun_formats::message::Message),
    #[error("got a message that is not an indication where an indication is expected: {0}")]
    ExpectIndication(stun_formats::message::Message),
    #[error("error while building Allocation: {0}")]
    BuildAllocationError(String),
    #[error("allocation mismatch from: {remote} to {local}")]
    AllocationMismatch {
        remote: SocketAddr,
        local: SocketAddr,
    },
    #[error("channel not match: channel_number: {channel_number}, peer: {peer}")]
    ChannelNotMatch {
        channel_number: u16,
        peer: SocketAddr,
    },
    #[error("no even port avaliable")]
    NoEvenPortAvaliable,
    #[error("failed to make reservation: {0}")]
    MakeReservationFailed(u16),
    #[error("invalid reservation token: {0}")]
    InvalidReservationToken(turn_formats::attributes::rfc8656::ReservationTokenAttribute),
}

pub type TurnSessionResult<T> = Result<T, Report>;

impl TurnSessionError {
    pub fn try_prepare_error_response(
        &self,
        message_builder: &mut MessageBuilder,
    ) -> TurnSessionResult<bool> {
        match self {
            Self::Io(_)
            | Self::ExpectRequest(_)
            | Self::ExpectIndication(_)
            | Self::BuildAllocationError(_) => Ok(false),
            Self::StunSessionError(stun) => stun.try_prepare_error_response(message_builder),
            Self::StunFormatError(stun) => stun.try_prepare_error_response(message_builder),
            Self::TurnFormatError(turn) => turn.try_prepare_error_response(message_builder),
            Self::UnsupportedProtocol(protocol) => {
                message_builder
                    .error_mut()
                    .attribute_mut(ErrorCodeAttribute::new_concrete(
                        error_codes::rfc8656::UNSUPPORTED_TRANSPORT_PROTOCOL::new_with_reason(
                            format!("{} is not supported", protocol),
                        ),
                    ))?;
                Ok(true)
            }
            Self::UnknownProtocol(number) => {
                message_builder
                    .error_mut()
                    .attribute_mut(ErrorCodeAttribute::new_concrete(
                        error_codes::rfc8656::UNSUPPORTED_TRANSPORT_PROTOCOL::new_with_reason(
                            format!("{} is not recognized as a protocol", number),
                        ),
                    ))?;
                Ok(true)
            }
            Self::UnsupportedAddressFamily(family) => {
                message_builder
                    .error_mut()
                    .attribute_mut(ErrorCodeAttribute::new_concrete(
                        error_codes::rfc8656::ADDRESS_FAMILY_NOT_SUPPORTED::new_with_reason(
                            format!("{} is not supported", family),
                        ),
                    ))?;
                Ok(true)
            }
            Self::PeerAddressFamilyNotMatch(family) => {
                message_builder
                    .error_mut()
                    .attribute_mut(ErrorCodeAttribute::new_concrete(
                        error_codes::rfc8656::PEER_ADDRESS_FAMILY_MISMATCH::new_with_reason(
                            format!(
                                "address family {} not match the relayed transport address",
                                family
                            ),
                        ),
                    ))?;
                Ok(true)
            }
            Self::AllocationMismatch { remote, local } => {
                message_builder
                    .error_mut()
                    .attribute_mut(ErrorCodeAttribute::new_concrete(
                        error_codes::rfc8656::ALLOCATION_MISMATCH::new_with_reason(format!(
                            "allocation from {} to {} mismatch",
                            remote, local
                        )),
                    ))?;
                Ok(true)
            }
            Self::ChannelNotMatch { .. } => {
                message_builder
                    .error_mut()
                    .attribute_mut(ErrorCodeAttribute::new_concrete(
                        stun_formats::error_codes::rfc8489::BAD_REQUEST::new_with_reason(format!(
                            "{}",
                            self
                        )),
                    ))?;
                Ok(true)
            }
            Self::NoEvenPortAvaliable => {
                message_builder
                    .error_mut()
                    .attribute_mut(ErrorCodeAttribute::new_concrete(
                        turn_formats::error_codes::rfc8656::INSUFFICIENT_CAPACITY::new_with_reason(
                            "no even port avaliable",
                        ),
                    ))?;
                Ok(true)
            }
            Self::MakeReservationFailed(port) => {
                message_builder
                    .error_mut()
                    .attribute_mut(ErrorCodeAttribute::new_concrete(
                        turn_formats::error_codes::rfc8656::INSUFFICIENT_CAPACITY::new_with_reason(
                            format!("failed to make reservation with port: {}", port),
                        ),
                    ))?;
                Ok(true)
            }
            Self::InvalidReservationToken(token) => {
                message_builder
                    .error_mut()
                    .attribute_mut(ErrorCodeAttribute::new_concrete(
                        turn_formats::error_codes::rfc8656::INSUFFICIENT_CAPACITY::new_with_reason(
                            format!("no reservation token found for: {}", token),
                        ),
                    ))?;
                Ok(true)
            }
        }
    }
}
