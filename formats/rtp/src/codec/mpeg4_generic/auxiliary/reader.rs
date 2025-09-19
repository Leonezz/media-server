use std::io;

use bitstream_io::{BigEndian, BitRead2, BitReader};
use num::ToPrimitive;
use tokio_util::bytes::Bytes;
use utils::traits::reader::ReadRemainingFrom;

use crate::codec::mpeg4_generic::{errors::RtpMpeg4Error, parameters::RtpMpeg4Fmtp};

use super::AuxiliaryData;

impl<R: io::Read> ReadRemainingFrom<&RtpMpeg4Fmtp, R> for AuxiliaryData {
    type Error = RtpMpeg4Error;
    fn read_remaining_from(header: &RtpMpeg4Fmtp, reader: &mut R) -> Result<Self, Self::Error> {
        let mut reader = BitReader::endian(reader, BigEndian);
        let auxiliary_data_size_length = header.auxiliary_data_size_length.unwrap_or(0);
        if auxiliary_data_size_length == 0 {
            return Err(RtpMpeg4Error::AuxiliaryDataEmpty);
        }
        let auxiliary_data_size: u64 = BitRead2::read(
            &mut reader,
            auxiliary_data_size_length
                .to_u32()
                .expect("integer overflow u32"),
        )?;
        let bytes_cnt = auxiliary_data_size / 8;
        let remaining_bits = auxiliary_data_size % 8;
        let mut bytes = vec![0; bytes_cnt as usize + if remaining_bits > 0 { 1 } else { 0 }];
        for i in 0..bytes_cnt {
            bytes[i as usize] = reader.read_in::<8, _>()?;
        }
        if remaining_bits > 0 {
            bytes[bytes_cnt as usize] = BitRead2::read(
                &mut reader,
                remaining_bits.to_u32().expect("integer overflow u32"),
            )?;
        }

        BitRead2::byte_align(&mut reader);

        Ok(Self {
            auxiliary_data_size,
            data: Bytes::from_owner(bytes),
        })
    }
}
