#[macro_use]
extern crate rocket;

use rocket::fs::{FileServer, relative};

use sass_rocket_fairing::SassFairing;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(SassFairing::default())
        .mount("/", routes![index])
        .mount("/static", FileServer::from(relative!("examples/static")))
}