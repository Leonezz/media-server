#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RtmpServerConfig {
    pub port: u16,
    pub chunk_size: u32,
    pub write_timeout_ms: u64,
    pub read_timeout_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RtmpSessionConfig {
    pub chunk_size: u32,
    pub write_timeout_ms: u64,
    pub read_timeout_ms: u64,
}
