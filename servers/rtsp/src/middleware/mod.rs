use crate::errors::RtspServerResult;
use rtsp_formats::{request::RtspRequest, response::RtspResponse};
pub mod file_dumpper;
pub mod response_header_appender;

pub trait RtspMiddleware {
    fn pre_request(&mut self, request: RtspRequest) -> RtspServerResult<RtspRequest> {
        Ok(request)
    }

    fn pre_response(
        &mut self,
        request: &RtspRequest,
        response: RtspResponse,
    ) -> RtspServerResult<RtspResponse> {
        let _ = request;
        Ok(response)
    }
}
