use byteorder::{BigEndian, ReadBytesExt};
use tokio_util::{
    bytes::{Buf, BytesMut},
    either::Either,
};

use std::io::{self, Cursor, Read};

use crate::errors::{FLVError, FLVResult};

use super::{
    FLVTag, FLVTagBody, FLVTagBodyWithFilter, FLVTagHeader, FLVTagType, Filter, audio_tag_header,
    encryption::{
        EncryptionFilterParams, EncryptionTagHeader, FilterParams, SelectiveEncryptionFilterParams,
    },
    video_tag_header,
};

pub struct Reader<R> {
    inner: R,
}

impl<R> Reader<R>
where
    R: io::Read,
{
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn read(&mut self) -> FLVResult<FLVTag> {
        let tag_header = self.read_tag_header()?;
        let mut payload = BytesMut::with_capacity(tag_header.data_size as usize);
        self.inner.read_exact(&mut payload)?;

        let tag_body =
            FLVTag::read_tag_body(tag_header.tag_type, payload, tag_header.filter_enabled)?;
        Ok(FLVTag {
            tag_header,
            body_with_filter: tag_body,
        })
    }

    pub fn read_tag_header(&mut self) -> FLVResult<FLVTagHeader> {
        let first_byte = self.inner.read_u8()?;
        let filter_enabled = ((first_byte >> 5) & 0b1) != 0;
        let tag_type: FLVTagType = ((first_byte >> 0) & 0b11111).try_into()?;
        let data_size = self.inner.read_u24::<BigEndian>()?;
        let timestamp = self.inner.read_u24::<BigEndian>()?;
        let timestamp_extended = self.inner.read_u8()?;
        let timestamp = (((timestamp_extended as u32) << 24) | timestamp) & 0x7FFF_FFFF;
        let _stream_id = self.inner.read_u24::<BigEndian>()?;

        Ok(FLVTagHeader {
            tag_type,
            data_size,
            timestamp,
            filter_enabled,
        })
    }
}

impl FLVTag {
    pub fn read_from<R>(reader: R) -> FLVResult<Self>
    where
        R: io::Read,
    {
        Reader::new(reader).read()
    }

    pub fn read_tag_header_from<R>(reader: R) -> FLVResult<FLVTagHeader>
    where
        R: io::Read,
    {
        Reader::new(reader).read_tag_header()
    }

    pub fn read_tag_body(
        tag_type: FLVTagType,
        mut payload: BytesMut,
        filter_enabled: bool,
    ) -> FLVResult<FLVTagBodyWithFilter> {
        let mut cursor_bytes = Cursor::new(&mut payload);

        // TODO(check) - how the fuck should I do with the filter params, the spec sucks
        let filter = if !filter_enabled {
            None
        } else {
            Some(Self::read_filter(&mut cursor_bytes)?)
        };
        match tag_type {
            FLVTagType::Audio => {
                let tag_header = audio_tag_header::AudioTagHeader::read_from(&mut cursor_bytes)?;
                let mut audio_body_bytes = BytesMut::new();
                let remaining_bytes = cursor_bytes.remaining();
                audio_body_bytes.resize(remaining_bytes, 0);
                cursor_bytes.read_exact(&mut audio_body_bytes)?;
                return Ok(FLVTagBodyWithFilter {
                    filter,
                    body: FLVTagBody::Audio {
                        header: tag_header,
                        body: audio_body_bytes,
                    },
                });
            }
            FLVTagType::Video => {
                let tag_header = video_tag_header::VideoTagHeader::read_from(&mut cursor_bytes)?;
                let mut video_body_bytes = BytesMut::new();
                let remaining_bytes = cursor_bytes.remaining();
                video_body_bytes.resize(remaining_bytes, 0);
                cursor_bytes.read_exact(&mut video_body_bytes)?;
                return Ok(FLVTagBodyWithFilter {
                    filter,
                    body: FLVTagBody::Video {
                        header: tag_header,
                        body: video_body_bytes,
                    },
                });
            }
            FLVTagType::Script => Ok(FLVTagBodyWithFilter {
                filter,
                body: Self::read_meta(&mut cursor_bytes)?,
            }),
        }
    }

    pub fn read_filter<Reader>(mut reader: Reader) -> FLVResult<Filter>
    where
        Reader: io::Read,
    {
        let tag_header = EncryptionTagHeader::read_from(reader.by_ref())?;
        let mut bytes = vec![0 as u8; tag_header.length as usize];
        reader.read_exact(&mut bytes)?;
        let mut cursor_bytes = Cursor::new(bytes);
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

    pub fn read_meta<Reader>(mut reader: Reader) -> FLVResult<FLVTagBody>
    where
        Reader: io::Read,
    {
        let name = amf::amf0::Value::read_from(reader.by_ref())?;
        if name.is_none() {
            return Err(FLVError::UnexpectedValue(
                "expect an amf string for meta name, got nothing".to_string(),
            ));
        }

        let name = name.expect("this cannot be none");
        let name_str;
        match name {
            amf::amf0::Value::String(str) => {
                name_str = str;
            }
            _ => {
                return Err(FLVError::UnexpectedValue(format!(
                    "expect an amf string for meta name, got {:?} instead",
                    name
                )));
            }
        }

        let value = amf::amf0::Value::read_from(reader.by_ref())?;
        if value.is_none() {
            return Err(FLVError::UnexpectedValue(
                "expected an amf ECMA Array for meta value, got nothing".to_string(),
            ));
        }
        let value = value.expect("this cannot be none");
        let value_arr;
        match value {
            amf::amf0::Value::ECMAArray(arr) => {
                value_arr = arr;
            }
            _ => {
                return Err(FLVError::UnexpectedValue(format!(
                    "expect an amf ECMA Array for meta value, got {:?} instead",
                    value
                )));
            }
        }
        Ok(FLVTagBody::Script {
            name: name_str,
            value: value_arr,
        })
    }
}
