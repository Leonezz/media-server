#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RtmpPublishServerConfig {
    pub port: u16,
}
