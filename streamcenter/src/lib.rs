#![feature(error_generic_member_access)]

use codec_common::{audio::AudioCodecCommon, video::VideoCodecCommon};
use flv_formats::tag::on_meta_data::OnMetaData;
pub mod errors;
pub mod events;
pub mod frame_info;
pub mod gop;
pub mod mix_queue;
pub mod signal;
pub mod stream_center;
pub mod stream_source;

pub fn make_fake_on_meta_data(
    audio_codec: AudioCodecCommon,
    video_codec: VideoCodecCommon,
    height: f64,
    width: f64,
) -> OnMetaData {
    OnMetaData {
        audio_codec_id: Some(audio_codec),
        audio_data_rate: None,
        audio_delay: None,
        audio_sample_rate: None,
        audio_sample_size: None,
        can_seek_to_end: None,
        creation_date: None,
        duration: None,
        file_size: None,
        frame_rate: None,
        height: Some(height),
        stereo: None,
        video_codec_id: Some(video_codec),
        video_data_rate: None,
        width: Some(width),
        audio_track_id_info_map: None,
        video_track_id_info_map: None,
        keyframes: None,
    }
}
