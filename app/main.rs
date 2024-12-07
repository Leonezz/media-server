use tokio::net::TcpListener;
use tracing::{self, Dispatch, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        // Use a more compact, abbreviated log format
        .compact()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Display the thread name an event was recorded on
        .with_thread_names(true)
        // display the event's target (module path)
        .with_target(true)
        // Build the subscriber
        .finish();
    tracing::dispatcher::set_global_default(Dispatch::new(subscriber)).unwrap();

    tracing::debug!("running");

    let listener = TcpListener::bind("127.0.0.1:9999".to_string())
        .await
        .unwrap();
    loop {
        let (tcp_stream, _) = listener.accept().await.unwrap();
        tracing::debug!("got a connection");
        let handshaker = rtmp::handshake::server::HandshakeServer::new(tcp_stream);
        let res = handshaker.handshake(false).await;
        tracing::debug!("{:?}", res);
    }
}
