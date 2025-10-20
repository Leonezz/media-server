use std::{
    fmt,
    io::{self, Read},
};

use crate::{channel_data::ChannelData, errors::TurnMessageError};
use byteorder::ReadBytesExt;
use tokio_util::{
    bytes::{Buf, BufMut},
    codec::{Decoder, Encoder},
};
use utils::traits::{
    protocol_message::ProtocolMessage,
    reader::{ReadFrom, ReadRemainingFrom, TryReadFrom, TryReadRemainingFrom},
    writer::WriteTo,
};

#[derive(Debug, Clone)]
pub enum Message {
    Stun(stun_formats::message::Message),
    ChannelData(ChannelData),
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChannelData(data) => write!(f, "{}", data),
            Self::Stun(stun) => write!(f, "{}", stun),
        }
    }
}

impl From<stun_formats::message::Message> for Message {
    fn from(value: stun_formats::message::Message) -> Self {
        Self::Stun(value)
    }
}

impl From<ChannelData> for Message {
    fn from(value: ChannelData) -> Self {
        Self::ChannelData(value)
    }
}

impl<R: std::io::Read> ReadFrom<R> for Message {
    type Error = TurnMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let first_byte = reader.read_u8()?;
        match first_byte {
            0..=3 => {
                let stun_message =
                    stun_formats::message::Message::read_remaining_from(first_byte, reader)?;
                Ok(Self::Stun(stun_message))
            }
            64..=79 => {
                let channel_data = ChannelData::read_remaining_from(first_byte, reader)?;
                Ok(Self::ChannelData(channel_data))
            }
            _ => Err(TurnMessageError::UnknownFirstByte(first_byte)),
        }
    }
}

impl<R: AsRef<[u8]>> TryReadFrom<R> for Message {
    type Error = TurnMessageError;
    fn try_read_from(reader: &mut io::Cursor<R>) -> Result<Option<Self>, Self::Error> {
        if !reader.has_remaining() {
            return Ok(None);
        }
        let first_byte = reader.read_u8()?;
        match first_byte {
            0..=3 => {
                let stun_message =
                    stun_formats::message::Message::try_read_remaining_from(first_byte, reader)?
                        .map(|item| Self::Stun(item));
                Ok(stun_message)
            }
            64..=79 => {
                let channel_data = ChannelData::try_read_remaining_from(first_byte, reader)?
                    .map(|item| Self::ChannelData(item));
                Ok(channel_data)
            }
            _ => Err(TurnMessageError::UnknownFirstByte(first_byte)),
        }
    }
}

impl<W: io::Write> WriteTo<W> for Message {
    type Error = TurnMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::Stun(stun) => {
                stun.write_to(writer)?;
                Ok(())
            }
            Self::ChannelData(channel_data) => channel_data.write_to(writer),
        }
    }
}

#[derive(Debug)]
pub struct MessageFramed;
impl Encoder<Message> for MessageFramed {
    type Error = TurnMessageError;
    fn encode(
        &mut self,
        item: Message,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        item.write_to(&mut dst.writer())?;
        Ok(())
    }
}

impl Decoder for MessageFramed {
    type Error = TurnMessageError;
    type Item = Message;
    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let (res, position) = {
            let mut cursor = io::Cursor::new(&src);
            let res = Message::try_read_from(cursor.by_ref());
            (res, cursor.position())
        };
        if res.is_ok() && res.as_ref().unwrap().is_some() {
            src.advance(position as usize);
        }
        res
    }
}

impl ProtocolMessage for Message {
    type Codec = MessageFramed;
    type Error = TurnMessageError;
    type In = Message;
    type Out = Message;
    fn codec() -> Self::Codec {
        MessageFramed {}
    }
}
