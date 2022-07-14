mod context;

use rocket::{
    fairing::{Fairing, Info, Kind},
    log::PaintExt,
    yansi::Paint,
    Build, Orbit, Rocket,
};

use std::path::PathBuf;

// Re-exports
// pub use sass_rs;
pub use context::{Context, ContextManager};
pub use rsass;

const DEFAULT_SASS_DIR: &str = "static/sass";
const DEFAULT_CSS_DIR: &str = "static/css";

/// Compiles a single sass file and returns the resultant `String`
/// Using the rsass format specified
pub fn compile_file(path_buf: PathBuf, format: rsass::output::Format) -> Result<String, String> {
    match rsass::compile_scss_path(path_buf.as_path(), format) {
        Ok(res) => Ok(String::from_utf8(res).unwrap()),
        Err(e) => Err(e.to_string()),
    }
}

/// Main user facing rocket `Fairing`
pub struct SassFairing {
    rsass_format: rsass::output::Format,
}

impl SassFairing {
    /// Creates a new `SassFairing` with the specified format
    pub fn new(format: rsass::output::Format) -> Self {
        Self {
            rsass_format: format,
        }
    }
}

impl Default for SassFairing {
    fn default() -> Self {
        Self {
            rsass_format: rsass::output::Format::default(),
        }
    }
}

#[rocket::async_trait]
impl Fairing for SassFairing {
    fn info(&self) -> Info {
        let kind = Kind::Ignite | Kind::Liftoff | Kind::Singleton;

        // Enable Request Kind in debug mode
        #[cfg(debug_assertions)]
        let kind = kind | Kind::Request;

        Info {
            name: "Sass Compiler",
            kind,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        use rocket::figment::value::magic::RelativePathBuf;

        // Get sass directory
        let sass_dir = rocket
            .figment()
            .extract_inner::<RelativePathBuf>("sass_dir")
            .map(|path| path.relative());

        let sass_path = match sass_dir {
            Ok(dir) => dir,
            Err(e) if e.missing() => DEFAULT_SASS_DIR.into(),
            Err(e) => {
                rocket::config::pretty_print_error(e);
                return Err(rocket);
            }
        };

        // Get css directory
        let css_dir = rocket
            .figment()
            .extract_inner::<RelativePathBuf>("css_dir")
            .map(|path| path.relative());

        let css_path = match css_dir {
            Ok(dir) => dir,
            Err(e) if e.missing() => DEFAULT_CSS_DIR.into(),
            Err(e) => {
                rocket::config::pretty_print_error(e);
                return Err(rocket);
            }
        };

        if let Some(ctx) = Context::initialize(&sass_path, &css_path, self.rsass_format) {
            Ok(rocket.manage(ContextManager::new(ctx)))
        } else {
            rocket::error!("Sass Initialization failed. Aborting launch.");
            Err(rocket)
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let ctx_manager = rocket
            .state::<ContextManager>()
            .expect("Sass Context not registered in on_ignite");

        let context = &*ctx_manager.context();

        let sass_dir = context
            .sass_dir
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap();
        let css_dir = context
            .css_dir
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap();

        rocket::info!("{}{}:", Paint::emoji("✨ "), Paint::magenta("Sass"));
        rocket::info_!("sass directory: {}", Paint::white(sass_dir.display()));
        rocket::info_!("css directory: {}", Paint::white(css_dir.display()));

        // Precompile sass files if in debug mode
        if cfg!(debug_assertions) {
            rocket::info_!("pre-compiling sass files");
            ctx_manager.compile_all_and_write();
        }
    }

    /// Calls `ContextManager.reload_if_needed` on new incoming request.
    /// Only applicable in debug builds
    #[cfg(debug_assertions)]
    async fn on_request(&self, req: &mut rocket::Request<'_>, _data: &mut rocket::Data<'_>) {
        let context_manager = req
            .rocket()
            .state::<ContextManager>()
            .expect("Sass ContextManager not registered in on_ignite");

        context_manager.reload_if_needed();
    }
}
