use clap::{CommandFactory, FromArgMatches, crate_authors, crate_version};
mod errors;
mod logger;
#[tokio::main]
async fn main() {
    let cmd = yam_server::cli::AppCli::command()
        .name("yam_server")
        .version(crate_version!())
        .author(crate_authors!())
        .about("yet another media server")
        .help_template("{name} v{version} by {author-section}{about-section}\n{usage-heading}\n{usage}\n\n{all-args}")
        .arg_required_else_help(false);
    let matches = cmd.get_matches();
    let cli = yam_server::cli::AppCli::from_arg_matches(&matches);
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
    let _guard = app_utils::logger::setup_logger(None, Some("yam_server"), &cli.inner.logger);
    println!("yam server is running with config:\n{}", cli);
    yam_server::app_run(app_utils::signal::stop(), cli.inner.inner).await;
}
