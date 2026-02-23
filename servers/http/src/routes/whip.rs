use std::{collections::HashMap, fmt};

use axum::{
    extract::{FromRequest, Query},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use sdp_formats::session::{SDPAttrManager, Sdp};
use tokio_util::bytes::Buf;
use utils::traits::reader::ReadFrom;
use webrtc_formats::sdp_extension::rfc8830::Msid;

use crate::errors::HttpServerError;

#[derive(Debug)]
pub(crate) struct SdpBody(Sdp);

impl fmt::Display for SdpBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<S> FromRequest<S> for SdpBody
where
    S: Send + Sync,
{
    type Rejection = Response;
    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        if !req.headers().contains_key(axum::http::header::CONTENT_TYPE) {
            return Err(HttpServerError::InvalidRequestPayloadType(
                "expect application/sdp but not content type header found".to_string(),
            )
            .into_response());
        }

        {
            let content_type = req
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .unwrap()
                .to_str()
                .map_err(|e| {
                    HttpServerError::BadRequest(format!(
                        "error decoding content type header value as str: {}",
                        e
                    ))
                    .into_response()
                })?
                .to_lowercase();
            if !content_type.eq("application/sdp") {
                return Err(HttpServerError::InvalidRequestPayloadType(format!(
                    "expect application/sdp, but got {} instead",
                    content_type
                ))
                .into_response());
            }
        }

        let body = axum::body::Bytes::from_request(req, state)
            .await
            .map_err(IntoResponse::into_response)?;
        let sdp = Sdp::read_from(&mut body.reader()).map_err(|err| {
            HttpServerError::BadRequest(format!("error trying to read sdp from body, err: {}", err))
                .into_response()
        })?;

        Ok(Self(sdp))
    }
}

pub(crate) async fn serve(
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    body: SdpBody,
) -> String {
    tracing::trace!("headers: {:?}", headers);
    tracing::trace!("params: {:?}", params);
    tracing::trace!("body: {}", body);
    let msids = body.0.media_description[1].get_all_extension_attr::<Msid>();
    tracing::trace!("msids: {:?}", msids);
    "OK".to_string()
}
