pub const FMSVER: &str = "FMS/3,0,1,123";
pub const FMS_CAPABILITIES: f64 = 31.0;

pub mod response_code {
    pub const NET_CONNECTION_CONNECT_SUCCESS: &str = "NetConnection.Connect.Success";
    pub const NET_STREAM_DELETE_SUCCESS: &str = "NetStream.DeleteStream.Success";
    pub const NET_STREAM_PUBLISH_START_SUCCESS: &str = "NetStream.Publish.Start";
    pub const NET_STREAM_PLAY_START: &str = "NetStream.Play.Start";
    pub const NET_STREAM_PLAY_RESET: &str = "NetStream.Play.Reset";
    pub const NET_STREAM_PLAY_NOT_FOUND: &str = "NetStream.Play.StreamNotFound";
}

pub mod response_level {
    pub const STATUS: &str = "status";
    pub const WARNING: &str = "warning";
    pub const ERROR: &str = "error";
}
