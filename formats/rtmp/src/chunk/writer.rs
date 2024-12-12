use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use std::{
    borrow::BorrowMut,
    collections::HashMap,
    fmt::Debug,
    io::{self},
};
use tokio_util::bytes::{BufMut, BytesMut};
use tracing::instrument;
use utils::system::util::get_timestamp_ms;

use crate::{
    commands::{
        CallCommandRequest, CallCommandResponse, ConnectCommandRequest, ConnectCommandResponse,
        CreateStreamCommandRequest, CreateStreamCommandResponse, DeleteStreamCommand,
        OnStatusCommand, PauseCommand, Play2Command, PlayCommand, PublishCommand,
        ReceiveAudioCommand, ReceiveVideoCommand, RtmpC2SCommands, RtmpS2CCommands, SeekCommand,
    },
    message::{RtmpMessageType, RtmpUserMessageBody},
    protocol_control::{
        AbortMessage, Acknowledgement, ProtocolControlMessage, ProtocolControlMessageType,
        SetChunkSize, SetPeerBandWidthLimitType, SetPeerBandwidth, WindowAckSize,
        consts::PROTOCOL_CONTROL_MESSAGE_STREAM_ID,
    },
    user_control::{
        UserControlEvent, UserControlEventType,
        consts::{USER_CONTROL_MESSAGE_STREAM_ID, USER_CONTROL_MESSAGE_TYPE},
    },
};

use super::{
    CSID, ChunkBasicHeader, ChunkBasicHeaderType, ChunkMessage, ChunkMessageCommonHeader,
    ChunkMessageHeader, ChunkMessageType, RtmpChunkMessageBody,
    consts::{MAX_TIMESTAMP, csid},
    errors::ChunkMessageResult,
};

#[derive(Debug, Default)]
struct WriteContext {
    timestamp: u32,
    timestamp_delta: u32,
    extended_timestamp_enabled: bool,
    message_length: u32,
    message_stream_id: u32,
    message_type_id: u8,
    previous_message_header: Option<ChunkMessageHeader>,
}

type ChunkMessageWriteContext = HashMap<CSID, WriteContext>;

#[derive(Debug)]
pub struct Writer<W> {
    inner: W,
    context: ChunkMessageWriteContext,
}

impl<W> Writer<W>
where
    W: io::Write + Debug,
{
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            context: ChunkMessageWriteContext::new(),
        }
    }

    pub fn write(&mut self, mut value: ChunkMessage) -> ChunkMessageResult<()> {
        // self.transparent_write(basic_header, message_header, value.chunk_data)?;
        let bytes = BytesMut::with_capacity(4096);
        match value.chunk_message_body {
            RtmpChunkMessageBody::ProtocolControl(message) => {
                message.write_to(bytes.clone().writer())
            }
            RtmpChunkMessageBody::UserControl(message) => message.write_to(bytes.clone().writer()),
            RtmpChunkMessageBody::RtmpUserMessage(message) => {
                message.write_c2s_to(bytes.clone().writer(), amf::Version::Amf0)
            }
        }?;

        value.header.message_length = bytes.len() as u32;
        let (basic_header, message_header) = self.justify_message_type(&value.header);
        self.write_basic_header(&basic_header)?;
        self.write_message_header(&message_header, basic_header.chunk_stream_id)?;
        self.inner.write_all(&bytes)?;

        Ok(())
    }

    pub fn write_set_chunk_size(&mut self, chunk_size: u32) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_protocol_control_common_header(
                4,
                ProtocolControlMessageType::SetChunkSize,
            )?,
            chunk_message_body: RtmpChunkMessageBody::ProtocolControl(
                ProtocolControlMessage::SetChunkSize(SetChunkSize {
                    chunk_size: chunk_size & 0x7FFFFFFF,
                }),
            ),
        })
    }

    pub fn write_abort_message(&mut self, chunk_stream_id: u32) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_protocol_control_common_header(
                4,
                ProtocolControlMessageType::Abort,
            )?,
            chunk_message_body: RtmpChunkMessageBody::ProtocolControl(
                ProtocolControlMessage::Abort(AbortMessage { chunk_stream_id }),
            ),
        })
    }

    pub fn write_acknowledgement_message(
        &mut self,
        sequence_number: u32,
    ) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_protocol_control_common_header(
                4,
                ProtocolControlMessageType::Acknowledgement,
            )?,
            chunk_message_body: RtmpChunkMessageBody::ProtocolControl(ProtocolControlMessage::Ack(
                Acknowledgement { sequence_number },
            )),
        })
    }

    pub fn write_window_ack_size_message(
        &mut self,
        window_ack_size: u32,
    ) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_protocol_control_common_header(
                4,
                ProtocolControlMessageType::WindowAckSize,
            )?,
            chunk_message_body: RtmpChunkMessageBody::ProtocolControl(
                ProtocolControlMessage::WindowAckSize(WindowAckSize {
                    size: window_ack_size,
                }),
            ),
        })
    }

    pub fn write_set_peer_bandwidth(
        &mut self,
        ack_window_size: u32,
        limit_type: SetPeerBandWidthLimitType,
    ) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_protocol_control_common_header(
                5,
                ProtocolControlMessageType::SetPeerBandwidth,
            )?,
            chunk_message_body: RtmpChunkMessageBody::ProtocolControl(
                ProtocolControlMessage::SetPeerBandwidth(SetPeerBandwidth {
                    size: ack_window_size,
                    limit_type,
                }),
            ),
        })
    }

    fn make_protocol_control_common_header(
        size: u32,
        message_type: ProtocolControlMessageType,
    ) -> ChunkMessageResult<ChunkMessageCommonHeader> {
        Ok(ChunkMessageCommonHeader {
            basic_header: ChunkBasicHeader::new(0, csid::PROTOCOL_CONTROL.into())?,
            timestamp: 0,
            message_length: size,
            message_type_id: message_type.into(),
            message_stream_id: PROTOCOL_CONTROL_MESSAGE_STREAM_ID.into(),
        })
    }

    pub fn write_stream_begin(&mut self, stream_id: u32) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_user_control_common_header(6)?,
            chunk_message_body: RtmpChunkMessageBody::UserControl(UserControlEvent::StreamBegin {
                stream_id,
            }),
        })
    }

    pub fn write_stream_eof(&mut self, stream_id: u32) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_user_control_common_header(6)?,
            chunk_message_body: RtmpChunkMessageBody::UserControl(UserControlEvent::StreamEOF {
                stream_id,
            }),
        })
    }

    pub fn write_stream_dry(&mut self, stream_id: u32) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_user_control_common_header(6)?,
            chunk_message_body: RtmpChunkMessageBody::UserControl(UserControlEvent::StreamDry {
                stream_id,
            }),
        })
    }

    pub fn write_set_buffer_length(
        &mut self,
        stream_id: u32,
        buffer_length: u32,
    ) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_user_control_common_header(10)?,
            chunk_message_body: RtmpChunkMessageBody::UserControl(
                UserControlEvent::SetBufferLength {
                    stream_id,
                    buffer_length,
                },
            ),
        })
    }

    pub fn write_stream_ids_recorded(&mut self, stream_id: u32) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_user_control_common_header(6)?,
            chunk_message_body: RtmpChunkMessageBody::UserControl(
                UserControlEvent::StreamIdsRecorded { stream_id },
            ),
        })
    }

    pub fn write_ping_request(&mut self, timestamp: u32) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_user_control_common_header(6)?,
            chunk_message_body: RtmpChunkMessageBody::UserControl(UserControlEvent::PingRequest {
                timestamp,
            }),
        })
    }

    pub fn write_ping_response(&mut self, timestamp: u32) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_user_control_common_header(6)?,
            chunk_message_body: RtmpChunkMessageBody::UserControl(UserControlEvent::PingResponse {
                timestamp,
            }),
        })
    }

    fn make_user_control_common_header(size: u32) -> ChunkMessageResult<ChunkMessageCommonHeader> {
        Ok(ChunkMessageCommonHeader {
            basic_header: ChunkBasicHeader::new(0, csid::USER_CONTROL.into())?,
            timestamp: 0,
            message_length: size,
            message_type_id: USER_CONTROL_MESSAGE_TYPE,
            message_stream_id: USER_CONTROL_MESSAGE_STREAM_ID.into(),
        })
    }

    pub fn write_connect_request(
        &mut self,
        message: ConnectCommandRequest,
    ) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Connect(message)),
            ),
        })
    }

    pub fn write_connect_response(
        &mut self,
        message: ConnectCommandResponse,
    ) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::S2Command(RtmpS2CCommands::Connect(message)),
            ),
        })
    }

    pub fn write_call_request(&mut self, message: CallCommandRequest) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Call(message)),
            ),
        })
    }

    pub fn write_call_response(&mut self, message: CallCommandResponse) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::S2Command(RtmpS2CCommands::Call(message)),
            ),
        })
    }

    pub fn write_create_stream_request(
        &mut self,
        message: CreateStreamCommandRequest,
    ) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::CreateStream(message)),
            ),
        })
    }

    pub fn write_create_stream_response(
        &mut self,
        message: CreateStreamCommandResponse,
    ) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::S2Command(RtmpS2CCommands::CreateStream(message)),
            ),
        })
    }

    pub fn write_play_request(&mut self, message: PlayCommand) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Play(message)),
            ),
        })
    }

    pub fn write_play2_request(&mut self, message: Play2Command) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Play2(message)),
            ),
        })
    }

    pub fn write_delete_stream_request(
        &mut self,
        message: DeleteStreamCommand,
    ) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::DeleteStream(message)),
            ),
        })
    }

    pub fn write_receive_audio_request(
        &mut self,
        message: ReceiveAudioCommand,
    ) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::ReceiveAudio(message)),
            ),
        })
    }

    pub fn write_receive_video_request(
        &mut self,
        message: ReceiveVideoCommand,
    ) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::ReceiveVideo(message)),
            ),
        })
    }

    pub fn write_publish_request(&mut self, message: PublishCommand) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Publish(message)),
            ),
        })
    }

    pub fn write_seek_request(&mut self, message: SeekCommand) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Seek(message)),
            ),
        })
    }

    pub fn write_pause_request(&mut self, message: PauseCommand) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Pause(message)),
            ),
        })
    }

    pub fn write_on_status_response(&mut self, message: OnStatusCommand) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: Self::make_command_common_header()?,
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::S2Command(RtmpS2CCommands::OnStatus(message)),
            ),
        })
    }

    fn make_command_common_header() -> ChunkMessageResult<ChunkMessageCommonHeader> {
        Ok(ChunkMessageCommonHeader {
            basic_header: ChunkBasicHeader::new(0, csid::NET_CONNECTION_COMMAND.into())?,
            timestamp: get_timestamp_ms()? as u32,
            message_length: 0, //NOTE - length will be justified later
            message_type_id: RtmpMessageType::AMF0Command.into(),
            message_stream_id: 0, //TODO - check this out
        })
    }

    pub fn write_meta(&mut self, meta: amf::Value) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: ChunkMessageCommonHeader {
                basic_header: ChunkBasicHeader::new(0, csid::NET_CONNECTION_COMMAND2.into())?,
                timestamp: get_timestamp_ms()? as u32,
                message_length: 0,
                message_type_id: match meta {
                    amf::Value::AMF0Value(_) => RtmpMessageType::AMF0Data.into(),
                    amf::Value::AMF3Value(_) => RtmpMessageType::AMF3Data.into(),
                },
                message_stream_id: 0,
            },
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(
                RtmpUserMessageBody::MetaData(meta),
            ),
        })
    }

    pub fn write_audio(&mut self, message: BytesMut) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: ChunkMessageCommonHeader {
                basic_header: ChunkBasicHeader::new(0, csid::AUDIO.into())?,
                timestamp: get_timestamp_ms()? as u32,
                message_length: 0,
                message_type_id: RtmpMessageType::Audio.into(),
                message_stream_id: 0,
            },
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(RtmpUserMessageBody::Audio {
                payload: message,
            }),
        })
    }

    pub fn write_video(&mut self, message: BytesMut) -> ChunkMessageResult<()> {
        self.write(ChunkMessage {
            header: ChunkMessageCommonHeader {
                basic_header: ChunkBasicHeader::new(0, csid::VIDEO.into())?,
                timestamp: get_timestamp_ms()? as u32,
                message_length: 0,
                message_type_id: RtmpMessageType::Video.into(),
                message_stream_id: 0,
            },
            chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(RtmpUserMessageBody::Video {
                payload: message,
            }),
        })
    }

    fn justify_message_type(
        &mut self,
        value: &ChunkMessageCommonHeader,
    ) -> (ChunkBasicHeader, ChunkMessageHeader) {
        let basic_header = value.basic_header.clone();

        // no context at all, this must be the first message of this chunk stream
        if !self.context.contains_key(&basic_header.chunk_stream_id) {
            return (
                basic_header,
                ChunkMessageHeader::Type0(super::ChunkMessageHeaderType0 {
                    timestamp: value.timestamp,
                    message_length: value.message_length,
                    message_type_id: value.message_type_id,
                    message_stream_id: value.message_stream_id,
                }),
            );
        }

        let ctx = self.context.get_mut(&basic_header.chunk_stream_id).expect(
            format!(
                "there should be {} in context map",
                basic_header.chunk_stream_id
            )
            .as_str(),
        );

        // maybe something weird happened, but we don't know what type the last message header is
        if ctx.previous_message_header.is_none() {
            return (
                basic_header,
                ChunkMessageHeader::Type0(super::ChunkMessageHeaderType0 {
                    timestamp: value.timestamp,
                    message_length: value.message_length,
                    message_type_id: value.message_type_id,
                    message_stream_id: value.message_stream_id,
                }),
            );
        }

        let previous_message_header = ctx
            .previous_message_header
            .as_ref()
            .expect("this must be valid");

        if let ChunkMessageHeader::Type0(_) = previous_message_header {
            // see: 5.3.1.2.4. Type 3
            if value.timestamp == ctx.timestamp * 2 {
                // make sure the timestamp_delta is correct when writing Type3 header
                ctx.timestamp_delta = ctx.timestamp;
                return (
                    basic_header,
                    ChunkMessageHeader::Type3(super::ChunkMessageHeaderType3 {}),
                );
            }
        }

        if ctx.message_length == value.message_length
            && ctx.message_stream_id == value.message_stream_id
            && ctx.message_type_id == value.message_type_id
            && ctx.timestamp_delta == value.timestamp - ctx.timestamp
        {
            return (
                basic_header,
                ChunkMessageHeader::Type3(super::ChunkMessageHeaderType3 {}),
            );
        } else if ctx.message_length == value.message_length
            && ctx.message_stream_id == value.message_stream_id
            && ctx.message_type_id == value.message_type_id
        {
            return (
                basic_header,
                ChunkMessageHeader::Type2(super::ChunkMessageHeaderType2 {
                    timestamp_delta: value.timestamp - ctx.timestamp,
                }),
            );
        } else if ctx.message_stream_id == value.message_stream_id {
            return (
                basic_header,
                ChunkMessageHeader::Type1(super::ChunkMessageHeaderType1 {
                    timestamp_delta: value.timestamp - ctx.timestamp,
                    message_length: value.message_length,
                    message_type_id: value.message_type_id,
                }),
            );
        } else {
            return (
                basic_header,
                ChunkMessageHeader::Type0(super::ChunkMessageHeaderType0 {
                    timestamp: value.timestamp,
                    message_length: value.message_length,
                    message_type_id: value.message_type_id,
                    message_stream_id: value.message_stream_id,
                }),
            );
        }
    }

    fn write_basic_header(&mut self, header: &ChunkBasicHeader) -> ChunkMessageResult<()> {
        match header.header_type {
            ChunkBasicHeaderType::Type1 => {
                let first_byte = (header.fmt << 6) + header.chunk_stream_id as u8;
                self.inner.write_u8(first_byte)?;
            }
            ChunkBasicHeaderType::Type2 => {
                let first_byte = header.fmt << 6;
                self.inner.write_u8(first_byte)?;
                self.inner.write_u8((header.chunk_stream_id - 64) as u8)?;
            }
            ChunkBasicHeaderType::Type3 => {
                let first_byte = header.fmt << 6 | 0b00111111;
                self.inner.write_u8(first_byte)?;
                self.inner
                    .write_u16::<BigEndian>((header.chunk_stream_id - 64) as u16)?;
            }
        }
        Ok(())
    }

    fn write_message_header(
        &mut self,
        header: &ChunkMessageHeader,
        csid: CSID,
    ) -> ChunkMessageResult<()> {
        match header {
            ChunkMessageHeader::Type0(header) => {
                let extended_timestamp_enabled = header.timestamp >= MAX_TIMESTAMP;
                let timestamp_field = header.timestamp.min(MAX_TIMESTAMP);
                self.inner.write_u24::<BigEndian>(timestamp_field)?;
                self.inner.write_u24::<BigEndian>(header.message_length)?;
                self.inner.write_u8(header.message_type_id)?;
                self.inner
                    .write_u32::<LittleEndian>(header.message_stream_id)?;
                if extended_timestamp_enabled {
                    self.inner.write_u32::<BigEndian>(header.timestamp)?;
                }

                if !self.context.contains_key(&csid) {
                    self.context.insert(csid, WriteContext::default());
                }
                let ctx = self
                    .context
                    .get_mut(&csid)
                    .expect(format!("there should be {} in context map", csid).as_str());

                ctx.extended_timestamp_enabled = extended_timestamp_enabled;
                ctx.timestamp = header.timestamp;
                ctx.message_length = header.message_length;
                ctx.message_stream_id = header.message_stream_id;
                ctx.message_type_id = header.message_type_id;
                ctx.timestamp_delta = 0;
            }
            ChunkMessageHeader::Type1(header) => {
                if !self.context.contains_key(&csid) {
                    return Err(super::errors::ChunkMessageError::InvalidMessageHead(
                        format!(
                            "invalid message header, got a type 1 header: {:?} while no context found for csid: {}",
                            header, csid
                        ),
                    ));
                }

                let extended_timestamp_enabled = header.timestamp_delta >= MAX_TIMESTAMP;
                let timestamp_delta_field = header.timestamp_delta.min(MAX_TIMESTAMP);
                self.inner.write_u24::<BigEndian>(timestamp_delta_field)?;
                self.inner.write_u24::<BigEndian>(header.message_length)?;
                self.inner.write_u8(header.message_type_id)?;
                if extended_timestamp_enabled {
                    self.inner.write_u32::<BigEndian>(header.timestamp_delta)?;
                }

                let ctx = self
                    .context
                    .get_mut(&csid)
                    .expect(format!("there should be {} in context map", csid).as_str());

                ctx.extended_timestamp_enabled = extended_timestamp_enabled;
                ctx.timestamp_delta = header.timestamp_delta;
                ctx.timestamp += header.timestamp_delta;
                ctx.message_length = header.message_length;
                ctx.message_type_id = header.message_type_id;
            }
            ChunkMessageHeader::Type2(header) => {
                if !self.context.contains_key(&csid) {
                    return Err(super::errors::ChunkMessageError::InvalidMessageHead(
                        format!(
                            "invalid message header, got a type 2 header: {:?} while no context found for csid: {}",
                            header, csid
                        ),
                    ));
                }

                let extended_timestamp_enabled = header.timestamp_delta >= MAX_TIMESTAMP;
                let timestamp_delta_field = header.timestamp_delta.min(MAX_TIMESTAMP);
                self.inner.write_u24::<BigEndian>(timestamp_delta_field)?;
                if extended_timestamp_enabled {
                    self.inner.write_u32::<BigEndian>(header.timestamp_delta)?;
                }

                let ctx = self
                    .context
                    .get_mut(&csid)
                    .expect(format!("there should be {} in context map", csid).as_str());

                ctx.extended_timestamp_enabled = extended_timestamp_enabled;
                ctx.timestamp_delta = header.timestamp_delta;
                ctx.timestamp += header.timestamp_delta;
            }
            ChunkMessageHeader::Type3(header) => {
                let ctx = self.context.get(&csid);
                if let Some(ctx) = ctx {
                    if ctx.extended_timestamp_enabled {
                        self.inner.write_u32::<BigEndian>(ctx.timestamp_delta)?;
                    }
                } else {
                    return Err(super::errors::ChunkMessageError::InvalidMessageHead(
                        format!(
                            "invalid message header, got a type 3 header: {:?} while no context found for csid: {}",
                            header, csid
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}
