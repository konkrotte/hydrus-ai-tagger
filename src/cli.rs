use std::path;

use clap::{Parser, Subcommand, ValueHint};

use crate::{DEFAULT_INTERVAL, DEFAULT_TAG_SERVICE, DEFAULT_THRESHOLD};

#[derive(Parser)]
#[command(author, version, about)]
pub struct Args {
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Parser)]
pub struct CommonArgs {
    /// Path to the model folder
    #[arg(env, long, value_hint = ValueHint::DirPath)]
    pub model_dir: path::PathBuf,

    /// The threshold for a tag to be used
    #[arg(env, long, default_value_t = DEFAULT_THRESHOLD)]
    pub threshold: f32,

    /// The tag service to use
    #[arg(env, long, default_value_t = String::from(DEFAULT_TAG_SERVICE))]
    pub tag_service: String,

    /// Access key for the Hydrus Client API
    #[arg(env, long)]
    pub access_key: String,

    /// URL for the Hydrus Client API server
    #[arg(env, long, value_hint = ValueHint::Url)]
    pub host: String,

    /// Don't commit anything to Hydrus
    #[arg(env, short, long)]
    pub dry_run: bool,
}

#[derive(clap::Args)]
#[group(required = true, multiple = false)]
#[clap()]
pub struct TargetImages {
    /// Path to text file containing new-line separated list of hashes
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub file: Option<path::PathBuf>,

    /// Hashes to evaluate
    #[arg(long)]
    pub hashes: Option<Vec<String>>,

    /// Tag images that are untagged in the provided tag service
    #[arg(long)]
    pub automatic: Option<bool>,
}

#[derive(Subcommand)]
pub enum Commands {
    Eval {
        #[command(flatten)]
        common: CommonArgs,

        #[clap(flatten)]
        target_images: TargetImages,
    },
    Daemon {
        #[command(flatten)]
        common: CommonArgs,

        /// Time in minutes to sleep between searches
        #[arg(env, long, default_value_t = DEFAULT_INTERVAL)]
        interval: usize,
    },
}
