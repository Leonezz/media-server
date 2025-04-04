use rtsp_formats::header::RtspHeader;

use super::RtspMiddleware;

pub const SERVER_NAME: &str = "yam_server/rtsp";

#[derive(Debug)]
pub struct ResponseHeaderAppender;

impl RtspMiddleware for ResponseHeaderAppender {
    fn pre_response(
        &self,
        request: &rtsp_formats::request::RtspRequest,
        mut response: rtsp_formats::response::RtspResponse,
    ) -> crate::errors::RtspServerResult<rtsp_formats::response::RtspResponse> {
        let cseq = request.headers().cseq().unwrap_or(0);
        let headers = response.headers_mut();
        headers.set(RtspHeader::CSeq, cseq.to_string());
        headers.set(RtspHeader::Server, SERVER_NAME);
        headers.set(RtspHeader::Date, chrono::Utc::now().to_rfc2822());
        response.set_version(request.version().clone());
        Ok(response)
    }
}
