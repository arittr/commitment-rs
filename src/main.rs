use clap::Parser;
use commitment_rs::cli::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if cli.run().await.is_err() {
        std::process::exit(1);
    }
}
