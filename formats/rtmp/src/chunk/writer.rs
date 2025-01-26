use byteorder::{BigEndian, LittleEndian, WriteBytesExt};

use std::{
    cmp::min,
    collections::HashMap,
    io::{Cursor, Read, Write},
};
use tokio::{io::BufWriter, net::TcpStream};
use tokio_util::{
    bytes::{Buf, BytesMut},
    either::Either,
};
use utils::system::time::get_timestamp_ms;

use crate::{
    commands::{
        CallCommandRequest, CallCommandResponse, ConnectCommandRequest, ConnectCommandResponse,
        CreateStreamCommandRequest, CreateStreamCommandResponse, DeleteStreamCommand,
        OnStatusCommand, PauseCommand, Play2Command, PlayCommand, PublishCommand,
        ReceiveAudioCommand, ReceiveVideoCommand, RtmpC2SCommands, RtmpS2CCommands, SeekCommand,
        consts::s2c_command_names::{self, ON_STATUS},
    },
    message::{RtmpMessageType, RtmpUserMessageBody},
    protocol_control::{
        AbortMessage, Acknowledgement, ProtocolControlMessage, ProtocolControlMessageType,
        SetChunkSize, SetPeerBandWidthLimitType, SetPeerBandwidth, WindowAckSize,
        consts::PROTOCOL_CONTROL_MESSAGE_STREAM_ID,
    },
    user_control::{
        UserControlEvent,
        consts::{USER_CONTROL_MESSAGE_STREAM_ID, USER_CONTROL_MESSAGE_TYPE},
    },
};

use super::{
    ChunkBasicHeader, ChunkBasicHeaderType, ChunkMessage, ChunkMessageCommonHeader,
    ChunkMessageHeader, Csid, RtmpChunkMessageBody,
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

type ChunkMessageWriteContext = HashMap<Csid, WriteContext>;

#[derive(Debug)]
pub struct Writer {
    inner: Vec<u8>,
    context: ChunkMessageWriteContext,
    chunk_size: Option<u32>,
    bytes_written: usize,
}

impl Writer {
    pub fn new() -> Self {
        Self {
            inner: Vec::with_capacity(4096),
            context: ChunkMessageWriteContext::new(),
            chunk_size: None,
            bytes_written: 0,
        }
    }

    #[inline]
    pub fn get_bytes_written(&self) -> usize {
        self.bytes_written
    }

    pub async fn write_to(&mut self, writer: &mut BufWriter<TcpStream>) -> ChunkMessageResult<()> {
        use tokio::io::AsyncWriteExt;
        writer.write_all(&self.inner).await?;
        self.inner.clear();
        Ok(())
    }

    pub fn write(
        &mut self,
        mut value: ChunkMessage,
        version: amf::Version,
    ) -> ChunkMessageResult<()> {
        // self.transparent_write(basic_header, message_header, value.chunk_data)?;
        let mut bytes = Vec::with_capacity(4096);
        match value.chunk_message_body {
            RtmpChunkMessageBody::ProtocolControl(message) => message.write_to(&mut bytes),
            RtmpChunkMessageBody::UserControl(message) => message.write_to(&mut bytes),
            RtmpChunkMessageBody::RtmpUserMessage(message) => {
                message.write_c2s_to(&mut bytes, version)
            }
        }?;

        value.header.message_length = bytes.len() as u32;
        let (basic_header, message_header) = self.justify_message_type(&value.header);

        match self.chunk_size {
            None => {
                self.write_basic_header(&basic_header)?;
                self.write_message_header(&message_header, basic_header.chunk_stream_id)?;

                self.inner.reserve(bytes.len());
                self.inner.write_all(&bytes)?;

                self.bytes_written += bytes.len()
                    + basic_header.get_header_length()
                    + message_header.get_header_length();
            }
            Some(len) => {
                self.write_basic_header(&basic_header)?;
                self.write_message_header(&message_header, basic_header.chunk_stream_id)?;

                let mut cursor_buf = Cursor::new(bytes);
                let mut tmp_buf = Vec::new();

                let bytes_to_write = min(cursor_buf.remaining(), len as usize);
                tmp_buf.resize(bytes_to_write, 0);
                cursor_buf.read_exact(&mut tmp_buf)?;

                self.inner.reserve(bytes_to_write);
                self.inner.write_all(&tmp_buf)?;

                self.bytes_written += bytes_to_write
                    + basic_header.get_header_length()
                    + message_header.get_header_length();

                while cursor_buf.has_remaining() {
                    let bytes_to_write = min(cursor_buf.remaining(), len as usize);
                    tmp_buf.clear();
                    tmp_buf.resize(bytes_to_write, 0);
                    cursor_buf.read_exact(&mut tmp_buf)?;

                    self.write_basic_header(&ChunkBasicHeader {
                        // !! important, remember to set this fmt to 3,
                        // !! otherwise the peer cannot parse the right chunk size
                        // !! this bug takes me days to debug
                        fmt: 3,
                        header_type: basic_header.header_type.clone(),
                        chunk_stream_id: basic_header.chunk_stream_id,
                    })?;
                    if value.header.timestamp >= MAX_TIMESTAMP {
                        self.inner.write_u32::<BigEndian>(value.header.timestamp)?;
                    }
                    self.inner.reserve(bytes_to_write);
                    self.inner.write_all(&tmp_buf)?;
                    self.bytes_written += bytes_to_write + basic_header.get_header_length();
                }
            }
        }
        Ok(())
    }

    pub fn write_set_chunk_size(&mut self, chunk_size: u32) -> ChunkMessageResult<()> {
        self.chunk_size = Some(chunk_size);

        self.write(
            ChunkMessage {
                header: Self::make_protocol_control_common_header(
                    4,
                    ProtocolControlMessageType::SetChunkSize,
                )?,
                chunk_message_body: RtmpChunkMessageBody::ProtocolControl(
                    ProtocolControlMessage::SetChunkSize(SetChunkSize {
                        chunk_size: chunk_size & 0x7FFF_FFFF,
                    }),
                ),
            },
            amf::Version::Amf0,
        )?;
        self.bytes_written = 0;
        Ok(())
    }

    pub fn write_abort_message(&mut self, chunk_stream_id: u32) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_protocol_control_common_header(
                    4,
                    ProtocolControlMessageType::Abort,
                )?,
                chunk_message_body: RtmpChunkMessageBody::ProtocolControl(
                    ProtocolControlMessage::Abort(AbortMessage { chunk_stream_id }),
                ),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_acknowledgement_message(
        &mut self,
        sequence_number: u32,
    ) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_protocol_control_common_header(
                    4,
                    ProtocolControlMessageType::Acknowledgement,
                )?,
                chunk_message_body: RtmpChunkMessageBody::ProtocolControl(
                    ProtocolControlMessage::Ack(Acknowledgement { sequence_number }),
                ),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_window_ack_size_message(
        &mut self,
        window_ack_size: u32,
    ) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_protocol_control_common_header(
                    4,
                    ProtocolControlMessageType::WindowAckSize,
                )?,
                chunk_message_body: RtmpChunkMessageBody::ProtocolControl(
                    ProtocolControlMessage::WindowAckSize(WindowAckSize {
                        size: window_ack_size,
                    }),
                ),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_set_peer_bandwidth(
        &mut self,
        ack_window_size: u32,
        limit_type: SetPeerBandWidthLimitType,
    ) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
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
            },
            amf::Version::Amf0,
        )
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
            extended_timestamp_enabled: false,

            // we do not need this to write
            runtime_stat: Default::default(),
        })
    }

    pub fn write_stream_begin(&mut self, stream_id: u32) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_user_control_common_header(6)?,
                chunk_message_body: RtmpChunkMessageBody::UserControl(
                    UserControlEvent::StreamBegin { stream_id },
                ),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_stream_eof(&mut self, stream_id: u32) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_user_control_common_header(6)?,
                chunk_message_body: RtmpChunkMessageBody::UserControl(
                    UserControlEvent::StreamEOF { stream_id },
                ),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_stream_dry(&mut self, stream_id: u32) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_user_control_common_header(6)?,
                chunk_message_body: RtmpChunkMessageBody::UserControl(
                    UserControlEvent::StreamDry { stream_id },
                ),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_set_buffer_length(
        &mut self,
        stream_id: u32,
        buffer_length: u32,
    ) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_user_control_common_header(10)?,
                chunk_message_body: RtmpChunkMessageBody::UserControl(
                    UserControlEvent::SetBufferLength {
                        stream_id,
                        buffer_length,
                    },
                ),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_stream_ids_recorded(&mut self, stream_id: u32) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_user_control_common_header(6)?,
                chunk_message_body: RtmpChunkMessageBody::UserControl(
                    UserControlEvent::StreamIdsRecorded { stream_id },
                ),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_ping_request(&mut self, timestamp: u32) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_user_control_common_header(6)?,
                chunk_message_body: RtmpChunkMessageBody::UserControl(
                    UserControlEvent::PingRequest { timestamp },
                ),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_ping_response(&mut self, timestamp: u32) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_user_control_common_header(6)?,
                chunk_message_body: RtmpChunkMessageBody::UserControl(
                    UserControlEvent::PingResponse { timestamp },
                ),
            },
            amf::Version::Amf0,
        )
    }

    fn make_user_control_common_header(size: u32) -> ChunkMessageResult<ChunkMessageCommonHeader> {
        Ok(ChunkMessageCommonHeader {
            basic_header: ChunkBasicHeader::new(0, csid::USER_CONTROL.into())?,
            timestamp: 0,
            message_length: size,
            message_type_id: USER_CONTROL_MESSAGE_TYPE,
            message_stream_id: USER_CONTROL_MESSAGE_STREAM_ID.into(),
            extended_timestamp_enabled: false,

            // we do not need this to write
            runtime_stat: Default::default(),
        })
    }

    pub fn write_connect_request(
        &mut self,
        message: ConnectCommandRequest,
    ) -> ChunkMessageResult<()> {
        let version = message.command_object.object_encoding;
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Connect(message)),
                )),
            },
            version,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write_connect_response(
        &mut self,
        success: bool,
        transaction_id: f64,
        fmsver: &str,
        capabilities: f64,
        code: &str,
        level: &str,
        description: &str,
        encoding: amf::Version,
    ) -> ChunkMessageResult<()> {
        let mut properties = HashMap::new();
        properties.insert("fmsVer".into(), amf::string(fmsver, encoding));
        properties.insert("capabilities".into(), amf::number(capabilities, encoding));

        let mut information = HashMap::new();
        information.insert("level".into(), amf::string(level, encoding));
        information.insert("code".into(), amf::string(code, encoding));
        information.insert("description".into(), amf::string(description, encoding));
        information.insert(
            "objectEncoding".into(),
            amf::number(encoding as u8, encoding),
        );
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::S2Command(RtmpS2CCommands::Connect(
                        ConnectCommandResponse {
                            success,
                            transaction_id: transaction_id as u8,
                            properties: Some(properties),
                            information: Some(Either::Right(information)),
                        },
                    )),
                )),
            },
            encoding,
        )
    }

    pub fn write_call_request(&mut self, message: CallCommandRequest) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Call(message)),
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_call_response(
        &mut self,
        success: bool,
        transaction_id: f64,
        command_object: Option<HashMap<String, amf::Value>>,
        response: Option<HashMap<String, amf::Value>>,
    ) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::S2Command(RtmpS2CCommands::Call(CallCommandResponse {
                        command_name: if success {
                            s2c_command_names::RESULT.to_string()
                        } else {
                            s2c_command_names::ERROR.to_string()
                        },
                        transaction_id,
                        command_object,
                        response,
                    })),
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_create_stream_request(
        &mut self,
        message: CreateStreamCommandRequest,
    ) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::CreateStream(message)),
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_create_stream_response(
        &mut self,
        success: bool,
        transaction_id: f64,
        command_object: Option<HashMap<String, amf::Value>>,
        stream_id: f64,
    ) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::S2Command(RtmpS2CCommands::CreateStream(
                        CreateStreamCommandResponse {
                            success,
                            transaction_id,
                            command_object,
                            stream_id,
                        },
                    )),
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_play_request(&mut self, message: PlayCommand) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Play(message)),
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_play2_request(&mut self, message: Play2Command) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Play2(message)),
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_delete_stream_request(
        &mut self,
        message: DeleteStreamCommand,
    ) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::DeleteStream(message)),
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_receive_audio_request(
        &mut self,
        message: ReceiveAudioCommand,
    ) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::ReceiveAudio(message)),
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_receive_video_request(
        &mut self,
        message: ReceiveVideoCommand,
    ) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::ReceiveVideo(message)),
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_publish_request(&mut self, message: PublishCommand) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Publish(message)),
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_seek_request(&mut self, message: SeekCommand) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Seek(message)),
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_pause_request(&mut self, message: PauseCommand) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::C2SCommand(RtmpC2SCommands::Pause(message)),
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_on_status_response(
        &mut self,
        level: &str,
        code: &str,
        description: &str,
        encoding: amf::Version,
        additional: Option<HashMap<String, amf::Value>>,
    ) -> ChunkMessageResult<()> {
        let mut info_object = HashMap::new();
        info_object.insert("level".into(), amf::string(level, encoding));
        info_object.insert("code".into(), amf::string(code, encoding));
        info_object.insert("description".into(), amf::string(description, encoding));
        if let Some(additional) = additional {
            info_object.extend(additional);
        }
        self.write(
            ChunkMessage {
                header: Self::make_command_common_header()?,
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::S2Command(RtmpS2CCommands::OnStatus(OnStatusCommand {
                        command_name: ON_STATUS.into(),
                        transaction_id: 0,
                        info_object,
                    })),
                )),
            },
            amf::Version::Amf0,
        )
    }

    fn make_command_common_header() -> ChunkMessageResult<ChunkMessageCommonHeader> {
        let timestamp = get_timestamp_ms()? as u32;
        Ok(ChunkMessageCommonHeader {
            basic_header: ChunkBasicHeader::new(0, csid::NET_CONNECTION_COMMAND.into())?,
            timestamp,
            message_length: 0, //NOTE - length will be justified later
            message_type_id: RtmpMessageType::AMF0Command.into(),
            message_stream_id: 0, //TODO - check this out
            extended_timestamp_enabled: timestamp >= MAX_TIMESTAMP,
            // we do not need this to write
            runtime_stat: Default::default(),
        })
    }

    pub fn write_meta(&mut self, meta: BytesMut, timestamp: u32) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: ChunkMessageCommonHeader {
                    basic_header: ChunkBasicHeader::new(0, csid::NET_CONNECTION_COMMAND2.into())?,
                    timestamp,
                    message_length: 0,
                    message_type_id: RtmpMessageType::AMF0Data.into(),
                    message_stream_id: 0,
                    extended_timestamp_enabled: timestamp >= MAX_TIMESTAMP,
                    // we do not need this to write
                    runtime_stat: Default::default(),
                },
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::MetaData { payload: meta },
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_audio(&mut self, message: BytesMut, timestamp: u32) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: ChunkMessageCommonHeader {
                    basic_header: ChunkBasicHeader::new(0, csid::AUDIO.into())?,
                    timestamp,
                    message_length: 0,
                    message_type_id: RtmpMessageType::Audio.into(),
                    message_stream_id: 0,
                    extended_timestamp_enabled: timestamp >= MAX_TIMESTAMP,
                    // we do not need this to write
                    runtime_stat: Default::default(),
                },
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::Audio { payload: message },
                )),
            },
            amf::Version::Amf0,
        )
    }

    pub fn write_video(&mut self, message: BytesMut, timestamp: u32) -> ChunkMessageResult<()> {
        self.write(
            ChunkMessage {
                header: ChunkMessageCommonHeader {
                    basic_header: ChunkBasicHeader::new(0, csid::VIDEO.into())?,
                    timestamp,
                    message_length: 0,
                    message_type_id: RtmpMessageType::Video.into(),
                    message_stream_id: 0,
                    extended_timestamp_enabled: timestamp >= MAX_TIMESTAMP,
                    // we do not need this to write
                    runtime_stat: Default::default(),
                },
                chunk_message_body: RtmpChunkMessageBody::RtmpUserMessage(Box::new(
                    RtmpUserMessageBody::Video { payload: message },
                )),
            },
            amf::Version::Amf0,
        )
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

        let ctx = self
            .context
            .get_mut(&basic_header.chunk_stream_id)
            .unwrap_or_else(|| {
                panic!(
                    "there should be {} in context map",
                    basic_header.chunk_stream_id
                )
            });

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
            (
                basic_header,
                ChunkMessageHeader::Type3(super::ChunkMessageHeaderType3 {}),
            )
        } else if ctx.message_length == value.message_length
            && ctx.message_stream_id == value.message_stream_id
            && ctx.message_type_id == value.message_type_id
        {
            (
                basic_header,
                ChunkMessageHeader::Type2(super::ChunkMessageHeaderType2 {
                    timestamp_delta: value.timestamp - ctx.timestamp,
                }),
            )
        } else if ctx.message_stream_id == value.message_stream_id {
            (
                basic_header,
                ChunkMessageHeader::Type1(super::ChunkMessageHeaderType1 {
                    timestamp_delta: value.timestamp - ctx.timestamp,
                    message_length: value.message_length,
                    message_type_id: value.message_type_id,
                }),
            )
        } else {
            (
                basic_header,
                ChunkMessageHeader::Type0(super::ChunkMessageHeaderType0 {
                    timestamp: value.timestamp,
                    message_length: value.message_length,
                    message_type_id: value.message_type_id,
                    message_stream_id: value.message_stream_id,
                }),
            )
        }
    }

    fn write_basic_header(&mut self, header: &ChunkBasicHeader) -> ChunkMessageResult<()> {
        self.inner.reserve(20);
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
        csid: Csid,
    ) -> ChunkMessageResult<()> {
        self.inner.reserve(20);
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

                // insert default if not exist, very rusty way
                self.context.entry(csid).or_default();

                let ctx = self
                    .context
                    .get_mut(&csid)
                    .unwrap_or_else(|| panic!("there should be {} in context map", csid));

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
                    .unwrap_or_else(|| panic!("there should be {} in context map", csid));

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
                    .unwrap_or_else(|| panic!("there should be {} in context map", csid));

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

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}
