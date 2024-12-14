use std::io::{self, Cursor};

use rtmp_formats::{
    chunk::{self, ChunkMessage, RtmpChunkMessageBody, errors::ChunkMessageError},
    commands::{
        ConnectCommandRequest, ConnectCommandResponse, CreateStreamCommandRequest, PublishCommand,
        RtmpC2SCommands,
        consts::{AMF0_ENCODING, RESPONSE_STREAM_ID},
    },
    handshake,
    message::RtmpUserMessageBody,
    protocol_control::{ProtocolControlMessage, SetChunkSize, SetPeerBandWidthLimitType},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
};
use tokio_util::bytes::{Buf, BytesMut};
use tracing::instrument;

use crate::publish::errors::RtmpPublishServerError;

use super::errors::RtmpPublishServerResult;

#[derive(Debug)]
pub struct RtmpPublishSession {
    read_buffer: BytesMut,
    stream: BufWriter<TcpStream>,
    chunk_reader: chunk::reader::Reader,
    chunk_writer: chunk::writer::Writer,
}

impl RtmpPublishSession {
    pub fn new(io: TcpStream) -> Self {
        Self {
            read_buffer: BytesMut::with_capacity(4096),
            stream: BufWriter::new(io),
            chunk_reader: chunk::reader::Reader::new(),
            chunk_writer: chunk::writer::Writer::new(),
        }
    }

    async fn read_chunk(&mut self) -> RtmpPublishServerResult<Option<ChunkMessage>> {
        loop {
            let mut buf = Cursor::new(&self.read_buffer[..]);
            match self.chunk_reader.read(&mut buf, true) {
                Ok(Some(chunk_message)) => {
                    self.read_buffer.advance(buf.position() as usize);
                    return Ok(Some(chunk_message));
                }
                Ok(None) => {}
                Err(ChunkMessageError::IncompleteChunk) => {
                    self.read_buffer.advance(buf.position() as usize);
                    return Ok(None);
                }
                Err(err) => return Err(err.into()),
            }
            if 0 == self.stream.read_buf(&mut self.read_buffer).await? {
                if self.read_buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(RtmpPublishServerError::Io(io::Error::new(
                        io::ErrorKind::ConnectionReset,
                        "connect reset by peer",
                    )));
                }
            }
        }
    }

    async fn flush_chunk(&mut self) -> RtmpPublishServerResult<()> {
        self.chunk_writer.write_to(&mut self.stream).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn run(&mut self) -> RtmpPublishServerResult<()> {
        handshake::server::HandshakeServer::new(&mut self.stream)
            .handshake(false)
            .await?;
        self.chunk_writer.write_set_chunk_size(4096)?;

        loop {
            match self.read_chunk().await {
                Ok(maybe_chunk) => match maybe_chunk {
                    Some(message) => {
                        tracing::info!("got message: {:?}", message);
                        self.process_message(message).await?;
                    }
                    None => continue,
                },
                Err(err) => match err {
                    RtmpPublishServerError::ChunkMessageReadFailed(
                        ChunkMessageError::UnknownMessageType { type_id, backtrace },
                    ) => {}
                    RtmpPublishServerError::Io(io_err) => {
                        if io_err.kind() == io::ErrorKind::ConnectionReset {
                            return Ok(());
                        }
                        tracing::error!("io error: {:?}", io_err);
                    }
                    err => {
                        tracing::error!("{:?}", err);
                    }
                },
            }
        }
    }

    async fn process_message(&mut self, message: ChunkMessage) -> RtmpPublishServerResult<()> {
        let header = message.header;
        let body = message.chunk_message_body;
        match body {
            RtmpChunkMessageBody::ProtocolControl(request) => {
                self.process_protocol_control_message(request).await?
            }
            RtmpChunkMessageBody::UserControl(control) => {
                todo!()
            }
            RtmpChunkMessageBody::RtmpUserMessage(message) => {
                self.process_user_message(message).await?
            }
        }
        Ok(())
    }

    async fn process_user_message(
        &mut self,
        message: RtmpUserMessageBody,
    ) -> RtmpPublishServerResult<()> {
        match message {
            RtmpUserMessageBody::C2SCommand(command) => self.process_user_command(command).await?,
            RtmpUserMessageBody::MetaData(meta) => {}
            RtmpUserMessageBody::Aggregate { payload } => {}
            RtmpUserMessageBody::Audio { payload } => {}
            RtmpUserMessageBody::Video { payload } => {}
            RtmpUserMessageBody::S2Command(_) => {
                // ignore this
            }
            RtmpUserMessageBody::SharedObject() => {
                // ignore this
            }
        }
        Ok(())
    }

    async fn process_user_command(
        &mut self,
        command: RtmpC2SCommands,
    ) -> RtmpPublishServerResult<()> {
        match command {
            RtmpC2SCommands::Connect(request) => {
                self.process_connect_command(request).await?;
            }
            RtmpC2SCommands::Call(request) => {}
            RtmpC2SCommands::CreateStream(request) => {
                self.process_create_stream_command(request).await?
            }
            RtmpC2SCommands::DeleteStream(request) => {}
            RtmpC2SCommands::Pause(request) => {}
            RtmpC2SCommands::Play(request) => {}
            RtmpC2SCommands::Play2(request) => {}
            RtmpC2SCommands::Publish(request) => {
                self.process_publish_command(request).await?;
            }
            RtmpC2SCommands::ReceiveAudio(request) => {}
            RtmpC2SCommands::ReceiveVideo(request) => {}
            RtmpC2SCommands::Seek(request) => {}
        }
        Ok(())
    }

    #[instrument]
    async fn process_connect_command(
        &mut self,
        request: ConnectCommandRequest,
    ) -> RtmpPublishServerResult<()> {
        self.chunk_writer.write_window_ack_size_message(4096)?;
        self.chunk_writer
            .write_set_peer_bandwidth(4096, SetPeerBandWidthLimitType::Dynamic)?;
        self.flush_chunk().await?;

        self.chunk_writer.write_connect_response(
            true,
            request.transaction_id.into(),
            super::consts::FMSVER,
            super::consts::FMS_CAPABILITIES,
            super::consts::response_code::CONNECT_SUCCESS,
            super::consts::response_level::STATUS,
            "Connection Succeeded.",
            amf::Version::Amf0,
        )?;
        self.flush_chunk().await?;
        Ok(())
    }

    #[instrument]
    async fn process_create_stream_command(
        &mut self,
        request: CreateStreamCommandRequest,
    ) -> RtmpPublishServerResult<()> {
        self.chunk_writer.write_create_stream_response(
            true,
            request.transaction_id,
            None,
            RESPONSE_STREAM_ID.into(),
        )?;
        self.flush_chunk().await?;
        Ok(())
    }

    #[instrument]
    async fn process_publish_command(
        &mut self,
        request: PublishCommand,
    ) -> RtmpPublishServerResult<()> {
        self.chunk_writer.write_on_status_response(
            "status",
            "NetStream.Publish.Start",
            "Publish Start",
            amf::Version::Amf0,
        )?;
        self.flush_chunk().await?;
        Ok(())
    }

    #[instrument]
    async fn process_protocol_control_message(
        &mut self,
        request: ProtocolControlMessage,
    ) -> RtmpPublishServerResult<()> {
        match request {
            ProtocolControlMessage::SetChunkSize(request) => {
                self.process_set_chunk_size_request(request).await?
            }
            ProtocolControlMessage::Abort(request) => {}
            ProtocolControlMessage::Ack(request) => {}
            ProtocolControlMessage::SetPeerBandwidth(request) => {}
            ProtocolControlMessage::WindowAckSize(request) => {}
        }
        Ok(())
    }

    async fn process_set_chunk_size_request(
        &mut self,
        request: SetChunkSize,
    ) -> RtmpPublishServerResult<()> {
        let chunk_size = request.chunk_size;
        let old_size = self.chunk_reader.set_chunk_size(chunk_size as usize);
        tracing::trace!(
            "update read chunk size, from {} to {}",
            old_size,
            chunk_size
        );
        Ok(())
    }
}
