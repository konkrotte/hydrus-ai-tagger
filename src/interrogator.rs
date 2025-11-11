use std::{fs, path::Path, thread, time::Instant};

use anyhow::{anyhow, ensure, Result};
use image::{
    imageops::{self, FilterType},
    DynamicImage, GenericImageView, ImageBuffer, Rgba,
};
use indexmap::IndexMap;
use log::{debug, info};
use ndarray::{Array, Array4};
use ort::execution_providers::{
    CUDAExecutionProvider, CoreMLExecutionProvider, TensorRTExecutionProvider,
};
use ort::inputs;
use ort::session::{builder::GraphOptimizationLevel, Session};
use serde::{Deserialize, Deserializer, Serialize};

type InterrogateReturn = Result<(Option<IndexMap<String, f32>>, IndexMap<String, f32>)>;

pub struct Interrogator {
    model: Session,
    ratings_flag: bool,
    number_of_ratings: usize,
    tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelInfo {
    #[serde(rename = "modelname")]
    name: String,
    #[serde(rename = "modelfile")]
    model_file: String,
    source: String,
    #[serde(rename = "tagsfile")]
    tags_file: String,
    #[serde(rename = "ratingsflag")]
    #[serde(deserialize_with = "from_int_bool")]
    ratings_flag: bool,
    #[serde(rename = "numberofratings")]
    number_of_ratings: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct Tag {
    #[serde(rename = "tag_id")]
    id: usize,
    name: String,
    category: usize,
    count: usize,
}

fn from_int_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let x: isize = Deserialize::deserialize(deserializer)?;
    match x {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(serde::de::Error::custom(format!(
            "Invalid boolean value: {x}"
        ))),
    }
}

fn prepare_image(image: &DynamicImage, size: u32) -> Result<Array4<f32>> {
    let white = Rgba([255, 255, 255, 255]);
    let image = image.resize(size, size, FilterType::Lanczos3);
    let (width, height) = image.dimensions();
    let mut square_image = ImageBuffer::from_pixel(size, size, white);
    let x_offset = (size - width) / 2;
    let y_offset = (size - height) / 2;
    imageops::overlay(&mut square_image, &image, x_offset.into(), y_offset.into());
    let image = DynamicImage::ImageRgba8(square_image);
    let mut input = Array::zeros((1, size.try_into()?, size.try_into()?, 3));

    for pixel in image.pixels() {
        let x = pixel.0 as usize;
        let y = pixel.1 as usize;
        let [r, g, b, _] = pixel.2 .0;
        input[[0, y, x, 0]] = f32::from(b);
        input[[0, y, x, 1]] = f32::from(g);
        input[[0, y, x, 2]] = f32::from(r);
    }

    Ok(input)
}

impl Interrogator {
    pub fn init(model_dir: &Path) -> Result<Self> {
        ensure!(
            model_dir.is_dir(),
            "Supplied model path does not exist or is not a directory"
        );

        let model_info_file = model_dir.join("info.json");
        let model_info: ModelInfo = serde_json::from_str(&fs::read_to_string(model_info_file)?)?;

        info!("Loading model {} by {}", model_info.name, model_info.source);

        let tags_file = model_dir.join(model_info.tags_file);
        let mut csv_rdr = csv::Reader::from_path(tags_file)?;
        let tags: Vec<String> = csv_rdr
            .deserialize()
            .filter_map(|result: Result<Tag, csv::Error>| result.ok().map(|tag| tag.name))
            .collect();
        let model_file = model_dir.join(model_info.model_file);
        let mut execution_providers = Vec::new();

        #[cfg(target_os = "macos")]
        execution_providers.push(CoreMLExecutionProvider::default().build());

        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            execution_providers.push(TensorRTExecutionProvider::default().build());
            execution_providers.push(CUDAExecutionProvider::default().build());
        }

        let model = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(thread::available_parallelism()?.get())?
            .with_execution_providers(execution_providers)?
            .commit_from_file(model_file)?;

        Ok(Interrogator {
            model,
            ratings_flag: model_info.ratings_flag,
            number_of_ratings: model_info.number_of_ratings,
            tags,
        })
    }

    pub fn interrogate(&self, image: &DynamicImage) -> InterrogateReturn {
        let size = self.model.inputs[0]
            .input_type
            .tensor_dimensions()
            .ok_or(anyhow!("No input tensor dimensions"))?[1];

        let input = prepare_image(image, size.try_into()?)?;
        let input_name = &self.model.inputs[0].name;
        let time = Instant::now();
        let outputs = self.model.run(inputs![input_name => input.view()]?)?;
        debug!("Inference took {} s", time.elapsed().as_secs_f64());
        let output = &outputs[0];
        let confidences = output.try_extract_tensor::<f32>()?.to_owned();
        let mut result = IndexMap::new();

        for (tag, &confidence) in self.tags.iter().zip(confidences.iter()) {
            result.insert(tag.clone(), confidence);
        }

        if self.ratings_flag {
            let mut ratings = IndexMap::new();
            let mut regular_tags = IndexMap::new();

            for (key, value) in result {
                if ratings.len() < self.number_of_ratings {
                    ratings.insert(key, value);
                } else {
                    regular_tags.insert(key, value);
                }
            }

            Ok((Some(ratings), regular_tags))
        } else {
            Ok((None, result))
        }
    }
}
