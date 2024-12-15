pub const FMSVER: &str = "FMS/3,0,1,123";
pub const FMS_CAPABILITIES: f64 = 31.0;

pub mod response_code {
    pub const CONNECT_SUCCESS: &str = "NetConnection.Connect.Success";
    pub const DELETE_SUCCESS: &str = "NetStream.DeleteStream.Success";
    pub const PUBLISH_START_SUCCESS: &str = "NetStream.Publish.Start";
}

pub mod response_level {
    pub const STATUS: &str = "status";
}
