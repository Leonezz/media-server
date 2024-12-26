use byteorder::ReadBytesExt;
use std::io::{self, Cursor};
use tokio_util::{bytes::BytesMut, either::Either};

use crate::{errors::FLVResult, tag::enhanced::ex_audio::ex_audio_header::ExAudioTagHeader};

use super::{AACPacketType, AudioTagHeader, SoundFormat, SoundRate, SoundSize, SoundType};

impl AudioTagHeader {
    pub fn read_from(
        mut reader: &mut Cursor<&mut BytesMut>,
    ) -> FLVResult<Either<AudioTagHeader, ExAudioTagHeader>> {
        let first_byte = reader.read_u8()?;
        let sound_format: SoundFormat = ((first_byte >> 4) & 0b1111).try_into()?;

        if sound_format == SoundFormat::ExHeader {
            let ex_header = ExAudioTagHeader::read_from(&mut reader, first_byte)?;
            return Ok(Either::Right(ex_header));
        }

        let sound_rate: SoundRate = ((first_byte >> 2) & 0b11).try_into()?;
        let sound_size: SoundSize = ((first_byte >> 1) & 0b1).into();
        let sound_type: SoundType = ((first_byte >> 0) & 0b1).into();
        let mut aac_packet_type: Option<AACPacketType> = None;
        if sound_format == SoundFormat::AAC {
            let aac_type_byte = reader.read_u8()?;
            aac_packet_type = Some(aac_type_byte.into());
        }
        Ok(Either::Left(AudioTagHeader {
            sound_format,
            sound_rate,
            sound_size,
            sound_type,
            aac_packet_type,
        }))
    }
}
