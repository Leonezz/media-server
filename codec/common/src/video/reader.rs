use std::io::{self, Read};

use byteorder::ReadBytesExt;
use num::ToPrimitive;
use tokio_util::bytes::{Buf, Bytes};
use utils::traits::reader::ReadFrom;

use crate::errors::{CodecCommonError, CodecCommonResult};

use super::{VideoCodecCommon, VideoFrameUnit};

pub fn split_nalu_by_start_codes(bytes: &[u8]) -> CodecCommonResult<Vec<Bytes>> {
    let mut nalu_bytes = Vec::new();
    let mut last_pos = 0;
    for i in 0..bytes.len() {
        if i < 2 {
            continue;
        }
        if bytes[i - 2..=i] == [0, 0, 0] {
            last_pos += 1;
            continue;
        }
        if bytes[i - 2..=i] == [0, 0, 1] {
            if last_pos != i - 2 {
                nalu_bytes.push(Bytes::copy_from_slice(&bytes[last_pos..i - 2]));
            }
            last_pos = i + 1;
        }
    }
    Ok(nalu_bytes)
}

pub fn split_nalu_by_nalu_size_length(
    bytes: &[u8],
    nalu_size_length: u8,
) -> CodecCommonResult<Vec<Bytes>> {
    if nalu_size_length != 1 && nalu_size_length != 2 && nalu_size_length != 4 {
        return Err(CodecCommonError::InvalidNaluSizeLengthMinueOne(
            nalu_size_length - 1,
        ));
    }

    let nalu_size_length = nalu_size_length.to_usize().unwrap();
    let mut nalu_bytes = Vec::new();
    let mut cursor = io::Cursor::new(bytes);
    while cursor.has_remaining() {
        let nalu_size = match nalu_size_length {
            1 => cursor.read_u8()?.to_usize().unwrap(),
            2 => cursor
                .read_u16::<byteorder::BigEndian>()?
                .to_usize()
                .unwrap(),
            4 => cursor
                .read_u32::<byteorder::BigEndian>()?
                .to_usize()
                .unwrap(),
            _ => unreachable!(),
        };
        let mut buf = vec![0; nalu_size as usize];
        cursor.read_exact(&mut buf)?;
        nalu_bytes.push(Bytes::from_owner(buf));
    }
    Ok(nalu_bytes)
}

pub fn parse_to_nalu_bytes(
    bytes: &[u8],
    nalu_size_length: Option<u8>,
) -> CodecCommonResult<Vec<Bytes>> {
    if let Some(nalu_size_length) = nalu_size_length {
        split_nalu_by_nalu_size_length(bytes, nalu_size_length)
    } else {
        split_nalu_by_start_codes(bytes)
    }
}

pub fn parse_to_avc_nal_units(
    bytes: Vec<Bytes>,
) -> CodecCommonResult<Vec<codec_h264::nalu::NalUnit>> {
    let mut nal_units = Vec::new();
    bytes.into_iter().try_for_each(|nalu_bytes| {
        nal_units.push(codec_h264::nalu::NalUnit::read_from(
            &mut nalu_bytes.reader(),
        )?);
        Ok::<(), CodecCommonError>(())
    })?;
    Ok(nal_units)
}

pub fn parse_to_nal_units(
    bytes: &[u8],
    codec_id: VideoCodecCommon,
    nalu_size_length: Option<u8>,
) -> CodecCommonResult<VideoFrameUnit> {
    match codec_id {
        VideoCodecCommon::AVC => {
            let nalu_bytes = parse_to_nalu_bytes(bytes, nalu_size_length)?;
            let nal_units = parse_to_avc_nal_units(nalu_bytes)?;
            Ok(VideoFrameUnit::H264 { nal_units })
        }
        _ => unimplemented!(),
    }
}
