use byteorder::{BigEndian, WriteBytesExt};
use std::io;
use tokio_util::either::Either;

use crate::errors::FLVResult;

use super::{FLVTag, FLVTagBody, FLVTagBodyWithFilter, FLVTagType, Filter};

#[derive(Debug)]
pub struct Writer<W> {
    inner: W,
}

impl<W> Writer<W>
where
    W: io::Write,
{
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    pub fn write(&mut self, tag: &FLVTag) -> FLVResult<()> {
        let mut byte: u8 = 0;
        if tag.body_with_filter.filter.is_some() {
            byte = 0b0010_0000;
        }
        byte |= <FLVTagType as Into<u8>>::into(tag.tag_type);
        self.inner.write_u8(byte)?;
        self.inner.write_u24::<BigEndian>(tag.data_size)?;
        self.inner
            .write_u24::<BigEndian>(tag.timestamp & 0x00FF_FFFF)?;
        self.inner.write_u8(((tag.timestamp >> 24) & 0xFF) as u8)?;
        self.inner.write_u24::<BigEndian>(0)?;

        self.write_tag_body(&tag.body_with_filter)?;

        Ok(())
    }

    fn write_tag_body(&mut self, tag_body: &FLVTagBodyWithFilter) -> FLVResult<()> {
        if let Some(filter) = &tag_body.filter {
            self.write_filter(filter)?;
        }
        match &tag_body.body {
            FLVTagBody::Audio { header, body } => {
                header.write_to(self.inner.by_ref())?;
                self.inner.write_all(&body)?;
            }
            FLVTagBody::Video { header, body } => {
                header.write_to(self.inner.by_ref())?;
                self.inner.write_all(&body)?;
            }
            FLVTagBody::Meta { name, value } => {
                amf::amf0::Value::String(name.clone()).write_to(self.inner.by_ref())?;
                amf::amf0::Value::ECMAArray(value.clone()).write_to(self.inner.by_ref())?;
            }
        }
        Ok(())
    }

    fn write_filter(&mut self, filter: &Filter) -> FLVResult<()> {
        filter.encryption_header.write_to(self.inner.by_ref())?;
        match &filter.filter_params.filter_params {
            Either::Left(encryption_params) => {
                encryption_params.write_to(self.inner.by_ref())?;
            }
            Either::Right(se_params) => {
                se_params.write_to(self.inner.by_ref())?;
            }
        }
        Ok(())
    }
}

impl FLVTag {
    pub fn write_to<W>(&self, writer: W) -> FLVResult<()>
    where
        W: io::Write,
    {
        Writer::new(writer).write(self)
    }
}
