use std::{fmt::Display, str::FromStr};

use bitstream_io::BitRead;
use tokio_util::bytes::Bytes;
use utils::traits::reader::BitwiseReadFrom;

use crate::codec::mpeg4_generic::{errors::RtpMpeg4Error, parameters::Mode};

use super::RtpMpeg4Fmtp;

impl FromStr for RtpMpeg4Fmtp {
    type Err = RtpMpeg4Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = Self::default();
        fn parse_from_str<E: Display, T: FromStr<Err = E>>(
            key: &str,
            value: &str,
        ) -> Result<T, RtpMpeg4Error> {
            value.parse().map_err(|err| {
                RtpMpeg4Error::ParseFromFmtpFailed(format!(
                    "parse {} from {} failed: {}",
                    key, value, err
                ))
            })
        }
        for key_value in s.split(';') {
            let (key, value) = key_value
                .trim()
                .split_once('=')
                .ok_or_else(|| RtpMpeg4Error::ParseFromFmtpFailed(s.to_owned()))?;
            match key.to_lowercase().as_str() {
                "profile-level-id" => {
                    result.profile_level_id = parse_from_str(key, value)?;
                }
                "config" => {
                    result.config = Bytes::from_owner(utils::bytes::hex_to_bytes(value).ok_or(
                        RtpMpeg4Error::ParseFromFmtpFailed(format!(
                            "parse fmtp.config from hex string failed: {}",
                            value
                        )),
                    )?);
                }
                "mode" => {
                    result.mode = value.parse()?;
                }
                "objecttype" => {
                    result.object_type = Some(parse_from_str(key, value)?);
                }
                "constantsize" => {
                    result.constant_size = Some(parse_from_str(key, value)?);
                }
                "constantduration" => {
                    result.constant_duration = Some(parse_from_str(key, value)?);
                }
                "maxdisplacement" => {
                    result.max_displacement = Some(parse_from_str(key, value)?);
                }
                "de-interleavebuffersize" => {
                    result.de_interleave_buffer_size = Some(parse_from_str(key, value)?);
                }
                "sizelength" => {
                    result.size_length = Some(parse_from_str(key, value)?);
                }
                "indexlength" => {
                    result.index_length = Some(parse_from_str(key, value)?);
                }
                "indexdeltalength" => {
                    result.index_delta_length = Some(parse_from_str(key, value)?);
                }
                "ctsdeltalength" => {
                    result.cts_delta_length = Some(parse_from_str(key, value)?);
                }
                "dtsdeltalength" => {
                    result.dts_delta_length = Some(parse_from_str(key, value)?);
                }
                "randomaccessindication" => {
                    result.random_access_indication = Some(parse_from_str(key, value)?);
                }
                "streamstateindication" => {
                    result.stream_state_indication = Some(parse_from_str(key, value)?);
                }
                "auxiliarydatasizelength" => {
                    result.auxiliary_data_size_length = Some(parse_from_str(key, value)?);
                }
                _ => {}
            }
        }
        result.validate()?;
        if result.mode == Mode::Generic {
            return Err(RtpMpeg4Error::InvalidMode(
                "generic mode is not supported".to_owned(),
            ));
        }
        let mut reader = codec_bitstream::reader::BitstreamReader::new(&result.config);
        result.aac_audio_specific_config = Some(
            codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig::read_from(
                reader.by_ref(),
            )?,
        );
        Ok(result)
    }
}
