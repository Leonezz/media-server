use clap::{CommandFactory, FromArgMatches, crate_authors, crate_version};
mod errors;
mod logger;
#[tokio::main]
async fn main() {
    let cmd = stun_client::cli::AppCli::command()
        .name("stun_client")
        .version(crate_version!())
        .author(crate_authors!())
        .about("a simple stun client implements rfc 8489")
        .help_template("{name} v{version} by {author-section}{about-section}\n{usage-heading}\n{usage}\n\n{all-args}")
        .arg_required_else_help(false);
    let matches = cmd.get_matches();
    let cli = stun_client::cli::AppCli::from_arg_matches(&matches);
    if let Err(err) = cli {
        eprintln!("argument parse failed: {}", err);
        std::process::exit(1);
    }
    let cli = cli.unwrap().merge();
    if let Err(err) = cli {
        eprintln!("argument parse failed: {}", err);
        std::process::exit(1);
    }
    let cli = cli.unwrap();
    let _guard = app_utils::logger::setup_logger(None, Some("stun_client"), &cli.inner.logger);
    println!("stun_client is running with config:\n{}", cli);
    stun_client::app_run(app_utils::signal::stop(), cli.inner.inner).await;
}
