use std::io;

use crate::consts::common::{CR, CRLF_STR, LF};

pub struct TextReader<R: io::BufRead> {
    inner: R,
}

#[allow(unused)]
impl<R: io::BufRead> TextReader<R> {
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn read_line(&mut self) -> io::Result<Option<String>> {
        let mut line = String::new();
        let len = self.inner.read_line(&mut line)?;
        if len == 0 {
            return Ok(None);
        }
        Ok(Some(line))
    }

    pub fn try_read_line(&mut self) -> io::Result<Option<String>> {
        if !self.inner.fill_buf()?.contains(&LF) {
            return Ok(None);
        }
        self.read_line()
    }

    pub fn expect(&mut self, expected: &[u8]) -> io::Result<bool> {
        let mut real = vec![0; expected.len()];
        self.inner.read_exact(&mut real)?;
        Ok(real.eq(expected))
    }

    pub fn read_all(&mut self) -> io::Result<String> {
        let mut all = String::new();
        self.inner.read_to_string(&mut all)?;
        Ok(all)
    }

    pub fn skip_all(&mut self, skip: &[u8]) -> io::Result<usize> {
        let mut skipped = 0;
        loop {
            let buf = self.inner.fill_buf()?;
            if buf.starts_with(skip) {
                self.inner.consume(skip.len());
                skipped += skip.len();
                continue;
            }
            break;
        }
        Ok(skipped)
    }

    pub fn skip_empty_lines(&mut self) -> io::Result<usize> {
        let mut skipped = 0;
        loop {
            let mut skipped_this_round = 0;
            skipped_this_round += self.skip_all(CRLF_STR.as_bytes())?;
            skipped_this_round += self.skip_all(&[CR])?;
            skipped_this_round += self.skip_all(&[LF])?;
            if skipped_this_round == 0 {
                break;
            }
            skipped += skipped_this_round;
        }
        Ok(skipped)
    }

    pub fn read_exact(&mut self, len: usize) -> io::Result<String> {
        let mut result = vec![0_u8; len];
        self.inner.read_exact(&mut result)?;
        Ok(String::from_utf8_lossy(&result).to_string())
    }

    pub fn try_read_exact(&mut self, len: usize) -> io::Result<Option<String>> {
        if len == 0 || self.inner.fill_buf()?.len() < len {
            return Ok(None);
        }
        self.read_exact(len).map(Some)
    }
}
