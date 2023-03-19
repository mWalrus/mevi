use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Cli {
    pub path: PathBuf,
    #[arg(long, required = false, help = "Print debug information")]
    pub debug: bool,
    #[arg(
        long,
        short,
        required = false,
        help = "Display image information in window"
    )]
    pub info: bool,
    #[arg(long, short, required = false, help = "Start Mevi in fullscreen mode")]
    pub fullscreen: bool,
}
