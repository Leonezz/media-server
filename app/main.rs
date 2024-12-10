use tokio::net::TcpListener;
use tracing::{self, Dispatch, Level};
use tracing_subscriber::{self, util::SubscriberInitExt};

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

    let rtmp_server_config = rtmp_server::publish::config::RtmpPublishServerConfig { port: 9999 };
    let mut server = rtmp_server::publish::server::RtmpPublishServer::new(&rtmp_server_config);
    let _ = server.run().await;
}
