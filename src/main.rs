use std::{
    path,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use hydrus_api::api_core::{
    common::FileIdentifier,
    endpoints::{
        adding_tags::AddTagsRequestBuilder,
        searching_and_fetching_files::{FileSearchOptions, SearchQueryEntry},
    },
};
use image::load_from_memory;
use interrogator::Interrogator;
use tokio::runtime::Runtime;

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
        #[arg(long)]
        model_dir: path::PathBuf,

        /// The threshold for a tag to be used
        #[arg(long, default_value_t = 0.35)]
        threshold: f32,

        /// The tag service to use
        #[arg(long, default_value_t = String::from("ai tags"))]
        tag_service: String,

        /// Time in minutes to sleep between searches
        #[arg(long, default_value_t = 60)]
        interval: usize,

        /// Access key for the Hydrus Client API
        #[arg(long)]
        access_key: String,

        /// URL for the Hydrus Client API server
        #[arg(long)]
        host: String,

        #[arg(short, long, default_value_t = false)]
        dry_run: bool,
    },
}

fn evaluate_hash(
    rt: &Runtime,
    client: &hydrus_api::Client,
    interrogator: &Interrogator,
    threshold: f32,
    service_key: &str,
    hash: &str,
    dry_run: bool,
) -> Result<()> {
    println!("{}", hash);
    let record = rt.block_on(client.get_file(FileIdentifier::hash(hash)))?;
    let image = load_from_memory(&record.bytes)?;
    let (ratings, tags) = interrogator.interrogate(image)?;

    let request = AddTagsRequestBuilder::default()
        .add_hash(hash)
        .add_tags(
            service_key.to_string(),
            ratings
                .as_ref()
                .unwrap()
                .iter()
                .chain(tags.iter())
                .filter(|tag| *tag.1 > threshold)
                .map(|tag| tag.0.to_owned())
                .collect::<Vec<String>>(),
        )
        .build();
    println!("{:?}", request.service_keys_to_tags);
    if !dry_run {
        rt.block_on(client.add_tags(request))?;
    }
    Ok(())
}

fn search(rt: &Runtime, client: &hydrus_api::Client, tag_service: &str) -> Result<Vec<String>> {
    let hashes = rt
        .block_on(client.search_file_hashes(
            vec![
                SearchQueryEntry::Tag(String::from("system:untagged")),
                SearchQueryEntry::Tag(String::from("system:filetype is image")),
            ],
            FileSearchOptions::new().tag_service_name(tag_service.to_string()),
        ))?
        .hashes;
    Ok(hashes)
}

fn main() -> Result<()> {
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
            let client = hydrus_api::Client::new(host, access_key);

            let interval_duration = Duration::from_secs((interval * 60) as u64);
            let interrogator = Interrogator::init(&model_dir).unwrap();
            let service_key = rt
                .block_on(client.get_services())?
                .services
                .iter()
                .find(|x| x.1.name == tag_service)
                .map(|x| x.1.name.to_owned())
                .ok_or(anyhow!("Could not find tag service {}", tag_service))?;
            loop {
                let start_time = Instant::now();

                match search(&rt, &client, &tag_service) {
                    Ok(hashes) => {
                        for hash in hashes {
                            if let Err(e) = evaluate_hash(
                                &rt,
                                &client,
                                &interrogator,
                                threshold,
                                &service_key,
                                &hash,
                                dry_run,
                            ) {
                                println!("Error evaluating hash: {:?}", e);
                            }
                        }
                    }
                    Err(e) => println!("Search error: {:?}", e),
                }

                let elapsed_time = start_time.elapsed();
                if elapsed_time < interval_duration {
                    let sleep_duration = interval_duration - elapsed_time;
                    println!("Sleeping for {:?}", sleep_duration);
                    std::thread::sleep(sleep_duration);
                }
            }
        }
    }
}
