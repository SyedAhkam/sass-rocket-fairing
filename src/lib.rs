mod context;

use rocket::{
    Rocket,
    Build,
    Orbit,
    log::PaintExt,
    yansi::Paint,
    fairing::{Fairing, Info, Kind},
};

use std::path::PathBuf;

// Re-exports
pub use sass_rs;
pub use context::{Context, ContextManager};

const DEFAULT_SASS_DIR: &str = "static/sass";
const DEFAULT_CSS_DIR: &str = "static/css";

/// Compiles a single sass file and returns the resultant `String`
pub fn compile_file(path_buf: PathBuf) -> Result<String, String> {
    sass_rs::compile_file(
        path_buf.as_path(),
        sass_rs::Options::default()
    )
}

/// Main user facing rocket `Fairing`
pub struct SassFairing;

#[rocket::async_trait]
impl Fairing for SassFairing {
    fn info(&self) -> Info {
        let kind = Kind::Ignite | Kind::Liftoff | Kind::Singleton;

        // Enable Request Kind in debug mode
        #[cfg(debug_assertions)] let kind = kind | Kind::Request;

        Info {
            name: "Sass Compiler",
            kind
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        use rocket::figment::value::magic::RelativePathBuf;

        // Get sass directory
        let sass_dir = rocket.figment()
            .extract_inner::<RelativePathBuf>("sass_dir")
            .map(|path| path.relative());
        
        let sass_path = match sass_dir {
            Ok(dir) => dir,
            Err(e) if e.missing() => DEFAULT_SASS_DIR.into(),
            Err(e) => {
                rocket::config::pretty_print_error(e);
                return Err(rocket)
            }
        };

        // Get css directory
        let css_dir = rocket.figment()
            .extract_inner::<RelativePathBuf>("css_dir")
            .map(|path| path.relative());
        
        let css_path = match css_dir {
            Ok(dir) => dir,
            Err(e) if e.missing() => DEFAULT_CSS_DIR.into(),
            Err(e) => {
                rocket::config::pretty_print_error(e);
                return Err(rocket)
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

        let ctx_manager = rocket.state::<ContextManager>()
            .expect("Sass Context not registered in on_ignite");

        let context = &*ctx_manager.context();

        let sass_dir = context.sass_dir.strip_prefix(std::env::current_dir().unwrap()).unwrap();
        let css_dir = context.css_dir.strip_prefix(std::env::current_dir().unwrap()).unwrap();

        rocket::info!("{}{}:", Paint::emoji("✨ "), Paint::magenta("Sass"));
        rocket::info_!("sass directory: {}", Paint::white(sass_dir.display()));
        rocket::info_!("css directory: {}", Paint::white(css_dir.display()));

        rocket::info_!("compiling initial files.");
        ctx_manager.compile_all_and_write();
    } 

    /// Calls `ContextManager.reload_if_needed` on new incoming request
    /// Only applicable in debug builds
    #[cfg(debug_assertions)]
    async fn on_request(&self, req: &mut rocket::Request<'_>, _data: &mut rocket::Data<'_>) { 
        let context_manager = req.rocket().state::<ContextManager>()
            .expect("Sass ContextManager not registered in on_ignite");
        
        context_manager.reload_if_needed();
    }
}