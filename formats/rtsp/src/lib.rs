use std::{
    fmt::{self, Write},
    io::{self, BufRead, Read, Seek},
};

use byteorder::ReadBytesExt;
use consts::{
    common::{LF, SPACE_STR},
    methods::RtspMethod,
    version::RtspVersion,
};
use errors::RTSPMessageError;
use interleaved::{DOLLAR_SIGN, RtspInterleavedPacket};
use request::RtspRequest;
use response::RtspResponse;
use tokio_util::{
    bytes::{Buf, BufMut, BytesMut},
    codec::{Decoder, Encoder},
};
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    reader::{ReadFrom, TryReadFrom, TryReadRemainingFrom},
    writer::WriteTo,
};

pub mod consts;
pub mod errors;
pub mod header;
pub mod interleaved;
pub mod request;
pub mod response;
mod util;

#[derive(Debug)]
pub enum RtspMessage {
    Request(RtspRequest),
    Response(RtspResponse),
    Interleaved(RtspInterleavedPacket),
}

impl<R: io::BufRead> ReadFrom<R> for RtspMessage {
    type Error = RTSPMessageError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let buffer = reader.fill_buf()?;
        let mut cursor = io::Cursor::new(buffer);
        Self::try_read_from(cursor.by_ref()).map(|msg| {
            msg.ok_or(RTSPMessageError::InvalidRtspMessageFormat(
                "rtsp message is incomplete".to_string(),
            ))
        })?
    }
}

impl<R: AsRef<[u8]>> TryReadFrom<R> for RtspMessage {
    type Error = RTSPMessageError;
    fn try_read_from(reader: &mut io::Cursor<R>) -> Result<Option<Self>, Self::Error> {
        if !reader.has_remaining() {
            return Ok(None);
        }

        let first_byte = reader.read_u8().unwrap();
        if first_byte == DOLLAR_SIGN {
            RtspInterleavedPacket::try_read_remaining_from(first_byte, reader)
                .map(|interleaved| interleaved.map(Self::Interleaved))?;
        }
        reader.seek_relative(-1).unwrap();

        if !reader.fill_buf()?.contains(&LF) {
            return Ok(None);
        }
        let mut first_line = String::new();
        reader.fill_buf()?.read_line(&mut first_line)?;
        if let Some((first_word, _)) = first_line.split_once(SPACE_STR) {
            if let Ok(method) = RtspMethod::try_from(first_word) {
                reader.consume(first_word.len());
                return RtspRequest::try_read_remaining_from(method, reader)
                    .map(|req| req.map(Self::Request));
            }

            if let Ok(version) = RtspVersion::try_from(first_word) {
                reader.consume(first_word.len());
                return RtspResponse::try_read_remaining_from(version, reader)
                    .map(|res| res.map(Self::Response));
            }
        }

        Err(RTSPMessageError::InvalidRtspMessageFormat(format!(
            "not a rtsp message: {}",
            first_line
        )))
    }
}

impl<W: io::Write> WriteTo<W> for RtspMessage {
    type Error = RTSPMessageError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        match self {
            Self::Request(req) => write!(writer, "{}", req)?,
            Self::Response(res) => write!(writer, "{}", res)?,
            Self::Interleaved(interleaved) => interleaved.write_to(writer)?,
        }
        Ok(())
    }
}

impl fmt::Display for RtspMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(req) => write!(f, "{}", req),
            Self::Response(res) => write!(f, "{}", res),
            Self::Interleaved(interleaved) => {
                let mut bytes = BytesMut::with_capacity(interleaved.get_packet_bytes_count());
                let mut writer = bytes.writer();
                interleaved.write_to(&mut writer).unwrap();
                bytes = writer.into_inner();
                write!(f, "{}", String::from_utf8_lossy(bytes.as_ref()))
            }
        }
    }
}

#[derive(Debug)]
pub struct RtspMessageFramed;

impl Encoder<RtspMessage> for RtspMessageFramed {
    type Error = RTSPMessageError;

    fn encode(
        &mut self,
        item: RtspMessage,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        dst.write_fmt(format_args!("{}", item))?;
        Ok(())
    }
}

impl Decoder for RtspMessageFramed {
    type Error = RTSPMessageError;
    type Item = RtspMessage;

    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let (res, position) = {
            let mut cursor = io::Cursor::new(&src);
            let res = RtspMessage::try_read_from(cursor.by_ref());
            (res, cursor.position())
        };
        if res.is_ok() {
            src.advance(position as usize);
        }
        res
    }
}
