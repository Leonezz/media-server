#[cfg(tokio_unstable)]
use std::time::Duration;

use time::macros::format_description;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{
    EnvFilter, fmt::time::LocalTime, layer::SubscriberExt, util::SubscriberInitExt,
};

#[tokio::main]

async fn main() {
    let writer = tracing_appender::rolling::Builder::new()
        .filename_prefix("turn_server")
        .rotation(Rotation::HOURLY)
        .build("./logs/turn")
        .expect("build log writer for turn server");
    let (nonblocking_writer, _guard) = tracing_appender::non_blocking(writer);

    let registry = tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_timer(LocalTime::new(format_description!(
                    "[hour]:[minute]:[second]"
                )))
                .with_file(true)
                .with_line_number(true)
                .compact()
                .with_ansi(false) // enable color when sink to stdout
                .with_writer(nonblocking_writer),
        )
        .with(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(format!("{},tokio=trace,runtime=trace", "debug"))));
    #[cfg(tokio_unstable)]
    let registry = registry.with(
        console_subscriber::ConsoleLayer::builder()
            // set how long the console will retain data from completed tasks
            .retention(Duration::from_secs(60))
            // set the address the server is bound to
            .server_addr(([127, 0, 0, 1], 6669))
            .spawn(),
    );

    registry.init();
    tracing::info!("turn server running at: udp://127.0.0.1:5799");
    turn_server::app_run(signal::stop()).await
}
