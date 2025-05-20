use std::io;

use bitstream_io::BitRead;
use codec_bitstream::reader::BitstreamReader;

use crate::errors::H264CodecError;

pub trait RbspReadExt {
    type Error;
    fn more_rbsp_data(&mut self) -> Result<bool, Self::Error>;
}

impl<'a> RbspReadExt for BitstreamReader<'a> {
    type Error = H264CodecError;
    fn more_rbsp_data(&mut self) -> Result<bool, Self::Error> {
        let remaining = self.remaining_bits();
        if remaining > 8 {
            return Ok(true);
        }
        if remaining == 0 {
            return Ok(false);
        }
        let mut temp_reader = self.reader().clone();
        temp_reader.skip(1)?;
        match temp_reader.read_unary::<1>() {
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
            Err(e) => Err(e.into()),
            Ok(_) => Ok(true),
        }
    }
}
