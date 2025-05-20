use bitstream_io::BitRead;
use utils::traits::reader::{BitwiseReadFrom, BitwiseReadReaminingFrom};

use crate::errors::AACCodecError;

use super::{ErrorProtectionSpecificConfig, PredefinedSet, PredefinedSetClass};

impl<R: BitRead> BitwiseReadReaminingFrom<(u8, u8), R> for PredefinedSetClass {
    type Error = AACCodecError;
    fn read_remaining_from(header: (u8, u8), reader: &mut R) -> Result<Self, Self::Error> {
        let (interleave_type, number_of_concatenated_frame) = header;
        let length_escape = reader.read_bit()?;
        let rate_escape = reader.read_bit()?;
        let crclen_escape = reader.read_bit()?;
        let concatenate_flag = if number_of_concatenated_frame != 1 {
            Some(reader.read_bit()?)
        } else {
            None
        };
        let fec_type = reader.read::<2, u8>()?;
        let termination_switch = if fec_type == 0 {
            Some(reader.read_bit()?)
        } else {
            None
        };
        let interleave_switch = if interleave_type == 2 {
            Some(reader.read::<2, u8>()?)
        } else {
            None
        };
        let class_optional = reader.read_bit()?;
        let number_of_bits_for_length = if length_escape {
            Some(reader.read::<4, u8>()?)
        } else {
            None
        };
        let class_length = if !length_escape {
            Some(reader.read::<16, u16>()?)
        } else {
            None
        };
        let class_rate_7bits = if !rate_escape && fec_type != 0 {
            Some(reader.read::<7, u8>()?)
        } else {
            None
        };
        let class_rate_5bits = if !rate_escape && fec_type == 0 {
            Some(reader.read::<5, u8>()?)
        } else {
            None
        };
        let class_crclen = if !crclen_escape {
            Some(reader.read::<5, u8>()?)
        } else {
            None
        };
        Ok(Self {
            length_escape,
            rate_escape,
            crclen_escape,
            concatenate_flag,
            fec_type,
            termination_switch,
            interleave_switch,
            class_optional,
            number_of_bits_for_length,
            class_length,
            class_rate_7bits,
            class_rate_5bits,
            class_crclen,
        })
    }
}

impl<R: BitRead> BitwiseReadReaminingFrom<(u8, u8), R> for PredefinedSet {
    type Error = AACCodecError;
    fn read_remaining_from(header: (u8, u8), reader: &mut R) -> Result<Self, Self::Error> {
        let number_of_class = reader.read::<6, u8>()?;
        let class = {
            let mut class = vec![];
            for _ in 0..number_of_class {
                class.push(PredefinedSetClass::read_remaining_from(header, reader)?);
            }
            class
        };
        let class_reordered_output = reader.read_bit()?;
        let class_output_order = if class_reordered_output {
            let mut output_order = vec![];
            for _ in 0..number_of_class {
                output_order.push(reader.read::<6, u8>()?);
            }
            Some(output_order)
        } else {
            None
        };
        Ok(Self {
            number_of_class,
            class,
            class_reordered_output,
            class_output_order,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for ErrorProtectionSpecificConfig {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let number_of_predefined_set = reader.read::<8, u8>()?;
        let interleave_type = reader.read::<2, u8>()?;
        let bit_stuffing = reader.read::<3, u8>()?;
        let number_of_concatenated_frame = reader.read::<3, u8>()?;
        let predefined_sets = {
            let mut sets = vec![];
            for _ in 0..number_of_predefined_set {
                sets.push(PredefinedSet::read_remaining_from(
                    (interleave_type, number_of_concatenated_frame),
                    reader,
                )?);
            }
            sets
        };
        let header_protection = reader.read_bit()?;
        let (header_rate, header_crclen) = if header_protection {
            (Some(reader.read::<5, u8>()?), Some(reader.read::<5, u8>()?))
        } else {
            (None, None)
        };
        Ok(Self {
            number_of_predefined_set,
            interleave_type,
            bit_stuffing,
            number_of_concatenated_frame,
            predefined_sets,
            header_protection,
            header_rate,
            header_crclen,
        })
    }
}
