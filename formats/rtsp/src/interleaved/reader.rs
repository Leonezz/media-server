use crate::errors::RTSPMessageError;
use byteorder::ReadBytesExt;
use tokio_util::bytes::{Buf, Bytes};
use utils::traits::reader::{TryReadFrom, TryReadRemainingFrom};

use super::{DOLLAR_SIGN, RtspInterleavedPacket};

impl<R: AsRef<[u8]>> TryReadRemainingFrom<u8, R> for RtspInterleavedPacket {
    type Error = RTSPMessageError;
    fn try_read_remaining_from(
        header: u8,
        reader: &mut std::io::Cursor<R>,
    ) -> Result<Option<Self>, Self::Error> {
        if header != DOLLAR_SIGN {
            return Err(RTSPMessageError::InvalidInterleavedSign(header));
        }

        if reader.remaining() < 3 {
            return Ok(None);
        }

        let channel_id = reader.read_u8()?;
        let data_length = reader.read_u16::<byteorder::BigEndian>()? as usize;
        if reader.remaining() < data_length {
            return Ok(None);
        }

        let mut data = vec![0; data_length];
        reader.copy_to_slice(&mut data);
        Ok(Some(Self {
            channel_id,
            payload: Bytes::from(data),
        }))
    }
}

impl<R: AsRef<[u8]>> TryReadFrom<R> for RtspInterleavedPacket {
    type Error = RTSPMessageError;
    fn try_read_from(reader: &mut std::io::Cursor<R>) -> Result<Option<Self>, Self::Error> {
        if reader.remaining() < 4 {
            return Ok(None);
        }

        let sign = reader.read_u8()?;
        if sign != DOLLAR_SIGN {
            return Err(RTSPMessageError::InvalidInterleavedSign(sign));
        }
        Self::try_read_remaining_from(sign, reader)
    }
}
