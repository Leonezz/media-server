use std::{borrow::BorrowMut, fmt::Debug, pin::Pin};

use rtmp_formats::{
    chunk::{self, ChunkMessage},
    handshake,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::bytes::{
    Buf, BufMut, BytesMut,
    buf::{Reader, Writer},
};
use tracing::info;

use crate::publish::errors::RtmpPublishServerError;

use super::errors::RtmpPublishServerResult;

#[derive(Debug)]
pub struct RtmpPublishSession {
    io: TcpStream,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    chunk_reader: chunk::reader::Reader<Reader<BytesMut>>,
    chunk_writer: chunk::writer::Writer<Writer<BytesMut>>,
}

impl RtmpPublishSession {
    pub fn new(io: TcpStream) -> Self {
        let read_buffer = BytesMut::with_capacity(4096);
        let write_buffer = BytesMut::with_capacity(4096);
        Self {
            io,
            chunk_reader: chunk::reader::Reader::new(read_buffer.clone().reader()),
            chunk_writer: chunk::writer::Writer::new(write_buffer.clone().writer()),
            read_buffer,
            write_buffer,
        }
    }

    pub async fn run(&mut self) -> RtmpPublishServerResult<()> {
        handshake::server::HandshakeServer::new(self.io.borrow_mut())
            .handshake(false)
            .await?;
        let mut read_buffer = [0 as u8; 4096];
        loop {
            let len = self.io.read(&mut read_buffer).await?;
            info!("bytes read: {}", len);
            self.read_buffer.extend_from_slice(&read_buffer[..len]);

            match self.chunk_reader.read(true) {
                Err(err) => {
                    tracing::error!("err when read chunk message: {:?}", err);
                    return Err(RtmpPublishServerError::ChunkMessageReadFailed(err));
                }
                Ok(message) => {
                    tracing::debug!("got chunk message: {:?}", message);
                }
            }
        }
    }
}
