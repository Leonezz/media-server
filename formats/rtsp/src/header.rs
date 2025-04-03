use std::{
    fmt,
    io::{self, Read},
};

use tokio_util::bytes::Buf;
use utils::traits::reader::{ReadFrom, TryReadFrom};

use crate::{
    consts::{common::CRLF_STR, headers::RtspHeader},
    errors::RTSPMessageError,
    util::TextReader,
};

#[derive(Debug, Default, Clone)]
pub struct RtspHeaders(Vec<(RtspHeader, String)>);

impl RtspHeaders {
    pub fn new(items: Vec<(RtspHeader, String)>) -> Self {
        Self(items)
    }
    pub fn push(&mut self, key: RtspHeader, value: String) {
        self.0.push((key, value));
    }

    pub fn append(&mut self, mut items: Vec<(RtspHeader, String)>) {
        self.0.append(&mut items);
    }

    pub fn get(&self, key: RtspHeader) -> Vec<&String> {
        self.0
            .iter()
            .filter(|(k, _)| k.eq(&key))
            .map(|(_, value)| value)
            .collect()
    }

    pub fn contains(&self, key: RtspHeader) -> bool {
        self.0.iter().any(|(k, _)| k.eq(&key))
    }

    pub fn remove(&mut self, key: RtspHeader) {
        self.0.retain(|(k, _)| k.ne(&key));
    }

    pub fn entries(&self) -> &Vec<(RtspHeader, String)> {
        &self.0
    }

    pub fn entries_mut(&mut self) -> &mut Vec<(RtspHeader, String)> {
        &mut self.0
    }
}

impl fmt::Display for RtspHeaders {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.entries().iter().try_for_each(|(key, value)| {
            f.write_fmt(format_args!("{}: {}{}", key, value, CRLF_STR))
        })
    }
}

impl<R: io::BufRead> ReadFrom<R> for RtspHeaders {
    type Error = RTSPMessageError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let buffer = reader.fill_buf()?.to_vec();
        let mut cursor = io::Cursor::new(&buffer);
        if let Some(headers) = Self::try_read_from(cursor.by_ref())? {
            reader.consume(cursor.position() as usize);
            return Ok(headers);
        }
        Err(RTSPMessageError::InvalidRtspMessageFormat(format!(
            "the message is incomplete: {}",
            String::from_utf8_lossy(&buffer),
        )))
    }
}

impl<R: AsRef<[u8]>> TryReadFrom<R> for RtspHeaders {
    type Error = RTSPMessageError;
    fn try_read_from(reader: &mut io::Cursor<R>) -> Result<Option<Self>, Self::Error> {
        if !reader.has_remaining() {
            return Ok(None);
        }
        let mut text_reader = TextReader::new(reader.by_ref());
        let mut headers = vec![];
        loop {
            let line = text_reader.read_line()?;
            if line.is_none() {
                // at least CRLF should be there
                return Ok(None);
            }

            let line = line.unwrap();
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() {
                break;
            }
            let parts: Vec<_> = trimmed_line.split(":").collect();
            if parts.len() < 2 {
                return Err(RTSPMessageError::InvalidRtspMessageFormat(format!(
                    "invalid header line: {}",
                    line
                )));
            }

            let key = RtspHeader::try_from(parts[0])?;
            let value = parts[1..].join(":");
            headers.push((key, value.trim().to_owned()));
        }

        Ok(Some(Self(headers)))
    }
}
