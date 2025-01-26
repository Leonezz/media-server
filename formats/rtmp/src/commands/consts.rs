// @see: 7.2.1. NetConnection Commands
pub mod c2s_command_names {
    pub const CONNECT: &str = "connect";
    pub const CLOSE: &str = "close"; // FIXME - no spec for this action
    pub const CREATE_STREAM: &str = "createStream";
    pub const PLAY: &str = "play";
    pub const PLAY2: &str = "play2";
    pub const DELETE_STREAM: &str = "deleteStream";
    pub const CLOSE_STREAM: &str = "closeStream"; // FIXME - no spec for this action
    pub const RECEIVE_AUDIO: &str = "receiveAudio";
    pub const RECEIVE_VIDEO: &str = "receiveVideo";
    pub const PUBLISH: &str = "publish";
    pub const SEEK: &str = "seek";
    pub const PAUSE: &str = "pause";
}

pub mod s2c_command_names {
    pub const RESULT: &str = "_result";
    pub const ERROR: &str = "_error";
    pub const ON_STATUS: &str = "onStatus";
}

pub const AMF0_ENCODING: u8 = 0;
pub const AMF3_ENCODING: u8 = 3;

pub mod audio_codecs {
    pub const SUPPORT_SND_NONE: u16 = 0x0001; // Raw sound, no compression
    pub const SUPPORT_SND_ADPCM: u16 = 0x0002; // ADPCM compression
    pub const SUPPORT_SND_MP3: u16 = 0x0004; // mp3 compression
    pub const SUPPORT_SND_INTEL: u16 = 0x0008; // Not used
    pub const SUPPORT_SND_UNUSED: u16 = 0x0010; // Not used
    pub const SUPPORT_SND_NELLY8: u16 = 0x0020; // NellyMoser at 8-kHz compression
    pub const SUPPORT_SND_NELLY: u16 = 0x0040; // NellyMoser compression (5, 11, 22, and 44 kHz)
    pub const SUPPORT_SND_G711A: u16 = 0x0080; // G711A sound compression (Flash Media Server only)
    pub const SUPPORT_SND_G711U: u16 = 0x0100; // G711U sound compression (Flash Media Server only)
    pub const SUPPORT_SND_NELLY16: u16 = 0x0200; // NellyMouser at 16-kHz compression
    pub const SUPPORT_SND_AAC: u16 = 0x0400; // Advanced audio coding (AAC) codec
    pub const SUPPORT_SND_SPEEX: u16 = 0x0800; // Speex Audio
    pub const SUPPORT_SND_ALL: u16 = 0x0FFF; // All RTMP-supported audio codecs
}

pub mod video_codecs {
    pub const SUPPORT_VID_UNUSED: u16 = 0x0001; // Obsolete value
    pub const SUPPORT_VID_JPEG: u16 = 0x0002; // Obsolete value
    pub const SUPPORT_VID_SORENSON: u16 = 0x0004; //  Sorenson Flash video 
    pub const SUPPORT_VID_HOMEBREW: u16 = 0x0008; // V1 screen sharing
    pub const SUPPORT_VID_VP6_ON2: u16 = 0x0010; // On2 video (Flash 8+)
    pub const SUPPORT_VID_VP6ALPHA: u16 = 0x0020; // On2 video with alpha 
    pub const SUPPORT_VID_HOMEBREWV: u16 = 0x0040; // Screen sharing version 2 
    pub const SUPPORT_VID_H264: u16 = 0x0080; // H264 video 
    pub const SUPPORT_VID_ALL: u16 = 0x00FF; // All RTMP-supported video
}

pub mod function_flags {
    // Indicates that the client can perform frame-accurate seeks
    pub const SUPPORT_VID_CLIENT_SEEK: u8 = 0x0001;
    // the below are from enhanced rtmp
    // Indicates that the client has support for HDR video.
    // Note: Implies support for colorInfo Object within VideoPacketType.Metadata.
    pub const SUPPORT_VID_CLIENT_HDR: u8 = 0x0002;
    //Indicates that the client has support for VideoPacketType.Metadata.
    // See Metadata Frame section for more detail.
    pub const SUPPORT_VID_CLIENT_VIDEO_PACKET_TYPE_METADATA: u8 = 0x0004;
    // The large-scale tile allows the decoder to extract only an interesting section
    // in a frame without the need to decompress the entire frame.
    // Support for this feature is not required and is assumed to not be
    // implemented by the client unless this property is present and set to true.
    pub const SUPPORT_VID_CLIENT_LARGE_SCALE_TILE: u8 = 0x0008;
}

pub const RESPONSE_STREAM_ID: u8 = 1;
