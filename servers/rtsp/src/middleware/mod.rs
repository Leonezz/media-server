use rtsp_formats::{request::RtspRequest, response::RtspResponse};

use crate::errors::RtspServerResult;
pub mod response_header_appender;
pub trait RtspMiddleware {
    fn pre_request(&self, request: RtspRequest) -> RtspServerResult<RtspRequest> {
        Ok(request)
    }

    fn pre_response(
        &self,
        request: &RtspRequest,
        response: RtspResponse,
    ) -> RtspServerResult<RtspResponse> {
        let _ = request;
        Ok(response)
    }
}
