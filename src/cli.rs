use std::path;

use clap::{Parser, Subcommand, ValueHint};

use crate::*;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Args {
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
    #[arg(env, long)]
    pub host: String,

    /// Don't commit anything to Hydrus
    #[arg(env, short, long)]
    pub dry_run: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    Eval {
        #[command(flatten)]
        common: CommonArgs,

        /// Hashes to evaluate
        #[arg(long)]
        hashes: Vec<String>,
    },
    Daemon {
        #[command(flatten)]
        common: CommonArgs,

        /// Time in minutes to sleep between searches
        #[arg(env, long, default_value_t = DEFAULT_INTERVAL)]
        interval: usize,
    },
}
