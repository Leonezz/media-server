use std::io;

use utils::traits::writer::WriteTo;

use super::{FLVTagHeader, FLVTagType};
use crate::{
    errors::FLVError,
    tag::{
        audio_tag_header::AudioTagHeader,
        flv_tag_body::{FLVTagBody, FLVTagBodyWithFilter},
        video_tag_header::VideoTagHeader,
    },
};
use byteorder::{BigEndian, WriteBytesExt};

impl<W: io::Write> WriteTo<W> for FLVTagHeader {
    type Error = FLVError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let mut byte: u8 = 0;
        if self.filter_enabled {
            byte = 0b0010_0000;
        }
        byte |= <FLVTagType as Into<u8>>::into(self.tag_type);
        writer.write_u8(byte)?;
        writer.write_u24::<BigEndian>(self.data_size)?;
        writer.write_u24::<BigEndian>(self.timestamp & 0x00FF_FFFF)?;
        writer.write_u8(((self.timestamp >> 24) & 0xFF) as u8)?;
        writer.write_u24::<BigEndian>(0)?;
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for FLVTagBodyWithFilter {
    type Error = FLVError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        if let Some(filter) = &self.filter {
            filter.write_to(writer)?;
        }
        match &self.body {
            FLVTagBody::Audio { header, body } => {
                match header {
                    AudioTagHeader::Legacy(header) => header.write_to(writer)?,
                    AudioTagHeader::Enhanced(_ex_header) => {
                        todo!()
                    }
                }
                writer.write_all(body)?;
            }
            FLVTagBody::Video { header, body } => {
                match header {
                    VideoTagHeader::Legacy(header) => header.write_to(writer)?,
                    VideoTagHeader::Enhanced(_ex_header) => {
                        todo!()
                    }
                }
                writer.write_all(body)?;
            }
            FLVTagBody::Script { name, value } => {
                amf_formats::amf0::Value::write_string(writer, name)?;
                amf_formats::amf0::Value::write_ecma_array(writer, value)?;
            }
        }
        Ok(())
    }
}
