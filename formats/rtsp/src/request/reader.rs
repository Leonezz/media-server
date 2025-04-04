use std::{
    io::{self, BufRead, Read},
    str::FromStr,
};

use tokio_util::bytes::Buf;
use url::Url;
use utils::traits::reader::{ReadFrom, ReadRemainingFrom, TryReadFrom, TryReadRemainingFrom};

use crate::{
    consts::{
        common::{CR_STR, LF, LF_STR, SPACE, SPACE_STR},
        methods::RtspMethod,
        version::RtspVersion,
    },
    errors::RtspMessageError,
    header::{RtspHeader, RtspHeaders},
    util::TextReader,
};

use super::RtspRequest;

impl<R: io::BufRead> ReadRemainingFrom<RtspMethod, R> for RtspRequest {
    type Error = RtspMessageError;
    fn read_remaining_from(header: RtspMethod, mut reader: R) -> Result<Self, Self::Error> {
        let buffer = reader.fill_buf()?;
        let (res, position) = {
            let mut cursor = io::Cursor::new(buffer);
            (
                Self::try_read_remaining_from(header, &mut cursor)?.ok_or(
                    RtspMessageError::InvalidRtspMessageFormat(format!(
                        "rtsp request is not complete: {}",
                        String::from_utf8_lossy(buffer)
                    )),
                ),
                cursor.position(),
            )
        };

        if res.is_ok() {
            reader.consume(position as usize);
        }
        res
    }
}

impl<R: io::BufRead> ReadFrom<R> for RtspRequest {
    type Error = RtspMessageError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let buffer = reader.fill_buf()?;
        let (res, position) = {
            let mut cursor = io::Cursor::new(&buffer);
            (
                Self::try_read_from(&mut cursor)?.ok_or(
                    RtspMessageError::InvalidRtspMessageFormat(format!(
                        "rtsp request is not complete: {}",
                        String::from_utf8_lossy(buffer)
                    )),
                ),
                cursor.position(),
            )
        };
        if res.is_ok() {
            reader.consume(position as usize);
        }
        res
    }
}

impl FromStr for RtspRequest {
    type Err = RtspMessageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::read_from(s.as_bytes())
    }
}

impl<R: AsRef<[u8]>> TryReadRemainingFrom<RtspMethod, R> for RtspRequest {
    type Error = RtspMessageError;
    fn try_read_remaining_from(
        header: RtspMethod,
        reader: &mut io::Cursor<R>,
    ) -> Result<Option<Self>, Self::Error> {
        if !reader.has_remaining() {
            return Ok(None);
        }
        if !TextReader::new(reader.by_ref()).expect(&[SPACE])? {
            return Err(RtspMessageError::InvalidRtspMessageFormat(
                "rtsp request first line expect a space".to_string(),
            ));
        }
        let line = TextReader::new(reader.by_ref()).read_line()?;
        if line.is_none() {
            return Ok(None);
        }
        let line = line.unwrap();
        let trimed_line_parts: Vec<_> = line.trim().split(SPACE_STR).collect();
        if trimed_line_parts.len() != 2 {
            return Err(RtspMessageError::InvalidRtspMessageFormat(line));
        }

        let uri = trimed_line_parts[0].parse::<Url>()?;
        let version: RtspVersion = trimed_line_parts[1].parse()?;
        let headers = RtspHeaders::try_read_from(reader.by_ref())?;
        if headers.is_none() {
            return Ok(None);
        }
        let headers = headers.unwrap();

        let content_length = headers
            .get(RtspHeader::ContentLength)
            .first()
            .map(|s| s.parse::<usize>().ok());

        let mut text_reader = TextReader::new(reader.by_ref());
        text_reader.skip_empty_lines()?;
        let body = if let Some(Some(length)) = content_length {
            let body_str = text_reader.try_read_exact(length)?;
            if body_str.is_none() {
                return Ok(None);
            }
            Some({
                body_str
                    .unwrap()
                    .trim_start_matches(CR_STR)
                    .trim_start_matches(LF_STR)
                    .to_owned()
            })
        } else {
            None
        };

        Ok(Some(Self {
            method: header,
            uri,
            version,
            headers,
            body,
        }))
    }
}

impl<R: AsRef<[u8]>> TryReadFrom<R> for RtspRequest {
    type Error = RtspMessageError;
    fn try_read_from(reader: &mut io::Cursor<R>) -> Result<Option<Self>, Self::Error> {
        if !reader.fill_buf()?.contains(&LF) {
            return Ok(None);
        }
        let mut first_line = String::new();
        reader.fill_buf()?.read_line(&mut first_line)?;

        if let Some((first_word, _)) = first_line.split_once(SPACE_STR) {
            if let Ok(method) = RtspMethod::from_str(first_word) {
                reader.consume(first_word.len());
                return Self::try_read_remaining_from(method, reader.by_ref());
            }
        }
        Err(RtspMessageError::InvalidRtspMessageFormat(format!(
            "message is not rtsp request, first line: {}",
            first_line
        )))
    }
}
