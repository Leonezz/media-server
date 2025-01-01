pub mod api;
mod ext;
pub mod hello;
pub mod httpflv;

pub mod params {
    pub const AUDIO_ONLY_KEY: &str = "audioOnly";
    pub const VIDEO_ONLY_KEY: &str = "videoOnly";
    pub const BACKTRACK_GOP_KEY: &str = "backtraceGopCnt";
}
