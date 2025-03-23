use std::{
    io::{self, Cursor},
    time::Duration,
};

use rtmp_formats::{
    chunk::{self, ChunkMessage, errors::ChunkMessageError},
    handshake,
    protocol_control::SetPeerBandwidth,
};
use stream_center::gop::FLVMediaFrame;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
    time,
};
use tokio_util::bytes::{Buf, BytesMut};

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
}

impl RtmpChunkStream {
    pub fn new(
        read_buffer_capacity: u64,
        write_buffer_capacity: u64,
        io: TcpStream,
        chunk_size: u32,
        read_timeout_ms: u64,
        write_timeout_ms: u64,
    ) -> Self {
        Self {
            chunk_reader: chunk::reader::Reader::new(),
            chunk_writer: chunk::writer::Writer::new(),
            read_buffer: BytesMut::with_capacity(read_buffer_capacity as usize),
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
                Ok(Err(err)) => return Err(err.into()),
                Err(err) => {
                    return Err(RtmpServerError::Io(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("read chunk data timeout: {}", err),
                    )));
                }
            }
        }
    }

    async fn flush_chunk(&mut self) -> RtmpServerResult<()> {
        let flushable = match &self.ack_window_size_write {
            None => true,
            Some(limit) => {
                let unacknowledged_bytes =
                    self.total_wrote_bytes - self.acknowledged_sequence_number.unwrap_or(0) as u64;
                unacknowledged_bytes < limit.size as u64
            }
        };
        if !flushable {
            log::error!("not flushable");
            return Ok(());
        }

        tokio::time::timeout(Duration::from_millis(self.write_timeout_ms), async move {
            self.chunk_writer.write_to(&mut self.stream).await?;
            self.stream.flush().await?;
            self.total_wrote_bytes = self.chunk_writer.get_bytes_written();
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
        log::info!("do ack: {}", size);
        self.chunk_writer.write_acknowledgement_message(size)?;
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

    pub async fn write_tag(&mut self, tag: &FLVMediaFrame) -> RtmpServerResult<()> {
        match tag {
            FLVMediaFrame::Audio {
                runtime_stat: _,
                pts,
                header: _,
                payload,
            } => {
                self.chunk_writer
                    .write_audio(payload.clone(), *pts as u32)?;
            }
            FLVMediaFrame::Video {
                runtime_stat: _,
                pts,
                header: _,
                payload,
            } => {
                self.chunk_writer.write_video(payload.clone(), *pts as u32);
            }
            FLVMediaFrame::Script {
                runtime_stat: _,
                pts,
                on_meta_data: _,
                payload,
            } => {
                self.chunk_writer.write_meta(payload.clone(), *pts as u32);
            }
        }
        self.flush_chunk().await?;
        Ok(())
    }
}
