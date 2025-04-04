use rtsp_formats::header::transport::TransportHeader;
use url::Url;
use uuid::Uuid;

use crate::session::RtspSession;

#[derive(Debug)]
struct StreamProperties {
    pub(crate) stream_name: String,
    pub(crate) session_id: Uuid,
    pub(crate) sub_stream_name: String,
    pub(crate) uri: Url,
    pub(crate) transport: TransportHeader,
}

#[derive(Debug)]
pub struct RtspMediaSession {
    pub(crate) stream_properities: StreamProperties,
    pub(crate) rtp_session: RtspSession,
}
