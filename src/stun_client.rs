use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use stun_client::config::AppConfig;
use time::macros::format_description;
use tracing_subscriber::{EnvFilter, fmt::time::LocalTime};
mod errors;
mod logger;
use crate::logger::parse_log_level;
#[tokio::main]
async fn main() {
    let app = clap::builder::Command::new("stun client")
        .version("0.1.0")
        .author("zhuwenq <zhuwenqa@outlook.com>")
        .about("a simple stun client tool")
        .arg(
            clap::Arg::new("server")
                .help("stun server address in addr:port format")
                .long("server")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .default_value("stun.l.google.com:19302"),
        )
        .arg(
            clap::Arg::new("protocol")
                .help("tcp or udp to use")
                .long("protocol")
                .value_parser(["tcp", "udp"])
                .default_value("udp"),
        )
        .arg(
            clap::Arg::new("family")
                .help("ip family, v4 for ipv4 or v6 for ipv6")
                .long("family")
                .value_parser([
                    clap::builder::PossibleValue::new("v4").help("use ipv4"),
                    clap::builder::PossibleValue::new("v6").help("use ipv6"),
                ])
                .default_value("v4"),
        )
        .arg(
            clap::Arg::new("localaddr")
                .help("local address used for stun transaction")
                .long("localaddr")
                .value_parser(clap::value_parser!(IpAddr))
                .default_value_ifs([
                    (
                        "family",
                        "v4",
                        clap::builder::Str::from(Ipv4Addr::UNSPECIFIED.to_string()),
                    ),
                    (
                        "family",
                        "v6",
                        clap::builder::Str::from(Ipv6Addr::UNSPECIFIED.to_string()),
                    ),
                ]),
        )
        .arg(
            clap::Arg::new("localport")
                .help("local port used for stun transaction")
                .long("localport")
                .value_parser(
                    clap::builder::RangedI64ValueParser::<u16>::new()
                        .range(u16::MIN as i64..=u16::MAX as i64),
                )
                .default_value("0"),
        )
        .arg(
            clap::Arg::new("loglevel")
                .help("log level: trace/debug/info/warn/error/none")
                .value_parser(["trace", "debug", "info", "warn", "error", "none"])
                .default_value("none")
                .long("loglevel"),
        );
    let matches = app.get_matches();
    let protocol = matches.get_one::<String>("protocol").unwrap().parse();
    if let Err(err) = protocol {
        tracing::error!("{}", err);
        return;
    }
    let config = AppConfig {
        protocol: protocol.unwrap(),
        server: matches.get_one::<String>("server").unwrap().to_owned(),
        local_addr: matches.get_one::<IpAddr>("localaddr").unwrap().to_owned(),
        local_port: matches.get_one::<u16>("localport").unwrap().to_owned(),
    };
    if let Err(err) = config.validate() {
        tracing::error!("{}", err);
        return;
    }

    let loglevel = if let Some(level) = matches.get_one::<String>("loglevel") {
        if level.eq("none") {
            None
        } else {
            parse_log_level(level).ok()
        }
    } else {
        None
    };

    if let Some(loglevel) = loglevel {
        tracing_subscriber::fmt()
            .with_timer(LocalTime::new(format_description!(
                "[hour]:[minute]:[second]"
            )))
            .with_env_filter(
                EnvFilter::try_from_default_env()
                    .unwrap_or(EnvFilter::new(format!("{}", loglevel))),
            )
            .compact()
            .with_ansi(true)
            .init();
        tracing::debug!("running with {:?}", config);
    }
    stun_client::app_run(config, signal::stop()).await;
}
