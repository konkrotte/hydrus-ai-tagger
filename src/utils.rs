use std::{
    fs,
    io::{self, BufRead},
    path,
};

use anyhow::{anyhow, Result};
use image::{DynamicImage, ImageReader};
use indexmap::IndexMap;
use rayon::prelude::*;

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

pub fn parse_hashes_file(path: &path::PathBuf) -> Result<Vec<String>> {
    let bytes = fs::read(path)?;
    let lines = bytes.lines().collect::<io::Result<Vec<String>>>()?;
    Ok(lines)
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

pub fn decode_image(bytes: &[u8]) -> Result<DynamicImage> {
    let mut reader = ImageReader::new(io::Cursor::new(bytes));
    reader.no_limits();
    reader.with_guessed_format()?.decode().map_err(Into::into)
}
