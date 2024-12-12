use std::io::{self, Cursor};

use rtmp_formats::{
    chunk::{self, ChunkMessage},
    handshake,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
};
use tokio_util::bytes::{Buf, BytesMut};

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

    pub async fn read_chunk(&mut self) -> RtmpPublishServerResult<Option<ChunkMessage>> {
        loop {
            let mut buf = Cursor::new(&self.read_buffer[..]);
            if let Some(chunk_message) = self.chunk_reader.read(&mut buf, true)? {
                self.read_buffer.advance(buf.position() as usize);
                return Ok(Some(chunk_message));
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

    pub async fn flush_chunk(&mut self) -> RtmpPublishServerResult<()> {
        self.chunk_writer.write_to(&mut self.stream).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn run(&mut self) -> RtmpPublishServerResult<()> {
        handshake::server::HandshakeServer::new(&mut self.stream)
            .handshake(false)
            .await?;

        loop {
            match self.read_chunk().await {
                Ok(maybe_chunk) => match maybe_chunk {
                    Some(message) => {
                        tracing::info!("got message: {:?}", message);
                    }
                    None => {}
                },
                Err(err) => {
                    tracing::error!("error when reading chunk messages: {:?}", err);
                }
            }
        }
    }
}
