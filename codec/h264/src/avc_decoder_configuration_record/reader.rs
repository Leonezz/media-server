use byteorder::{BigEndian, ReadBytesExt};
use codec_bitstream::reader::BitstreamReader;
use num::ToPrimitive;
use std::io;
use utils::traits::reader::{BitwiseReadFrom, BitwiseReadReaminingFrom, ReadFrom};

use crate::{
    errors::H264CodecError,
    nalu::NalUnit,
    pps::Pps,
    sps::{Sps, chroma_format_idc::ChromaFormatIdc},
    sps_ext::SpsExt,
};

use super::{
    AvcDecoderConfigurationRecord, ParameterSetInAvcDecoderConfigurationRecord, SpsExtRelated,
};

impl<R: io::Read> ReadFrom<R> for SpsExtRelated {
    type Error = H264CodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let byte = reader.read_u8()?;
        let chroma_format_idc: ChromaFormatIdc = (byte & 0b11).try_into()?;
        let reserved_6_bits_1 = (byte >> 2) & 0b111111;
        let byte2 = reader.read_u8()?;
        let bit_depth_luma_minus8 = byte2 & 0b111;
        let reserved_5_bits_1 = (byte2 >> 3) & 0b11111;
        let byte3 = reader.read_u8()?;
        let bit_depth_chroma_minus8 = byte3 & 0b111;
        let _reserved_5_bits_1 = (byte3 >> 3) & 0b11111;
        let num_of_sequence_parameter_ext = reader.read_u8()?;
        let mut sequence_parameter_set_ext =
            Vec::with_capacity(num_of_sequence_parameter_ext as usize);
        for _ in 0..num_of_sequence_parameter_ext {
            let sequence_parameter_set_length = reader.read_u16::<BigEndian>()?;
            let mut bytes = vec![0; sequence_parameter_set_length.to_usize().unwrap()];
            reader.read_exact(&mut bytes)?;
            let nalu = NalUnit::read_from(&mut bytes.as_slice())?;
            let mut bit_reader =
                bitstream_io::BitReader::endian(&nalu.body[..], bitstream_io::BigEndian);
            let sps_ext = SpsExt::read_from(&mut bit_reader)?;
            sequence_parameter_set_ext.push(ParameterSetInAvcDecoderConfigurationRecord {
                sequence_parameter_set_length,
                nalu,
                parameter_set: sps_ext,
            });
        }
        Ok(Self {
            reserved_6_bits_1,
            chroma_format_idc,
            reserved_5_bits_1,
            bit_depth_luma_minus8,
            _reserved_5_bits_1,
            bit_depth_chroma_minus8,
            num_of_sequence_parameter_ext,
            sequence_parameter_set_ext,
        })
    }
}

impl<R: io::Read> ReadFrom<R> for AvcDecoderConfigurationRecord {
    type Error = H264CodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let configuration_version = reader.read_u8()?;
        if configuration_version != 1 {
            return Err(H264CodecError::UnknownAvcDecoderConfigurationVersion(
                configuration_version,
            ));
        }
        let avc_profile_indication = reader.read_u8()?;
        let profile_compatibility = reader.read_u8()?;
        let avc_level_indication = reader.read_u8()?;
        let byte = reader.read_u8()?;
        let length_size_minus_one = byte & 0b11;
        if length_size_minus_one > 3 || length_size_minus_one == 2 {
            return Err(H264CodecError::InvalidLengthSizeMinusOne(
                length_size_minus_one,
            ));
        }
        let reserved_6_bits_1 = (byte >> 2) & 0b111111;
        let byte2 = reader.read_u8()?;
        let num_of_sequence_parameter_sets = byte2 & 0b11111;
        let reserved_3_bits_1 = (byte2 >> 5) & 0b111;
        let mut sequence_parameter_sets =
            Vec::with_capacity(num_of_sequence_parameter_sets.to_usize().unwrap());
        for _ in 0..num_of_sequence_parameter_sets {
            let sequence_parameter_set_length = reader.read_u16::<BigEndian>()?;
            let mut bytes = vec![0; sequence_parameter_set_length.to_usize().unwrap()];
            reader.read_exact(&mut bytes)?;
            let nalu = NalUnit::read_from(&mut bytes.as_slice())?;
            let mut bit_reader =
                bitstream_io::BitReader::endian(&nalu.body[..], bitstream_io::BigEndian);
            let sps = Sps::read_from(&mut bit_reader)?;
            sequence_parameter_sets.push(ParameterSetInAvcDecoderConfigurationRecord {
                sequence_parameter_set_length,
                nalu,
                parameter_set: sps,
            });
        }
        let num_of_picture_parameter_sets = reader.read_u8()?;
        let mut picture_parameter_sets =
            Vec::with_capacity(num_of_picture_parameter_sets.to_usize().unwrap());
        for _ in 0..num_of_picture_parameter_sets {
            let picture_parameter_set_length = reader.read_u16::<BigEndian>()?;
            let mut bytes = vec![0; picture_parameter_set_length.to_usize().unwrap()];
            reader.read_exact(&mut bytes)?;
            let nalu = NalUnit::read_from(&mut bytes.as_slice())?;
            let mut bit_reader = BitstreamReader::new(&nalu.body[..]);
            let pps = Pps::read_remaining_from(ChromaFormatIdc::Chroma420, &mut bit_reader)?; // TODO
            picture_parameter_sets.push(ParameterSetInAvcDecoderConfigurationRecord {
                sequence_parameter_set_length: picture_parameter_set_length,
                nalu,
                parameter_set: pps,
            });
        }
        let sps_ext_related = if [100, 110, 122, 144].contains(&avc_profile_indication) {
            Some(SpsExtRelated::read_from(reader)?)
        } else {
            None
        };
        Ok(Self {
            configuration_version,
            avc_profile_indication,
            profile_compatibility,
            avc_level_indication,
            reserved_6_bits_1,
            length_size_minus_one,
            reserved_3_bits_1,
            num_of_sequence_parameter_sets,
            sequence_parameter_sets,
            num_of_picture_parameter_sets,
            picture_parameter_sets,
            sps_ext_related,
        })
    }
}
