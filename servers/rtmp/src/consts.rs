pub const FMSVER: &str = "FMS/3,0,1,123";
pub const FMS_CAPABILITIES: f64 = 31.0;

pub mod response_code {
    pub const NET_CONNECTION_CONNECT_SUCCESS: &str = "NetConnection.Connect.Success";
    pub const NET_STREAM_DELETE_SUCCESS: &str = "NetStream.DeleteStream.Success";
    pub const NET_STREAM_PUBLISH_START_SUCCESS: &str = "NetStream.Publish.Start";
    pub const NET_STREAM_PLAY_START: &str = "NetStream.Play.Start";
    pub const NET_STREAM_PLAY_RESET: &str = "NetStream.Play.Reset";
    pub const NET_STREAM_PLAY_NOT_FOUND: &str = "NetStream.Play.StreamNotFound";

    // The NetConnection.call() method was not able to invoke the server-side method or command.
    // level: error
    pub const NET_CONNECTION_CALL_FAILED: &str = "NetConnection.Call.Failed";
    // The application has been shut down (for example,
    // if the application is out of memory resources and must shut down to prevent the server from crashing)
    // or the server has shut down.
    // level: error
    pub const NET_CONNECTION_CONNECT_APP_SHUTDOWN: &str = "NetConnection.Connect.AppShutdown";
    // The connection attempt failed.
    // level: error
    pub const NET_CONNECTION_CONNECT_FAILED: &str = "NetConnection.Connect.Failed";
    // The client does not have permission to connect to the application.
    // level: error
    pub const NET_CONNECTION_CONNECT_REJECTED: &str = "NetConnection.Connect.Rejected";
    // The connection was closed successfully.
    // level: status
    pub const NET_CONNECTION_CONNECT_CLOSED: &str = "NetConnection.Connect.Closed";
    // The proxy server is not responding. See the ProxyStream class.
    // level: error
    pub const NET_CONNECTION_PROXY_NOT_RESPONDING: &str = "NetConnection.Proxy.NotResponding";
    // The server is requesting the client to reconnect. (enhanced rtmp)
    // level: status
    pub const NET_CONNECTION_CONNECT_RECONNECT_REQUEST: &str =
        "NetConnection.Connect.ReconnectRequest";
}

pub mod response_level {
    pub const STATUS: &str = "status";
    pub const WARNING: &str = "warning";
    pub const ERROR: &str = "error";
}
