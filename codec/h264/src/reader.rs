use std::io;

use crate::{errors::H264CodecError, nalu::NalUnit, nalu_header::NaluHeader};
use byteorder::ReadBytesExt;
use tokio_util::bytes::Bytes;
use utils::traits::reader::{ReadExactFrom, ReadFrom, ReadRemainingFrom};

/// read all the remaining bytes as body, the header was read ahead
impl<R: io::Read> ReadRemainingFrom<NaluHeader, R> for NalUnit {
    type Error = H264CodecError;
    fn read_remaining_from(header: NaluHeader, reader: &mut R) -> Result<Self, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(Self {
            header,
            body: Bytes::from(bytes),
        })
    }
}

/// read extract bytes as body, the header was read ahead
impl<R: io::Read> ReadRemainingFrom<(NaluHeader, usize), R> for NalUnit {
    type Error = H264CodecError;
    fn read_remaining_from(
        header: (NaluHeader, usize),
        reader: &mut R,
    ) -> Result<Self, Self::Error> {
        let (header, body_size) = header;
        let mut bytes = vec![0; body_size];
        reader.read_exact(&mut bytes)?;
        Ok(Self {
            header,
            body: Bytes::from(bytes),
        })
    }
}

/// read all from reader, including the header
/// assumes all bytes from the reader consists the nalu
impl<R: io::Read> ReadFrom<R> for NalUnit {
    type Error = H264CodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let first_byte = reader.read_u8()?;
        let header: NaluHeader = first_byte.try_into()?;
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(Self {
            header,
            body: Bytes::from(bytes),
        })
    }
}

/// read extract bytes to consist a nalu
impl<R: io::Read> ReadExactFrom<R> for NalUnit {
    type Error = H264CodecError;
    fn read_exact_from(length: usize, reader: &mut R) -> Result<Self, Self::Error> {
        let header: NaluHeader = reader.read_u8()?.try_into()?;
        Self::read_remaining_from((header, length - 1), reader)
    }
}
