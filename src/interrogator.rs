use std::{any::Any, collections::HashMap, ffi::CString};

use image::{DynamicImage, GenericImageView};
use ndarray::Array;
use ort::{inputs, GraphOptimizationLevel, Session};

pub struct Interrogator {
    name: String,
    pub model: Session,
    tags: String, // FIXME: should be something else
}

impl Interrogator {
    pub fn init(name: &str, model_file: &str, _tags_file: &str) -> Self {
        let model = Session::builder()
            .unwrap()
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .unwrap()
            .with_intra_threads(4)
            .unwrap()
            .commit_from_file(model_file)
            .unwrap();

        Interrogator {
            name: name.to_string(),
            model,
            tags: "".to_string(),
        }
    }

    pub fn interrogate(
        &self,
        original_image: DynamicImage,
    ) -> Option<(HashMap<String, f32>, HashMap<String, f32>)> {
        let size = self.model.inputs[0].input_type.tensor_dimensions().unwrap()[1];
        let size = size as usize;
        let image = original_image.resize_exact(
            size as u32,
            size as u32,
            image::imageops::FilterType::CatmullRom,
        );

        let mut input = Array::zeros((1, size, size, 3));
        for pixel in image.pixels() {
            let x = pixel.0 as usize;
            let y = pixel.1 as usize;
            let [r, g, b, _] = pixel.2 .0;
            input[[0, y, x, 0]] = (b as f32) / 255.;
            input[[0, y, x, 1]] = (g as f32) / 255.;
            input[[0, x, y, 2]] = (r as f32) / 255.;
        }

        let input_name = &self.model.inputs[0].name;
        let output_name = &self.model.outputs[0].name;
        let outputs = self
            .model
            .run(inputs![input_name => input.view()].unwrap())
            .unwrap();
        let output = &outputs[0];
        println!("{:?}", output.memory_info());
        println!("{:?}", output.type_id());
        println!("{:?}", output.dtype().unwrap());
        println!("{:?}", output.shape().unwrap());
        None
    }
}
