use crate::{
    chunk::{ChunkMessageCommonHeader, errors::ChunkMessageError},
    commands,
};

use super::{RtmpMessageType, RtmpUserMessageBody};
use num::ToPrimitive;
use std::io::{self, Cursor, Read};

use utils::traits::reader::ReadRemainingFrom;

impl<R: io::Read> ReadRemainingFrom<(amf_formats::Version, bool, &ChunkMessageCommonHeader), R>
    for RtmpUserMessageBody
{
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: (amf_formats::Version, bool, &ChunkMessageCommonHeader),
        reader: &mut R,
    ) -> Result<Self, Self::Error> {
        let (version, c2s, header) = header;
        let mut payload = vec![0; header.message_length.to_usize().unwrap()];
        reader.read_exact(&mut payload)?;
        let mut payload_reader = Cursor::new(&payload);

        let message = match header.message_type_id.try_into()? {
            RtmpMessageType::AMF0Data | RtmpMessageType::AMF3Data => {
                RtmpUserMessageBody::MetaData {
                    payload: payload.into(),
                }
            }
            RtmpMessageType::Audio => RtmpUserMessageBody::Audio {
                payload: payload.into(),
            },
            RtmpMessageType::Video => RtmpUserMessageBody::Video {
                payload: payload.into(),
            },
            RtmpMessageType::Aggregate => RtmpUserMessageBody::Aggregate {
                payload: payload.into(),
            },
            RtmpMessageType::AMF0Command | RtmpMessageType::AMF3Command => {
                if c2s {
                    RtmpUserMessageBody::C2SCommand(commands::RtmpC2SCommands::read_remaining_from(
                        version,
                        payload_reader.by_ref(),
                    )?)
                } else {
                    todo!()
                }
            }
            RtmpMessageType::AMF0SharedObject | RtmpMessageType::AMF3SharedObject => {
                todo!("no spec on this")
            }
        };

        Ok(message)
    }
}
