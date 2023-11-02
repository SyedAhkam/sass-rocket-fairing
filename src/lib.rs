#[cfg(not(any(feature = "backend_rsass", feature = "backend_dart_sass")))]
compile_error!("No sass backend feature enabled. Enable one of `backend_rsass` or `backend_dart_sass`");

mod context;

use rocket::{
    fairing::{Fairing, Info, Kind},
    yansi::Paint,
    Build, Orbit, Rocket,
};

use std::path::PathBuf;

// Re-exports
pub use context::{Context, ContextManager};
#[cfg(feature = "backend_rsass")]
pub use rsass;

const DEFAULT_SASS_DIR: &str = "static/sass";
const DEFAULT_CSS_DIR: &str = "static/css";

/// Compiles a single sass file and returns the resultant `String`
/// Using the rsass format specified
pub fn compile_file(path_buf: PathBuf, backend: &SassBackend) -> Result<String, String> {
    match backend {
        #[cfg(feature = "backend_rsass")]
        SassBackend::RSass(format) => match rsass::compile_scss_path(path_buf.as_path(), *format) {
            Ok(res) => Ok(String::from_utf8(res).unwrap()),
            Err(e) => Err(e.to_string()),
        },
        #[cfg(feature = "backend_dart_sass")]
        SassBackend::DartSass => {
            use std::process::Command;
            let out = Command::new("sass")
                .arg(path_buf)
                .output()
                .map_err(|e| e.to_string())?;

            if !out.stderr.is_empty() {
                rocket::warn_!("Dart Sass stderr: {}", String::from_utf8_lossy(&out.stderr))
            }

            Ok(String::from_utf8_lossy(&out.stdout).to_string())
        }
    }


}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum SassBackend {
    #[cfg(feature = "backend_rsass")]
    RSass(rsass::output::Format),
    #[cfg(feature = "backend_dart_sass")]
    DartSass,
}

/// Main user facing rocket `Fairing`
pub struct SassFairing {
    backend: SassBackend,
}

impl SassFairing {
    /// Creates a new `SassFairing` with the specified backend configuration
    pub fn new(backend: SassBackend) -> Self {
        Self { backend }
    }
}

#[cfg(feature = "backend_rsass")]
impl Default for SassFairing {
    fn default() -> Self {
        Self {
            backend: SassBackend::RSass(rsass::output::Format::default()),
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

        if let Some(ctx) = Context::initialize(&sass_path, &css_path) {
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
            .expect("sass_dir is not defined");
        let css_dir = context
            .css_dir
            .strip_prefix(std::env::current_dir().unwrap())
            .expect("css_dir is not defined");

        rocket::info!("✨ {}:", Paint::magenta("Sass"));
        rocket::info_!("sass directory: {}", Paint::white(&sass_dir.display()));
        rocket::info_!("css directory: {}", Paint::white(&css_dir.display()));

        // Precompile sass files if in debug mode
        if cfg!(debug_assertions) {
            rocket::info_!("pre-compiling sass files");
            ctx_manager.compile_all_and_write(&self.backend);
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

        context_manager.reload_if_needed(&self.backend);
    }
}
