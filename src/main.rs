use std::path;

use anyhow::Result;
use clap::{Parser, Subcommand};
use image::ImageReader;
use interrogator::Interrogator;

mod interrogator;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Daemon {
    /// Path to the model folder
    #[arg(short, long)]
    model_dir: path::PathBuf,

    /// The threshold for a tag to be used
    #[arg(short, long, default_value_t = 0.35)]
    threshold: f32,
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Daemon { model_dir, threshold } => {
            let interrogator = Interrogator::init(model_dir)?;
            let image = ImageReader::open("./image.jpg")?;
            println!("{:?}", interrogator.interrogate(image.decode()?, threshold)?);
        }
    }
    Ok(())
}
