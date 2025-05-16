use std::io::{self, Read};

use num::ToPrimitive;
use tokio_util::{bytes::Buf, either::Either};
use utils::traits::reader::{ReadFrom, ReadRemainingFrom};

use crate::{
    errors::{FLVError, FLVResult},
    tag::{
        audio_tag_header::AudioTagHeader,
        encryption::{
            EncryptionFilterParams, EncryptionTagHeader, FilterParams,
            SelectiveEncryptionFilterParams,
        },
        flv_tag_header::{FLVTagHeader, FLVTagType},
        video_tag_header::VideoTagHeader,
    },
};

use super::{FLVTagBody, FLVTagBodyWithFilter, Filter};

impl<R: io::Read> ReadFrom<R> for Filter {
    type Error = FLVError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let tag_header = EncryptionTagHeader::read_from(reader)?;
        let mut bytes = vec![0_u8; tag_header.length as usize];
        reader.read_exact(&mut bytes)?;
        let mut cursor_bytes = io::Cursor::new(bytes);
        let filter_params = match tag_header.filter_name.as_str() {
            "Encryption" => Either::Left(EncryptionFilterParams::read_from(&mut cursor_bytes)?),
            "SE" => Either::Right(SelectiveEncryptionFilterParams::read_from(
                &mut cursor_bytes,
            )?),
            name => {
                return Err(FLVError::UnexpectedValue(format!(
                    "expect Encryption or SE for filter name, got {} instead",
                    name
                )));
            }
        };

        Ok(Filter {
            encryption_header: tag_header,
            filter_params: FilterParams { filter_params },
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<&FLVTagHeader, R> for FLVTagBodyWithFilter {
    type Error = FLVError;
    fn read_remaining_from(header: &FLVTagHeader, reader: &mut R) -> Result<Self, Self::Error> {
        // header.data_size tells us the total bytes count of the filter and the audio/video header plus the body part
        let mut body_bytes = vec![0; header.data_size.to_usize().unwrap()];
        reader.read_exact(&mut body_bytes)?;
        let mut cursor = io::Cursor::new(body_bytes);

        // TODO(check) - how the fuck should I do with the filter params, the spec sucks
        let filter = if !header.filter_enabled {
            None
        } else {
            Some(Filter::read_from(&mut cursor)?)
        };

        match header.tag_type {
            FLVTagType::Audio => {
                let tag_header = AudioTagHeader::read_from(&mut cursor)?;
                let mut audio_body_bytes = vec![0; cursor.remaining()];
                cursor.read_exact(&mut audio_body_bytes)?;

                Ok(FLVTagBodyWithFilter {
                    filter,
                    body: FLVTagBody::Audio {
                        header: tag_header,
                        body: audio_body_bytes.into(),
                    },
                })
            }
            FLVTagType::Video => {
                let tag_header = VideoTagHeader::read_from(&mut cursor)?;
                let mut video_body_bytes = vec![0; cursor.remaining()];
                cursor.read_exact(&mut video_body_bytes)?;

                Ok(FLVTagBodyWithFilter {
                    filter,
                    body: FLVTagBody::Video {
                        header: tag_header,
                        body: video_body_bytes.into(),
                    },
                })
            }
            FLVTagType::Script => {
                let (name, value) = read_meta(cursor.by_ref())?;
                Ok(FLVTagBodyWithFilter {
                    filter,
                    body: FLVTagBody::Script { name, value },
                })
            }
        }
    }
}

fn read_meta<R: io::Read>(
    reader: &mut R,
) -> FLVResult<(String, Vec<(String, amf_formats::amf0::Value)>)> {
    let name = amf_formats::amf0::Value::read_from(reader)?;
    let name_str = match name {
        amf_formats::amf0::Value::String(str) => str,
        _ => {
            return Err(FLVError::UnexpectedValue(format!(
                "expect an amf string for meta name, got {:?} instead",
                name
            )));
        }
    };
    let value = amf_formats::amf0::Value::read_from(reader)?;
    let value_arr = match value {
        amf_formats::amf0::Value::ECMAArray(arr) => arr,
        _ => {
            return Err(FLVError::UnexpectedValue(format!(
                "expect an amf ECMA Array for meta value, got {:?} instead",
                value
            )));
        }
    };
    Ok((name_str, value_arr))
}
