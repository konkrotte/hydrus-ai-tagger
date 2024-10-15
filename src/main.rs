use std::{
    io::Cursor,
    path,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueHint};
use hydrus_api::api_core::{
    common::FileIdentifier,
    endpoints::{
        adding_tags::AddTagsRequestBuilder,
        searching_and_fetching_files::{FileSearchOptions, SearchQueryEntry},
    },
};
use image::{DynamicImage, ImageReader};
use indexmap::IndexMap;
use interrogator::Interrogator;
use log::{debug, error, info, warn};
use rayon::prelude::*;
use tokio::runtime::Runtime;

mod interrogator;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

const DEFAULT_THRESHOLD: f32 = 0.35;
const DEFAULT_TAG_SERVICE: &str = "ai tags";
const DEFAULT_INTERVAL: usize = 60;

#[derive(Subcommand)]
enum Commands {
    Daemon {
        /// Path to the model folder
        #[arg(env, long, value_hint = ValueHint::DirPath)]
        model_dir: path::PathBuf,

        /// The threshold for a tag to be used
        #[arg(env, long, default_value_t = DEFAULT_THRESHOLD)]
        threshold: f32,

        /// The tag service to use
        #[arg(env, long, default_value_t = String::from(DEFAULT_TAG_SERVICE))]
        tag_service: String,

        /// Time in minutes to sleep between searches
        #[arg(env, long, default_value_t = DEFAULT_INTERVAL)]
        interval: usize,

        /// Access key for the Hydrus Client API
        #[arg(env, long)]
        access_key: String,

        /// URL for the Hydrus Client API server
        #[arg(env, long)]
        host: String,

        /// Don't commit anything to Hydrus
        #[arg(env, short, long)]
        dry_run: bool,
    },
}

fn decode_image(bytes: &[u8]) -> Result<DynamicImage> {
    let mut reader = ImageReader::new(Cursor::new(bytes));
    reader.no_limits();
    reader.with_guessed_format()?.decode().map_err(Into::into)
}

fn tag_image(
    rt: &Runtime,
    client: Arc<hydrus_api::Client>,
    interrogator: Arc<Interrogator>,
    threshold: f32,
    service_key: &str,
    hash: &str,
    dry_run: bool,
) -> Result<()> {
    debug!("Tagging {}", hash);

    let record = rt
        .block_on(client.get_file(FileIdentifier::hash(hash)))
        .context("Error getting image file from Hydrus API")?;

    let image = decode_image(&record.bytes)
        .or_else(|_| {
            warn!("Failed decoding original image, falling back to using hydrus render");
            let rendered = rt
                .block_on(client.get_render(FileIdentifier::hash(hash)))
                .context("Error rendering file")?;
            decode_image(&rendered.bytes)
        })
        .context("Failed to decode image")?;

    let (ratings, tags) = interrogator
        .interrogate(&image)
        .context("Failed interrogating model")?;

    let mut filtered_tags = filter_and_process_tags(tags, threshold);

    if let Some(ratings) = ratings {
        filtered_tags.push(get_rating(&ratings)?);
    }

    let request = AddTagsRequestBuilder::default()
        .add_hash(hash)
        .add_tags(service_key.to_string(), filtered_tags)
        .build();

    debug!("Tags to be added: {:?}", request.service_keys_to_tags);

    if dry_run {
        warn!("Not adding tags, because dry run");
    } else {
        rt.block_on(client.add_tags(request))
            .context("Failed adding tags")?;
    }

    Ok(())
}

fn get_untagged_images(
    rt: &Runtime,
    client: &hydrus_api::Client,
    service_key: &str,
) -> Result<Vec<String>> {
    let hashes = rt
        .block_on(client.search_file_hashes(
            vec![
                SearchQueryEntry::Tag(String::from("system:untagged")),
                SearchQueryEntry::Tag(String::from("system:filetype is image")),
            ],
            FileSearchOptions::new().tag_service_key(service_key.to_string()),
        ))?
        .hashes;
    Ok(hashes)
}

fn tag_untagged_images(
    rt: &Runtime,
    client: Arc<hydrus_api::Client>,
    interrogator: Arc<Interrogator>,
    threshold: f32,
    service_key: &str,
    dry_run: bool,
) {
    match get_untagged_images(rt, &client, service_key) {
        Ok(hashes) => {
            if hashes.is_empty() {
                info!("Nothing to tag");
                return;
            }

            hashes.par_iter().for_each(|hash| {
                if let Err(e) = tag_image(
                    rt,
                    client.clone(),
                    interrogator.clone(),
                    threshold,
                    service_key,
                    hash,
                    dry_run,
                ) {
                    error!("Error evaluating hash: {:?}", e);
                }
            });
        }
        Err(e) => error!("Search error: {:?}", e),
    }
}

fn get_tag_service_key_from_name(
    rt: &Runtime,
    client: &hydrus_api::Client,
    tag_service: &String,
) -> Result<String> {
    let service_key = rt
        .block_on(client.get_services())?
        .services
        .par_iter()
        .find_any(|x| x.1.name == *tag_service)
        .map(|x| x.0.to_owned())
        .ok_or(anyhow!("Could not find tag service {}", tag_service))?;
    Ok(service_key)
}

/// Kaomoji tags to be excluded from the process of replacing '_' with space
const KAOMOJIS: &[&str] = &[
    "0_0", "(o)_(o)", "+_+", "+_-", "._.", "<o>_<o>", "<|>_<|>", "=_=", ">_<", "3_3", "6_9", ">_o",
    "@_@", "^_^", "o_o", "u_u", "x_x", "|_|", "||_||",
];

pub fn get_rating(ratings: &IndexMap<String, f32>) -> Result<String> {
    ratings
        .par_iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(r, _)| format!("rating:{r}"))
        .ok_or_else(|| anyhow!("Ratings was empty"))
}

pub fn filter_and_process_tags(
    tags: indexmap::IndexMap<String, f32>,
    threshold: f32,
) -> Vec<String> {
    tags.into_par_iter()
        .filter(|(_, confidence)| *confidence > threshold)
        .map(|(tag, _)| {
            if KAOMOJIS.contains(&tag.as_str()) {
                tag
            } else {
                tag.replace('_', " ")
            }
        })
        .collect()
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    match args.command {
        Commands::Daemon {
            model_dir,
            threshold,
            tag_service,
            interval,
            access_key,
            host,
            dry_run,
        } => {
            let rt = Runtime::new()?;
            let client = Arc::new(hydrus_api::Client::new(host, access_key));

            let interval_duration = Duration::from_secs((interval * 60) as u64);
            let interrogator = Arc::new(Interrogator::init(&model_dir)?);
            let service_key = get_tag_service_key_from_name(&rt, &client, &tag_service)?;

            loop {
                let start_time = Instant::now();

                tag_untagged_images(
                    &rt,
                    client.clone(),
                    interrogator.clone(),
                    threshold,
                    &service_key,
                    dry_run,
                );

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
