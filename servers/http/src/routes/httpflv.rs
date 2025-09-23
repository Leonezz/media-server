use futures::StreamExt;
use server_utils::stream_properities::StreamProperties;
use std::{collections::HashMap, convert::Infallible};
use tokio::sync::mpsc::{self};

use crate::{
    errors::HttpServerError,
    server::HttpServerContext,
    sessions::httpflv::session::{HttpFlvSession, HttpFlvSessionConfig},
};

#[derive(Debug, serde::Deserialize)]
pub struct HttpFlvPullRequest {
    #[serde(alias = "audioOnly")]
    #[serde(alias = "audio_only")]
    #[serde(alias = "audio-only")]
    audio_only: Option<bool>,
    #[serde(alias = "videoOnly")]
    #[serde(alias = "video_only")]
    #[serde(alias = "vudio-only")]
    video_only: Option<bool>,
    #[serde(alias = "backtrackGopCnt")]
    #[serde(alias = "backtrack-gop-cnt")]
    #[serde(alias = "backtrack_gop_cnt")]
    backtrack_gop_cnt: Option<usize>,
    #[serde(alias = "ctx")]
    _ctx: Option<String>,
}

pub(crate) async fn serve(
    axum::extract::State(ctx): axum::extract::State<HttpServerContext>,
    axum::extract::Path((app, stream)): axum::extract::Path<(String, String)>,
    axum::extract::Query(params): axum::extract::Query<HttpFlvPullRequest>,
) -> impl axum::response::IntoResponse {
    tracing::info!(
        "get http flv pull request, app: {}, stream: {}, params: {:?}",
        app,
        stream,
        params
    );
    if app.is_empty() || stream.is_empty() || !stream.ends_with(".flv") {
        return Err(HttpServerError::BadRequest(format!(
            "bad app and stream, app: {}, stream: {}",
            app, stream
        )));
    }
    let stream = stream.strip_suffix(".flv").unwrap();

    let mut ctx_params: HashMap<String, String> = HashMap::new();

    if params.audio_only.unwrap_or(false) {
        ctx_params.insert(super::params::AUDIO_ONLY_KEY.to_string(), "".to_string());
    }
    if params.video_only.unwrap_or(false) {
        ctx_params.insert(super::params::VIDEO_ONLY_KEY.to_string(), "".to_string());
    }
    if let Some(cnt) = params.backtrack_gop_cnt {
        ctx_params.insert(
            super::params::BACKTRACK_GOP_KEY.to_string(),
            cnt.to_string(),
        );
    }

    let (response_sender, response_receiver) = mpsc::unbounded_channel();

    let mut session = HttpFlvSession::new(
        HttpFlvSessionConfig {
            chunk_size: 10000,
            write_timeout_ms: 10000,
            read_timeout_ms: 10000,
        },
        ctx.stream_center_event_sender.clone(),
        StreamProperties {
            app: app.to_string(),
            stream_name: stream.to_string(),
            stream_context: ctx_params,
        },
        response_sender,
    );

    // have to split subscribe from serve_pull_request so we can return 404 if not found
    let subscribe_response = session.subscribe_from_stream_center().await?;

    tokio::spawn(async move {
        let _ = session.serve_pull_request(subscribe_response).await;
        let _ = session.unsubscribe_from_stream_center().await;
    });

    Ok(axum::response::Response::builder()
        .status(axum::http::StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "video/x-flv")
        .header(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(
            axum::body::Body::from_stream(
                tokio_stream::wrappers::UnboundedReceiverStream::new(response_receiver)
                    .map(|chunk| Ok::<axum::body::Bytes, Infallible>(chunk.freeze())),
            )
            .into_data_stream(),
        )
        .unwrap())
}
