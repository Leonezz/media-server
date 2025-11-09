use app_utils::{logger::setup_logger, signal};
use clap::{CommandFactory, FromArgMatches, crate_authors, crate_version};

#[tokio::main]
async fn main() {
    let cmd = turn_server::cli::AppCli::command()
        .name("turn_server")
        .version(crate_version!())
        .author(crate_authors!())
        .about("a simple turn server implements rfc 8656, happy traversal")
        .help_template(
            "{name} v{version} by {author-section}{about-section}\n{usage-heading}\n{usage}\n\n{all-args}",
        )
        .arg_required_else_help(false);
    let matches = cmd.get_matches();
    let cli = turn_server::cli::AppCli::from_arg_matches(&matches);
    if let Err(err) = cli {
        eprintln!("argument parse failed: {}", err);
        std::process::exit(1);
    }
    let cli = cli.unwrap().merge();
    if let Err(err) = cli {
        eprintln!("argument parse failed: {}", err);
        std::process::exit(1);
    }
    let mut cli = cli.unwrap();
    cli.inner.inner = cli.inner.inner.resolve();
    let _guard = setup_logger(None, Some("turn_server"), &cli.inner.logger);
    println!("turn_server is running with config:\n{}", cli);
    turn_server::app_run(signal::stop(), cli.inner.inner).await
}
