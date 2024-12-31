use rocket::Responder;
use stream_center::errors::StreamCenterError;
use thiserror::Error;

use crate::sessions::httpflv::errors::HttpFlvSessionError;

#[derive(Error, Debug, Responder)]
pub enum HttpServerError {
    #[error("common http not found error: {0}")]
    #[response(status = 404, content_type = "plain")]
    NotFound(String),
    #[error("bad request error: {0}")]
    #[response(status = 400, content_type = "plain")]
    BadRequest(String),
    #[error("common http internal error: {0}")]
    #[response(status = 500, content_type = "plain")]
    InternalError(String),
}

pub type HttpServerResult<T> = Result<T, HttpServerError>;

impl From<HttpFlvSessionError> for HttpServerError {
    fn from(value: HttpFlvSessionError) -> Self {
        match value {
            HttpFlvSessionError::StreamCenterError(err) => match err {
                StreamCenterError::StreamNotFound(id) => Self::NotFound(format!(
                    "stream not found, app: {}, stream: {}",
                    id.app, id.stream_name
                )),
                StreamCenterError::InvalidStreamType(t) => {
                    Self::BadRequest(format!("bad stream type: {}", t))
                }
                _ => Self::InternalError("internal error".to_string()),
            },
            _ => Self::InternalError("internal error".to_string()),
        }
    }
}
