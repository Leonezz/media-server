#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HttpFlvServerConfig {
    pub ip: String,
    pub port: u16,
    pub chunk_size: u32,
    pub write_timeout_ms: u64,
    pub read_timeout_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HttpFlvSessionConfig {
    pub chunk_size: u32,
    pub write_timeout_ms: u64,
    pub read_timeout_ms: u64,
}
