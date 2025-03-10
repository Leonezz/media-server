use std::{collections::HashMap, io::Cursor};

use rocket::{FromForm, Request, Response, State, get, http::ContentType, response::Responder};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio_util::bytes::BytesMut;

use crate::{
    errors::{HttpServerError, HttpServerResult},
    server::HttpServerContext,
    sessions::httpflv::session::{HttpFlvSession, HttpFlvSessionConfig, StreamProperties},
};

use super::ext::FlvStreamName;

pub struct HttpFlvStream {
    receiver: UnboundedReceiver<BytesMut>,
    bytes_buffer: Option<Cursor<BytesMut>>,
}

impl tokio::io::AsyncRead for HttpFlvStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        use std::pin::Pin;
        use std::task::Poll;

        if let Some(cursor) = self.bytes_buffer.as_mut() {
            let available_data = cursor.get_ref().len() as u64 - cursor.position();
            if available_data > 0 {
                let to_read = buf.remaining().min(available_data as usize);
                let start = cursor.position() as usize;
                buf.put_slice(&cursor.get_ref()[start..start + to_read]);
                cursor.set_position(cursor.position() + to_read as u64);
                return Poll::Ready(Ok(()));
            }
        }

        match Pin::new(&mut self.receiver).poll_recv(cx) {
            Poll::Ready(Some(bytes)) => {
                self.bytes_buffer = Some(Cursor::new(bytes));
                self.poll_read(cx, buf)
            }
            Poll::Ready(None) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
        }
    }
}

// Implement Responder for HttpFlvStream
impl<'r> Responder<'r, 'r> for HttpFlvStream {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'r> {
        Response::build()
            .header(ContentType::new("video", "x-flv"))
            .streamed_body(self)
            .ok()
    }
}

#[derive(Debug, FromForm)]
pub struct HttpFlvPullRequest {
    #[field(name = uncased("audioOnly"))]
    #[field(name = uncased("audio_only"))]
    #[field(name = uncased("audio-only"))]
    audio_only: Option<bool>,
    #[field(name = uncased("videoOnly"))]
    #[field(name = uncased("video_only"))]
    #[field(name = uncased("video-only"))]
    video_only: Option<bool>,
    #[field(name = uncased("backtrackGopCnt"))]
    #[field(name = uncased("backtrack-gop-cnt"))]
    #[field(name = uncased("backtrack_gop_cnt"))]
    backtrack_gop_cnt: Option<usize>,
    #[field(name = "ctx")]
    _ctx: Option<String>,
}

#[get("/<app>/<stream>?<params..>")]
pub(crate) async fn serve(
    ctx: &State<HttpServerContext>,
    app: &str,
    stream: FlvStreamName<'_>,
    params: HttpFlvPullRequest,
) -> HttpServerResult<HttpFlvStream> {
    let stream = stream.0;
    log::info!(
        "get http flv pull request, app: {}, stream: {}, params: {:?}",
        app,
        stream,
        params
    );
    if app.is_empty() {
        return Err(HttpServerError::BadRequest(format!(
            "bad app and stream, app: {}, stream: {}",
            app, stream
        )));
    }

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
            stream_type: Default::default(),
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

    Ok(HttpFlvStream {
        receiver: response_receiver,
        bytes_buffer: Default::default(),
    })
}
