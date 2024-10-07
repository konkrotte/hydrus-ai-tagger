use std::path;

use anyhow::Result;
use clap::Parser;
use image::ImageReader;
use interrogator::Interrogator;

mod interrogator;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Path to the model folder
    #[arg(short, long)]
    model_dir: path::PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let interrogator = Interrogator::init(args.model_dir)?;
    let image = ImageReader::open("./image.jpg")?;
    interrogator.interrogate(image.decode()?);
    Ok(())
}
