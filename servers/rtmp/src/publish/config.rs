#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RtmpPublishServerConfig {
    pub port: u16,
    pub chunk_size: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RtmpSessionConfig {
    pub chunk_size: u32,
}
