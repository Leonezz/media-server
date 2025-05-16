use bitstream_io::{BitRead, BitWrite};
use num::{Signed, ToPrimitive};

use crate::errors::{H264CodecError, H264CodecResult};

fn read_code_num<R: BitRead>(reader: &mut R) -> H264CodecResult<u64> {
    let leading_zero_bits = reader.read_unary::<1>()?;
    if leading_zero_bits > 31 {
        return Err(H264CodecError::InvalidExpGolombCode(format!(
            "got more then 31 leading zero bits: {}",
            leading_zero_bits
        )));
    }

    let remaining = reader.read_var(leading_zero_bits)?;
    let result = 2_u64
        .checked_pow(leading_zero_bits)
        .and_then(|v| v.checked_sub(1))
        .and_then(|v| v.checked_add(remaining))
        .unwrap();
    Ok(result)
}

fn find_leading_zero_bits_count<T: Into<u64>>(value: T) -> H264CodecResult<u32> {
    let value: u64 = value.into();
    if value
        > 2_u64
            .checked_pow(31)
            .and_then(|v| v.checked_sub(1))
            .and_then(|v| v.checked_mul(2))
            .unwrap()
    {
        return Err(H264CodecError::InvalidExpGolombCode(format!(
            "value too big to encode as Exp-Golomb: {}",
            value
        )));
    }
    for cnt in 0..=31 {
        let lower_bound = 2_u64
            .checked_pow(cnt)
            .and_then(|v| v.checked_sub(1))
            .unwrap();
        let higher_bound = lower_bound.checked_mul(2).unwrap();
        if value >= lower_bound && value <= higher_bound {
            return Ok(cnt);
        }
    }
    unreachable!()
}

fn write_code_num<W: BitWrite, T: Into<u64> + Copy>(
    writer: &mut W,
    value: T,
) -> H264CodecResult<()> {
    let leading_zero_bits = find_leading_zero_bits_count(value)?;
    let remaining: u64 = value
        .into()
        .checked_add(1)
        .and_then(|v| v.checked_sub(2_u64.checked_pow(leading_zero_bits).unwrap()))
        .unwrap();
    writer.write_var(leading_zero_bits, 0)?;
    writer.write_bit(true)?;
    writer.write_var(leading_zero_bits, remaining)?;
    Ok(())
}

pub fn read_ue<R: BitRead>(reader: &mut R) -> H264CodecResult<u64> {
    read_code_num(reader)
}

pub fn write_ue<W: BitWrite, T: Into<u64> + Copy>(writer: &mut W, value: T) -> H264CodecResult<()> {
    write_code_num(writer, value)
}

pub fn read_se<R: BitRead>(reader: &mut R) -> H264CodecResult<i64> {
    let code_num = read_code_num(reader)?;
    let negetive = code_num & 0b1 != 0b1;
    Ok(code_num
        .div_ceil(2)
        .to_i64()
        .and_then(|v| v.checked_mul(if negetive { -1 } else { 1 }))
        .unwrap())
}

pub fn write_se<W: BitWrite, T: Signed + ToPrimitive>(
    writer: &mut W,
    value: T,
) -> H264CodecResult<()> {
    let code_num = value
        .abs()
        .to_u64()
        .unwrap()
        .checked_mul(2)
        .and_then(|v| {
            if value.is_positive() {
                v.checked_sub(1)
            } else {
                Some(v)
            }
        })
        .unwrap();
    write_code_num(writer, code_num)
}

pub fn read_me<R: BitRead>(
    reader: &mut R,
    chroma_array_type: u8,
    macroblock_prediction_mode: u8,
) -> H264CodecResult<u64> {
    todo!()
}

pub fn write_me<W: BitWrite>(
    writer: &mut W,
    chroma_array_type: u8,
    macroblock_prediction_mode: u8,
    value: u64,
) -> H264CodecResult<()> {
    todo!()
}

pub fn read_te<R: BitRead>(reader: &mut R, is_ue: bool) -> H264CodecResult<u64> {
    if is_ue {
        return read_ue(reader);
    }
    // codeNum = !read_bits(1)
    if reader.read_bit()? { Ok(0) } else { Ok(1) }
}

pub fn write_te<W: BitWrite>(writer: &mut W, value: u64, is_ue: bool) -> H264CodecResult<()> {
    if is_ue {
        return write_ue(writer, value);
    }
    writer.write_bit(value == 0)?;
    Ok(())
}
