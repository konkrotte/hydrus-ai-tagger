use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use hydrus_api::api_core::{
    common::FileIdentifier,
    endpoints::{
        adding_tags::AddTagsRequestBuilder,
        searching_and_fetching_files::{FileSearchOptions, SearchQueryEntry},
    },
};
use log::{debug, warn};
use rayon::prelude::*;
use tokio::runtime::Runtime;

use crate::{
    interrogator::Interrogator,
    utils::{decode_image, filter_and_process_tags, get_rating},
};

pub struct Tagger {
    rt: Arc<Runtime>,
    client: Arc<hydrus_api::Client>,
    interrogator: Arc<Interrogator>,
    threshold: f32,
}

impl Tagger {
    pub fn new(
        rt: Arc<Runtime>,
        client: Arc<hydrus_api::Client>,
        interrogator: Arc<Interrogator>,
        threshold: f32,
    ) -> Self {
        Self {
            rt,
            client,
            interrogator,
            threshold,
        }
    }
    pub fn tag_image(&self, service_key: &str, hash: &str, dry_run: bool) -> Result<Vec<String>> {
        debug!("Tagging {}", hash);

        let record = self
            .rt
            .block_on(self.client.get_file(FileIdentifier::hash(hash)))
            .context("Error getting image file from Hydrus API")?;

        let image = decode_image(&record.bytes)
            .or_else(|_| {
                warn!("Failed decoding original image, falling back to using hydrus render");
                let rendered = self
                    .rt
                    .block_on(self.client.get_render(FileIdentifier::hash(hash)))
                    .context("Error rendering file")?;
                decode_image(&rendered.bytes)
            })
            .context("Failed to decode image")?;

        let (ratings, tags) = self
            .interrogator
            .interrogate(&image)
            .context("Failed interrogating model")?;

        let mut filtered_tags = filter_and_process_tags(tags, self.threshold);

        if let Some(ratings) = ratings {
            filtered_tags.push(get_rating(&ratings)?);
        }

        let request = AddTagsRequestBuilder::default()
            .add_hash(hash)
            .add_tags(service_key.to_string(), filtered_tags.clone())
            .build();

        debug!("Tags to be added: {:?}", request.service_keys_to_tags);

        if !dry_run {
            self.rt
                .block_on(self.client.add_tags(request))
                .context("Failed adding tags")?;
        }

        Ok(filtered_tags)
    }

    pub fn get_untagged_images(&self, service_key: &str) -> Result<Vec<String>> {
        let hashes = self
            .rt
            .block_on(self.client.search_file_hashes(
                vec![
                    SearchQueryEntry::Tag(String::from("system:untagged")),
                    SearchQueryEntry::Tag(String::from("system:filetype is image")),
                ],
                FileSearchOptions::new().tag_service_key(service_key.to_string()),
            ))?
            .hashes;
        Ok(hashes)
    }

    pub fn get_tag_service_key_from_name(&self, tag_service: &String) -> Result<String> {
        let service_key = self
            .rt
            .block_on(self.client.get_services())?
            .services
            .par_iter()
            .find_any(|x| x.1.name == *tag_service)
            .map(|x| x.0.to_owned())
            .ok_or(anyhow!("Could not find tag service {}", tag_service))?;
        Ok(service_key)
    }
}
