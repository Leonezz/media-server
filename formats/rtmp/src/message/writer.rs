use std::io;

use utils::traits::writer::WriteTo;

use super::RtmpUserMessageBody;
use crate::{chunk::errors::ChunkMessageError, commands::writer::RtmpCommandWriteWrapper};

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, RtmpUserMessageBody> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (message, version) = (self.0, self.1);
        match message {
            RtmpUserMessageBody::C2SCommand(command) => {
                RtmpCommandWriteWrapper::new(command, version).write_to(writer)
            }
            RtmpUserMessageBody::S2Command(command) => {
                RtmpCommandWriteWrapper::new(command, version).write_to(writer)
            }
            RtmpUserMessageBody::MetaData { payload }
            | RtmpUserMessageBody::Audio { payload }
            | RtmpUserMessageBody::Video { payload }
            | RtmpUserMessageBody::Aggregate { payload } => {
                writer.write_all(payload).map_err(ChunkMessageError::Io)
            }
            RtmpUserMessageBody::SharedObject() => todo!(),
        }
    }
}
