use stream_center::stream_center;
use tracing::{self, Dispatch, Level};
use tracing_subscriber::{self};

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

    let mut stream_center = stream_center::StreamCenter::new();

    let rtmp_server_config = rtmp_server::config::RtmpServerConfig {
        port: 9999,
        chunk_size: 60000,
        write_timeout_ms: 10000,
        read_timeout_ms: 10000,
    };
    let mut server =
        rtmp_server::server::RtmpServer::new(stream_center.get_event_sender(), &rtmp_server_config);

    tokio::spawn(async move { stream_center.run().await });

    let _ = server.run().await;
}
