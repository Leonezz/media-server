use std::{
    cmp::min,
    io::{self, Cursor},
    time::Duration,
};

use flv_formats::tag::{
    audio_tag_header::LegacyAudioTagHeader, video_tag_header::LegacyVideoTagHeader,
};
use num::ToPrimitive;
use rtmp_formats::{
    chunk::{self, ChunkMessage, RtmpChunkMessageBody, errors::ChunkMessageError},
    handshake,
    protocol_control::{
        AbortMessage, Acknowledgement, ProtocolControlMessage, SetChunkSize,
        SetPeerBandWidthLimitType, SetPeerBandwidth, WindowAckSize,
    },
};
use stream_center::gop::MediaFrame;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
    time,
};
use tokio_util::bytes::{Buf, BufMut, BytesMut};
use utils::traits::{dynamic_sized_packet::DynamicSizedPacket, writer::WriteTo};

use crate::errors::{RtmpServerError, RtmpServerResult};

#[derive(Debug)]
pub struct RtmpChunkStream {
    chunk_reader: chunk::reader::Reader,
    chunk_writer: chunk::writer::Writer,
    read_buffer: BytesMut,
    stream: BufWriter<TcpStream>,
    read_timeout_ms: u64,
    write_timeout_ms: u64,

    chunk_size: u32,
    ack_window_size_read: Option<u32>,
    ack_window_size_write: Option<SetPeerBandwidth>,
    acknowledged_sequence_number: Option<u32>,
    total_wrote_bytes: u64,
    read_buffer_capacity: usize,
}

impl RtmpChunkStream {
    pub fn new(
        read_buffer_capacity: u64,
        io: TcpStream,
        chunk_size: u32,
        read_timeout_ms: u64,
        write_timeout_ms: u64,
    ) -> Self {
        Self {
            chunk_reader: chunk::reader::Reader::new(),
            chunk_writer: chunk::writer::Writer::new(),
            read_buffer: BytesMut::with_capacity(read_buffer_capacity as usize),
            read_buffer_capacity: read_buffer_capacity as usize,
            // write_buffer: BytesMut::with_capacity(write_buffer_capacity as usize),
            stream: BufWriter::new(io),
            read_timeout_ms,
            write_timeout_ms,
            chunk_size,
            ack_window_size_read: None,
            ack_window_size_write: None,
            acknowledged_sequence_number: None,
            total_wrote_bytes: 0,
        }
    }

    pub fn total_wrote_bytes(&self) -> u64 {
        self.total_wrote_bytes
    }

    pub async fn read_chunk(&mut self) -> RtmpServerResult<Option<ChunkMessage>> {
        loop {
            let mut buf = Cursor::new(&self.read_buffer);
            match self.chunk_reader.read(&mut buf, true) {
                Ok(Some(chunk_message)) => {
                    self.read_buffer.advance(buf.position() as usize);

                    if let RtmpChunkMessageBody::ProtocolControl(protocol_control) =
                        chunk_message.chunk_message_body
                    {
                        self.process_protocol_control_message(protocol_control)
                            .await?;
                        return Ok(None);
                    }

                    return Ok(Some(chunk_message));
                }
                Ok(None) => {}
                Err(ChunkMessageError::IncompleteChunk) => {
                    self.read_buffer.advance(buf.position() as usize);
                    return Ok(None);
                }
                Err(err) => return Err(err.into()),
            }

            match self.ack_window_size_read {
                None => {}
                Some(size) => {
                    if self.chunk_reader.get_bytes_read() >= size {
                        self.ack_window_size(size).await?;
                    }
                }
            }

            self.read_buffer.reserve(self.read_buffer_capacity);
            match tokio::time::timeout(
                time::Duration::from_millis(self.read_timeout_ms),
                self.stream.read_buf(&mut self.read_buffer),
            )
            .await
            {
                Ok(Ok(len)) => {
                    if len == 0 {
                        if self.read_buffer.is_empty() {
                            return Ok(None);
                        } else {
                            return Err(RtmpServerError::Io(io::Error::new(
                                io::ErrorKind::ConnectionReset,
                                "connect reset by peer",
                            )));
                        }
                    }
                }
                Ok(Err(err)) => {
                    return {
                        tracing::error!("io error: {}", err);
                        Err(err.into())
                    };
                }
                Err(err) => {
                    return Err(RtmpServerError::Io(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("read chunk data timeout: {}", err),
                    )));
                }
            }
        }
    }

    pub async fn flush_chunk(&mut self) -> RtmpServerResult<()> {
        let flushable = match &self.ack_window_size_write {
            None => true,
            Some(limit) => {
                let unacknowledged_bytes =
                    self.total_wrote_bytes - self.acknowledged_sequence_number.unwrap_or(0) as u64;
                unacknowledged_bytes < limit.size as u64
            }
        };
        if !flushable {
            return Ok(());
        }

        tokio::time::timeout(Duration::from_millis(self.write_timeout_ms), async move {
            self.chunk_writer.write_to(&mut self.stream).await?;
            self.stream.flush().await?;
            self.total_wrote_bytes = self.chunk_writer.get_bytes_written() as u64;
            Ok::<(), RtmpServerError>(())
        })
        .await
        .map_err(|err| {
            RtmpServerError::Io(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("write chunk timeout, {}", err),
            ))
        })??;
        Ok(())
    }

    async fn ack_window_size(&mut self, size: u32) -> RtmpServerResult<()> {
        self.chunk_writer.write_acknowledgement_message(size)?;
        self.flush_chunk().await?;
        Ok(())
    }

    pub async fn write_chunk_size(&mut self, size: u32) -> RtmpServerResult<()> {
        self.chunk_writer.write_set_chunk_size(size)?;
        self.flush_chunk().await?;
        Ok(())
    }

    pub async fn handshake(&mut self) -> RtmpServerResult<()> {
        handshake::server::HandshakeServer::new(&mut self.stream)
            .handshake(false)
            .await?;
        self.chunk_writer.write_set_chunk_size(self.chunk_size)?;
        Ok(())
    }

    pub async fn write_tag(&mut self, tag: &MediaFrame) -> RtmpServerResult<()> {
        match tag {
            MediaFrame::Audio {
                runtime_stat: _,
                frame_info,
                payload,
            } => {
                let tag_header: LegacyAudioTagHeader = frame_info.try_into()?;
                let mut payload_bytes =
                    Vec::with_capacity(payload.len() + tag_header.get_packet_bytes_count());
                tag_header.write_to(&mut payload_bytes)?;
                payload_bytes.put_slice(payload);
                self.chunk_writer.write_audio(
                    payload_bytes.into(),
                    frame_info
                        .timestamp_nano
                        .checked_div(1_000_000)
                        .unwrap()
                        .to_u32()
                        .unwrap(),
                )?;
            }
            MediaFrame::Video {
                runtime_stat: _,
                frame_info,
                payload,
            } => {
                let tag_header: LegacyVideoTagHeader = frame_info.try_into()?;
                let mut payload_bytes =
                    Vec::with_capacity(payload.len() + tag_header.get_packet_bytes_count());
                tag_header.write_to(&mut payload_bytes)?;
                payload_bytes.put_slice(payload);
                self.chunk_writer.write_video(
                    payload_bytes.into(),
                    frame_info
                        .timestamp_nano
                        .checked_div(1_000_000)
                        .unwrap()
                        .to_u32()
                        .unwrap(),
                )?;
            }
            MediaFrame::Script {
                runtime_stat: _,
                timestamp_nano: pts,
                on_meta_data: _,
                payload,
            } => {
                self.chunk_writer.write_meta(payload.clone(), *pts as u32)?;
            }
        }
        self.flush_chunk().await?;
        Ok(())
    }

    async fn process_protocol_control_message(
        &mut self,
        request: ProtocolControlMessage,
    ) -> RtmpServerResult<()> {
        match request {
            ProtocolControlMessage::SetChunkSize(request) => {
                self.process_set_chunk_size_request(request);
            }
            ProtocolControlMessage::Abort(request) => self.process_abort_chunk_request(request),
            ProtocolControlMessage::Ack(request) => {
                self.process_acknowledge_request(request);
            }
            ProtocolControlMessage::SetPeerBandwidth(request) => {
                self.process_set_peer_bandwidth_request(request).await?;
            }
            ProtocolControlMessage::WindowAckSize(request) => {
                self.process_window_ack_size_request(request);
            }
        }
        Ok(())
    }

    fn process_set_chunk_size_request(&mut self, request: SetChunkSize) {
        let chunk_size = request.chunk_size;
        let old_size = self.chunk_reader.set_chunk_size(chunk_size as usize);
        tracing::trace!(
            "update read chunk size, from {} to {}",
            old_size,
            chunk_size
        );
    }

    async fn process_set_peer_bandwidth_request(
        &mut self,
        request: SetPeerBandwidth,
    ) -> RtmpServerResult<()> {
        tracing::info!("got set_peer_bandwidth request: {:?}", request);
        let mut window_ack_size = None;
        match &mut self.ack_window_size_write {
            None => self.ack_window_size_write = Some(request),
            Some(limit) => match request.limit_type {
                SetPeerBandWidthLimitType::Hard => {
                    if limit.size != request.size {
                        window_ack_size = Some(request.size);
                    }
                    *limit = request
                }
                SetPeerBandWidthLimitType::Soft => {
                    if request.size != limit.size {
                        window_ack_size = Some(request.size);
                    }
                    limit.size = min(limit.size, request.size)
                }
                SetPeerBandWidthLimitType::Dynamic => {
                    if limit.limit_type == SetPeerBandWidthLimitType::Hard {
                        if limit.size != request.size {
                            window_ack_size = Some(request.size);
                        }
                        limit.size = request.size;
                    } else {
                        tracing::trace!(
                            "ignore set_peer_bandwidth command as documented by the spec, req: {:?}",
                            request
                        );
                    }
                }
            },
        }

        if window_ack_size.is_some() {
            self.chunk_writer
                .write_window_ack_size_message(window_ack_size.expect("this cannot be none"))?;
            self.flush_chunk().await?;
        }
        Ok(())
    }

    fn process_acknowledge_request(&mut self, request: Acknowledgement) {
        tracing::info!("got acknowledge request: {:?}", request);
        self.acknowledged_sequence_number = Some(request.sequence_number);
    }

    fn process_abort_chunk_request(&mut self, request: AbortMessage) {
        tracing::info!("got abort request: {:?}", request);
        self.chunk_reader
            .abort_chunk_message(request.chunk_stream_id);
    }

    fn process_window_ack_size_request(&mut self, request: WindowAckSize) {
        tracing::info!("got window_ack_size request: {:?}", request);
        self.ack_window_size_read = Some(request.size);
    }

    pub fn chunk_writer(&mut self) -> &mut rtmp_formats::chunk::writer::Writer {
        &mut self.chunk_writer
    }
}
