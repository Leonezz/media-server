use axum::{body::Body, response::IntoResponse};
use stream_center::errors::StreamCenterError;
use thiserror::Error;

use crate::sessions::httpflv::errors::HttpFlvSessionError;

#[derive(Error, Debug)]
pub enum HttpServerError {
    #[error("common http not found error: {0}")]
    NotFound(String),
    #[error("bad request error: {0}")]
    BadRequest(String),
    #[error("common http internal error: {0}")]
    InternalError(String),
    #[error("invalid request payload type: {0}")]
    InvalidRequestPayloadType(String),
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

impl IntoResponse for HttpServerError {
    fn into_response(self) -> axum::response::Response {
        let response_builder = axum::response::Response::builder();
        match self {
            Self::BadRequest(body) => response_builder
                .status(axum::http::StatusCode::BAD_REQUEST)
                .body(Body::from(body))
                .unwrap(),
            Self::InternalError(body) => response_builder
                .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(body))
                .unwrap(),
            Self::NotFound(body) => response_builder
                .status(axum::http::StatusCode::NOT_FOUND)
                .body(Body::from(body))
                .unwrap(),
            Self::InvalidRequestPayloadType(body) => response_builder
                .status(axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE)
                .body(Body::from(body))
                .unwrap(),
        }
    }
}
