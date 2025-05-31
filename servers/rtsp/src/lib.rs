#![feature(let_chains)]
#![feature(if_let_guard)]
use rtsp_formats::{consts::status::RtspStatus, response::RtspResponse};
pub mod config;
pub mod errors;
pub mod media_session;
pub mod middleware;
pub mod server;
pub mod session;
pub const SERVER_AGENT: &str = "yam_server/rtsp";

#[inline(always)]
pub fn rtsp_server_simple_response(status: RtspStatus) -> RtspResponse {
    RtspResponse::builder().status(status).build().unwrap()
}
