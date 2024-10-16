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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_rating() {
        let mut ratings = IndexMap::new();
        ratings.insert("general".to_string(), 0.9);
        ratings.insert("safe".to_string(), 0.7);
        ratings.insert("questionable".to_string(), 0.3);

        let result = get_rating(&ratings).unwrap();
        assert_eq!(result, "rating:general");
    }

    #[test]
    fn test_get_rating_empty() {
        let ratings = IndexMap::new();
        let result = get_rating(&ratings);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_hashes_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("hashes.txt");
        fs::write(
            &file_path,
            "245691b473fe6c2717bc350b2fb93f261277dab65f816cc023564b09cbdcb7af
7341a188d3be385f7f8264bc5383da8e18d2649e686206d2a9e2143dbf8726b1
a74b62685203edd41fb684a21377aa1fc2d699d2e11b8aab14d683c5505fce69",
        )
        .unwrap();

        let result = parse_hashes_file(&file_path).unwrap();
        assert_eq!(
            result,
            vec![
                "245691b473fe6c2717bc350b2fb93f261277dab65f816cc023564b09cbdcb7af",
                "7341a188d3be385f7f8264bc5383da8e18d2649e686206d2a9e2143dbf8726b1",
                "a74b62685203edd41fb684a21377aa1fc2d699d2e11b8aab14d683c5505fce69"
            ]
        );
    }

    #[test]
    fn test_filter_and_process_tags() {
        let mut tags = IndexMap::new();
        tags.insert("tag_one".to_string(), 0.9);
        tags.insert("tag_two".to_string(), 0.7);
        tags.insert("0_0".to_string(), 0.8);
        tags.insert("low_confidence".to_string(), 0.3);

        let result = filter_and_process_tags(tags, 0.5);
        assert_eq!(result, vec!["tag one", "tag two", "0_0"]);
    }

    #[test]
    fn test_decode_image() {
        let image_data = include_bytes!("../tests/test_image.jpg");
        let result = decode_image(image_data);
        assert!(result.is_ok());
    }
}
