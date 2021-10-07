# Sass Rocket Fairing

`sass-rocket-fairing` is a Fairing/middleware for [rocket.rs](https://rocket.rs) facilitating sass compilation. It compiles your sass files on change automagically ✨

## Installing

Add the following to your Cargo.toml file
```toml
sass-rocket-fairing = "0.1"
```

OR using git

```toml
sass-rocket-fairing = {version = "0.1", git="https://github.com/SyedAhkam/sass-rocket-fairing.git"}
```

## Usage

```rs
#[macro_use]
extern crate rocket;

use sass_rocket_fairing::SassFairing;

#[launch]
fn rocket() -> _ {
    rocket::build().attach(SassFairing)
}
```

## Configuration

`SassFairing` takes advantage of rocket's advanced configuration system. There are two ways to configure it.

1. Using Rocket.toml (recommended)
> Add a Rocket.toml file in root directory of your crate and add the following to it:

```toml
[default]
sass_dir = "static/sass"
css_dir = "static/css"
```

2. Using enviroment variables
> Set the following environment variables:
- ROCKET_SASS_DIR
- ROCKET_CSS_DIR

### Where
- `sass_dir` is the folder where your sass files are to be located.

- `css_dir` is where your built css files are to be located.

## Thanks
I've stolen a big chunk of code from [rocket_dyn_templates](https://github.com/SergioBenitez/Rocket/tree/1a42009e9f729661868d339c77f5b6fc8757cebe/contrib/dyn_templates) and adapted it to my needs.

## Contributing
Feel free to send me a pull request! My code might be a little iffy but that's because I'm new to the rust ecosystem.

## License
Licensed under the most permissive license, MIT.
