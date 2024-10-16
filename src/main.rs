use std::{
    fmt::Write,
    io::IsTerminal,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use clap::Parser;
use cli::*;
use indicatif::{HumanDuration, ParallelProgressIterator, ProgressState, ProgressStyle};
use interrogator::Interrogator;
use log::{error, info, warn};
use rayon::prelude::*;
use tagger::Tagger;
use tokio::runtime::Runtime;
use tracing_log::AsTrace;
use utils::parse_hashes_file;

mod cli;
mod interrogator;
mod tagger;
mod utils;

const DEFAULT_THRESHOLD: f32 = 0.35;
const DEFAULT_TAG_SERVICE: &str = "ai tags";
const DEFAULT_INTERVAL: usize = 60;

struct App {
    rt: Arc<Runtime>,
    args: Args,
}

impl App {
    fn new(args: Args) -> Result<Self> {
        Ok(Self {
            rt: Arc::new(Runtime::new()?),
            args,
        })
    }

    fn run(&self) -> Result<()> {
        match &self.args.command {
            Commands::Eval {
                common:
                    CommonArgs {
                        model_dir,
                        threshold,
                        tag_service,
                        access_key,
                        host,
                        dry_run,
                    },
                target_images,
            } => {
                let client = Arc::new(hydrus_api::Client::new(host, access_key));
                let interrogator = Arc::new(Interrogator::init(model_dir)?);
                let tagger = Tagger::new(self.rt.clone(), client, interrogator, *threshold);
                let service_key = tagger.get_tag_service_key_from_name(tag_service)?;

                let hashes = match (
                    &target_images.hashes,
                    &target_images.file,
                    &target_images.automatic,
                ) {
                    (Some(hashes), _, _) => hashes,
                    (_, Some(file_path), _) => &parse_hashes_file(file_path)?,
                    (_, _, Some(automatic)) if *automatic => {
                        &tagger.get_untagged_images(&service_key)?
                    }
                    _ => {
                        println!("Not doing anything");
                        return Ok(());
                    }
                };

                let style = ProgressStyle::with_template(
                    "[{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
                )
                .unwrap()
                .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
                    write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
                })
                .progress_chars("#>-");

                if *dry_run {
                    warn!("Not actually adding tags");
                }

                let start_time = Instant::now();

                println!("Tagging images");
                hashes
                    .par_iter()
                    .progress_with_style(style)
                    .try_for_each(|hash| {
                        tagger.tag_image(&service_key, hash, *dry_run).map(|_| ())
                    })?;

                println!("Done in {}", HumanDuration(start_time.elapsed()));

                Ok(())
            }
            Commands::Daemon {
                common:
                    CommonArgs {
                        model_dir,
                        threshold,
                        tag_service,
                        access_key,
                        host,
                        dry_run,
                    },
                interval,
            } => {
                let interval_duration = Duration::from_secs((interval * 60) as u64);
                let client = Arc::new(hydrus_api::Client::new(host, access_key));
                let interrogator = Arc::new(Interrogator::init(model_dir)?);
                let tagger = Tagger::new(self.rt.clone(), client, interrogator, *threshold);
                let service_key = tagger.get_tag_service_key_from_name(tag_service)?;

                loop {
                    let start_time = Instant::now();

                    match tagger.get_untagged_images(&service_key) {
                        Ok(hashes) => {
                            if hashes.is_empty() {
                                info!("Nothing to tag");
                            }

                            hashes.par_iter().for_each(|hash| {
                                if let Err(e) = tagger.tag_image(&service_key, hash, *dry_run) {
                                    error!("Error evaluating hash: {:?}", e);
                                }
                            });
                        }
                        Err(e) => error!("Search error: {:?}", e),
                    }

                    let elapsed_time = start_time.elapsed();
                    if elapsed_time < interval_duration {
                        let sleep_duration = interval_duration - elapsed_time;
                        info!("Sleeping for {:?}", sleep_duration);
                        std::thread::sleep(sleep_duration);
                    }
                }
            }
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    match std::io::stdout().is_terminal() {
        true => tracing_subscriber::fmt()
            .pretty()
            .with_thread_names(false)
            .with_max_level(args.verbose.log_level_filter().as_trace())
            .with_line_number(false)
            .without_time()
            .with_file(false)
            .with_writer(std::io::stderr)
            .init(),
        false => tracing_subscriber::fmt()
            .with_max_level(args.verbose.log_level_filter().as_trace())
            .with_writer(std::io::stderr)
            .init(),
    }

    let app = App::new(args)?;
    app.run()
}
