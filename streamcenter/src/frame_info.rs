use tokio_util::bytes::BytesMut;

#[derive(Debug, Default, Clone, Copy)]
pub struct MediaMessageRuntimeStat {
    pub read_time_ns: u128,
    pub session_process_time_ns: u128,
    pub publish_stream_source_time_ns: u128,
    pub stream_source_received_time_ns: u128,
    pub stream_source_parse_time_ns: u128,
    pub play_time_ns: u128,
}

#[derive(Debug, Clone)]
pub struct VideoMeta {
    pub pts: u64,

    pub runtime_stat: MediaMessageRuntimeStat,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ScriptMeta {
    pub pts: u64,

    pub runtime_stat: MediaMessageRuntimeStat,
}

#[derive(Debug, Clone)]
pub struct AudioMeta {
    pub pts: u64,

    pub runtime_stat: MediaMessageRuntimeStat,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AggregateMeta {
    pub pts: u64,
    pub read_time_ns: u128,
    pub session_process_time_ns: u128,
    pub publish_stream_source_time_ns: u128,
}

#[derive(Debug, Clone)]
pub enum ChunkFrameData {
    Video { meta: VideoMeta, payload: BytesMut },
    Audio { meta: AudioMeta, payload: BytesMut },
    Aggregate { meta: AggregateMeta, data: BytesMut },
    Script { meta: ScriptMeta, payload: BytesMut },
}

impl ChunkFrameData {
    pub fn log_runtime_stat(&self) {
        match self {
            ChunkFrameData::Video { meta, payload: _ } => {
                tracing::info!("video message stat: {:?}", meta.runtime_stat);
            }
            ChunkFrameData::Audio { meta, payload: _ } => {
                tracing::info!("audio message stat: {:?}", meta.runtime_stat);
            }
            ChunkFrameData::Script { meta, payload: _ } => {
                tracing::info!("meta message stat: {:?}", meta.runtime_stat);
            }
            _ => {}
        }
    }
}

impl ChunkFrameData {
    #[inline]
    pub fn is_video(&self) -> bool {
        match self {
            ChunkFrameData::Video {
                meta: _,
                payload: _,
            } => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_audio(&self) -> bool {
        match self {
            ChunkFrameData::Audio {
                meta: _,
                payload: _,
            } => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_script(&self) -> bool {
        match self {
            ChunkFrameData::Script {
                meta: _,
                payload: _,
            } => true,
            _ => false,
        }
    }
}
