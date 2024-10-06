use image::ImageReader;
use interrogator::Interrogator;
use rocket::State;

mod interrogator;
#[macro_use]
extern crate rocket;

#[get("/lookup?<hash>")]
fn lookup(hash: &str, interrogator: &State<Interrogator>) -> String {
    println!("{}", &hash);
    let image = ImageReader::open("./image.jpg").unwrap();
    interrogator.interrogate(image.decode().unwrap());
    hash.to_string()
}

#[launch]
fn rocket() -> _ {
    let interrogator = Interrogator::init("name", "models/Z3D-E621-Convnext.onnx", "");
    rocket::build()
        .mount("/", routes![lookup])
        .manage(interrogator)
}
