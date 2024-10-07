use std::path;

use clap::Parser;
use image::ImageReader;
use interrogator::Interrogator;
use rocket::State;

mod interrogator;
#[macro_use]
extern crate rocket;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Path to the model folder
    #[arg(short, long)]
    model_dir: path::PathBuf,
}

#[get("/lookup?<hash>")]
fn lookup(hash: &str, interrogator: &State<Interrogator>) -> String {
    println!("{}", &hash);
    let image = ImageReader::open("./image.jpg").unwrap();
    interrogator.interrogate(image.decode().unwrap());
    hash.to_string()
}

#[launch]
fn rocket() -> _ {
    let args = Args::parse();
    let interrogator = Interrogator::init(args.model_dir).unwrap();
    rocket::build()
        .mount("/", routes![lookup])
        .manage(interrogator)
}
