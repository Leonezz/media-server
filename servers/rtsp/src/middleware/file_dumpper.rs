use crate::middleware::RtspMiddleware;
use debug_tools::dump::DumpTool;

pub struct DialogFileDumpper {
    file_dump: debug_tools::dump::file_dump::FileDump,
}

impl DialogFileDumpper {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_dump: debug_tools::dump::file_dump::FileDump::new(file_path).unwrap(),
        }
    }
}

impl RtspMiddleware for DialogFileDumpper {
    fn pre_request(
        &mut self,
        request: rtsp_formats::request::RtspRequest,
    ) -> crate::errors::RtspServerResult<rtsp_formats::request::RtspRequest> {
        self.file_dump
            .dump_bytes(&format!("--- REQUEST {} ---\n", chrono::Utc::now()).as_bytes())
            .unwrap();
        self.file_dump
            .dump_bytes(&format!("{}", request).as_bytes())
            .unwrap();
        Ok(request)
    }

    fn pre_response(
        &mut self,
        request: &rtsp_formats::request::RtspRequest,
        response: rtsp_formats::response::RtspResponse,
    ) -> crate::errors::RtspServerResult<rtsp_formats::response::RtspResponse> {
        self.file_dump
            .dump_bytes(
                &format!(
                    "--- RESPONSE to {} AT {} ---\n",
                    request.method(),
                    chrono::Utc::now()
                )
                .as_bytes(),
            )
            .unwrap();
        self.file_dump
            .dump_bytes(&format!("{}", response).as_bytes())
            .unwrap();
        Ok(response)
    }
}
