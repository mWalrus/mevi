use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Cli {
    pub path: PathBuf,
    #[arg(long, required = false)]
    pub debug: bool,
}
