use httpflv_server::config::HttpFlvServerConfig;
use stream_center::stream_center;
use time::macros::format_description;
use tokio::signal;
use tracing::{self, Dispatch, Level};
use tracing_subscriber::{self, fmt::time::LocalTime};

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_timer(LocalTime::new(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second] [unix_timestamp precision:nanosecond]"
        )))
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
        ip: "0.0.0.0".to_string(),
        port: 9999,
        chunk_size: 60000,
        write_timeout_ms: 10000,
        read_timeout_ms: 10000,
    };
    let mut rtmp_server =
        rtmp_server::server::RtmpServer::new(&rtmp_server_config, stream_center.get_event_sender());
    let httpflv_server_config = HttpFlvServerConfig {
        ip: "0.0.0.0".to_string(),
        port: 8000,
        chunk_size: 60000,
        write_timeout_ms: 10000,
        read_timeout_ms: 10000,
    };
    let mut http_pull_server = httpflv_server::server::HttpFlvServer::new(
        &httpflv_server_config,
        stream_center.get_event_sender(),
    );

    tokio::spawn(async move { stream_center.run().await });
    tokio::spawn(async move {
        let _ = rtmp_server.run().await;
    });
    tokio::spawn(async move {
        let _ = http_pull_server.run().await;
    });

    let _ = signal::ctrl_c().await;
}
