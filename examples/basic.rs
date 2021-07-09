#[macro_use]
extern crate rocket;

use sass_rocket_fairing::SassFairing;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(SassFairing)
        .mount("/", routes![index])
}