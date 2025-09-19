use std::{collections::HashMap, iter::zip};

use amf_formats::{AmfComplexObject, Value};
use codec_common::{audio::AudioCodecCommon, video::VideoCodecCommon};

use super::{
    audio_tag_header::SoundFormat,
    enhanced::{ex_audio::ex_audio_header::AudioFourCC, ex_video::ex_video_header::VideoFourCC},
    video_tag_header::CodecID,
};

pub mod reader;
pub mod writer;
#[derive(Debug, Clone)]
pub struct ScriptKeyframeInfo {
    _file_position: f64,
    _time: f64,
}

#[derive(Debug, Clone)]
pub struct OnMetaData {
    /// "audiocodecid", from enhanced rtmp
    /// Audio codec ID used in the file: See AudioTagHeader of the legacy [FLV] specification for available CodecID values.
    /// When [FourCC] is used to signal the codec, this property is set to a FOURCC value.
    /// Note: A FOURCC value is big-endian relative to the underlying ASCII character sequence
    /// (e.g., "Opus" == 0x4F707573 == 1332770163.0).
    pub audio_codec_id: Option<AudioCodecCommon>,
    /// "audiodatarate"
    /// Audio bitrate, in kilobits per second
    pub audio_data_rate: Option<f64>,
    /// "audiodelay"
    /// Delay introduced by the audio codec, in seconds
    pub audio_delay: Option<f64>,
    /// "audiosamplerate"
    /// Frequency at which the audio stream is replayed
    pub audio_sample_rate: Option<f64>,
    /// "audiosamplesize"
    /// Resolution of a single audio sample
    pub audio_sample_size: Option<f64>,
    /// "canSeekToEnd"
    /// Indicating the last video frame is a key frame
    pub can_seek_to_end: Option<bool>,
    /// "creationdate"
    /// Creation date and time
    pub creation_date: Option<String>,
    /// "duration"
    /// Total duration of the file, in seconds
    pub duration: Option<f64>,
    /// "filesize"
    /// Total size of the file, in bytes
    pub file_size: Option<f64>,
    /// "framerate"
    /// Number of frames per second
    pub frame_rate: Option<f64>,
    /// "height"
    /// Height of the video, in pixels
    pub height: Option<f64>,
    /// "stereo"
    /// Indicates stereo audio
    pub stereo: Option<bool>,
    /// "videocodecid"
    /// Video codec ID used in the file: See VideoTagHeader of the legacy [FLV] specification for available CodecID values.
    /// When [FourCC] is used to signal the codec, this property is set to a FOURCC value.
    /// Note: A FOURCC value is big-endian relative to the underlying ASCII character sequence
    /// (e.g., "av01" == 0x61763031 == 1635135537.0).
    pub video_codec_id: Option<VideoCodecCommon>,
    /// "videodatarate"
    /// Video bitrate, in kilobits per second
    pub video_data_rate: Option<f64>,
    /// "width"
    /// Width of the video, in pixels
    pub width: Option<f64>,
    /// "audioTrackIdInfoMap" and "videoTrackIdInfoMap" are way too complicated, sucks
    /// @see Enhanced RTMP v2-2024-10-22-b1 p15
    pub audio_track_id_info_map: Option<HashMap<String, amf_formats::Value>>,
    pub video_track_id_info_map: Option<HashMap<String, amf_formats::Value>>,

    /// @see: http://www.cnblogs.com/musicfans/archive/2012/11/07/2819291.html
    /// "keyframes": {
    ///   "filepositions": [number],
    ///   "times": [number]
    /// }
    pub keyframes: Option<Vec<ScriptKeyframeInfo>>,
}

impl From<HashMap<String, amf_formats::Value>> for OnMetaData {
    fn from(value: HashMap<String, amf_formats::Value>) -> Self {
        let extract_keyframe_info = || match value.extract_object_field("keyframes") {
            None => None,
            Some(pairs) => {
                let mut map: HashMap<String, amf_formats::Value> = HashMap::new();
                for (k, v) in pairs {
                    map.insert(k, v);
                }

                if !map.contains_key("filepositions") || !map.contains_key("times") {
                    return None;
                }

                let filepositions = map.get("filepositions").cloned().unwrap().try_into_values();
                let times = map.get("times").cloned().unwrap().try_into_values();

                if filepositions.is_err() || times.is_err() {
                    return None;
                }

                let filepositions = filepositions.unwrap();
                let times = times.unwrap();

                let mut keyframe_infos = vec![];
                for (pos, time) in zip(filepositions, times) {
                    let pos_num = pos.try_as_f64();
                    let time_num = time.try_as_f64();
                    if pos_num.is_none() || time_num.is_none() {
                        return None;
                    }
                    keyframe_infos.push(ScriptKeyframeInfo {
                        _file_position: pos_num.unwrap(),
                        _time: time_num.unwrap(),
                    });
                }
                Some(keyframe_infos)
            }
        };

        Self {
            audio_codec_id: value.extract_number_field("audiocodecid").map(|v| {
                let four_cc_codec: Result<AudioFourCC, _> = (v as u32).try_into();
                if let Ok(codec) = four_cc_codec {
                    return codec.into();
                }
                let legacy_codec: Result<SoundFormat, _> = (v as u8).try_into();
                legacy_codec.unwrap_or(SoundFormat::AAC).into()
            }),
            audio_data_rate: value.extract_number_field("audiodatarate"),
            audio_delay: value.extract_number_field("audiodelay"),
            audio_sample_rate: value.extract_number_field("audiosamplerate"),
            audio_sample_size: value.extract_number_field("audiosamplesize"),
            can_seek_to_end: value.extract_bool_field("canSeekToEnd"),
            creation_date: value.extract_string_field("creationdate"),
            duration: value.extract_number_field("duration"),
            file_size: value.extract_number_field("filesize"),
            frame_rate: value.extract_number_field("framerate"),
            height: value.extract_number_field("height"),
            stereo: value.extract_bool_field("stereo"),
            video_codec_id: value.extract_number_field("videocodecid").map(|v| {
                let four_cc_codec: Result<VideoFourCC, _> = (v as u32).try_into();
                if let Ok(codec) = four_cc_codec {
                    return codec.into();
                }
                let legacy_codec: Result<CodecID, _> = (v as u8).try_into();
                legacy_codec.unwrap_or(CodecID::AVC).into()
            }),
            video_data_rate: value.extract_number_field("videodatarate"),
            width: value.extract_number_field("width"),
            audio_track_id_info_map: value.extract_object_field("audioTrackIdInfoMap").map(
                |pairs| {
                    let mut map: HashMap<String, amf_formats::Value> = HashMap::new();
                    for (k, v) in pairs {
                        map.insert(k, v);
                    }
                    map
                },
            ),
            video_track_id_info_map: value.extract_object_field("videoTrackIdInfoMap").map(
                |pairs| {
                    let mut map: HashMap<String, amf_formats::Value> = HashMap::new();
                    for (k, v) in pairs {
                        map.insert(k, v);
                    }
                    map
                },
            ),
            keyframes: extract_keyframe_info(),
        }
    }
}

impl From<&OnMetaData> for Vec<(String, amf_formats::amf0::Value)> {
    fn from(value: &OnMetaData) -> Self {
        let mut result = vec![];
        if let Some(audio_codec) = value.audio_codec_id {
            let audio_codec_four_cc: AudioFourCC =
                audio_codec.try_into().unwrap_or(AudioFourCC::AAC);
            let audio_codec_number: u32 = audio_codec_four_cc.into();
            result.push((
                "audiocodecid".to_string(),
                amf_formats::amf0::number(audio_codec_number),
            ));
        }
        if let Some(audio_data_rata) = value.audio_data_rate {
            result.push((
                "audiodatarate".to_string(),
                amf_formats::amf0::number(audio_data_rata),
            ));
        }
        if let Some(audio_delay) = value.audio_delay {
            result.push((
                "audiodelay".to_string(),
                amf_formats::amf0::number(audio_delay),
            ));
        }
        if let Some(audio_sample_rate) = value.audio_sample_rate {
            result.push((
                "audiosamplerate".to_string(),
                amf_formats::amf0::number(audio_sample_rate),
            ));
        }
        if let Some(audio_sample_size) = value.audio_sample_size {
            result.push((
                "audiosamplesize".to_string(),
                amf_formats::amf0::number(audio_sample_size),
            ));
        }
        if let Some(can_seek_to_end) = value.can_seek_to_end {
            result.push((
                "canSeekToEnd".to_string(),
                amf_formats::amf0::bool(can_seek_to_end),
            ));
        }
        if let Some(creation_date) = &value.creation_date {
            result.push((
                "creationdate".to_string(),
                amf_formats::amf0::string(creation_date),
            ));
        }
        if let Some(duration) = value.duration {
            result.push(("duration".to_string(), amf_formats::amf0::number(duration)));
        }
        if let Some(file_size) = value.file_size {
            result.push(("filesize".to_string(), amf_formats::amf0::number(file_size)));
        }
        if let Some(frame_rate) = value.frame_rate {
            result.push((
                "framerate".to_string(),
                amf_formats::amf0::number(frame_rate),
            ));
        }
        if let Some(height) = value.height {
            result.push(("height".to_string(), amf_formats::amf0::number(height)));
        }
        if let Some(stereo) = value.stereo {
            result.push(("stereo".to_string(), amf_formats::amf0::bool(stereo)));
        }
        if let Some(video_codec) = value.video_codec_id {
            let video_four_cc: VideoFourCC = video_codec.try_into().unwrap_or(VideoFourCC::AVC);
            let video_codec_num: u32 = video_four_cc.into();
            result.push((
                "videocodecid".to_string(),
                amf_formats::amf0::number(video_codec_num),
            ));
        }
        if let Some(video_data_rate) = value.video_data_rate {
            result.push((
                "videodatarate".to_string(),
                amf_formats::amf0::number(video_data_rate),
            ));
        }
        if let Some(width) = value.width {
            result.push(("width".to_string(), amf_formats::amf0::number(width)));
        }
        if let Some(audio_track_id_info_map) = &value.audio_track_id_info_map {
            result.push((
                "audioTrackIdInfoMap".to_string(),
                amf_formats::amf0::object(audio_track_id_info_map.clone().into_iter().map(
                    |(k, v)| {
                        (
                            k,
                            match v {
                                Value::AMF0Value(value) => value,
                                Value::AMF3Value(value) => amf_formats::amf0::Value::AVMPlus(value),
                            },
                        )
                    },
                )),
            ));
        }
        if let Some(video_track_id_info_map) = &value.video_track_id_info_map {
            result.push((
                "videoTrackIdInfoMap".to_string(),
                amf_formats::amf0::object(video_track_id_info_map.clone().into_iter().map(
                    |(k, v)| {
                        (
                            k,
                            match v {
                                Value::AMF0Value(value) => value,
                                Value::AMF3Value(value) => amf_formats::amf0::Value::AVMPlus(value),
                            },
                        )
                    },
                )),
            ));
        }
        if let Some(key_frames) = &value.keyframes {
            result.push((
                "keyframes".to_string(),
                amf_formats::amf0::Value::Object {
                    name: None,
                    entries: vec![
                        (
                            "filepositions".to_string(),
                            amf_formats::amf0::Value::StrictArray(
                                key_frames
                                    .iter()
                                    .map(|item| amf_formats::amf0::number(item._file_position))
                                    .collect(),
                            ),
                        ),
                        (
                            "times".to_string(),
                            amf_formats::amf0::Value::StrictArray(
                                key_frames
                                    .iter()
                                    .map(|item| amf_formats::amf0::number(item._time))
                                    .collect(),
                            ),
                        ),
                    ],
                },
            ));
        }

        result
    }
}
