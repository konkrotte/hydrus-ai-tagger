use std::path;

use image::ImageReader;
use interrogator::Interrogator;
use rocket::State;
use clap::Parser;

mod interrogator;
#[macro_use]
extern crate rocket;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    model: path::PathBuf
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
    let interrogator = Interrogator::init("name", args.model.as_path().to_str().unwrap(), "");
    rocket::build()
        .mount("/", routes![lookup])
        .manage(interrogator)
}
