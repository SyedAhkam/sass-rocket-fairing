use rocket::{
    Rocket,
    Build,
    Orbit,
    fairing::{Fairing, Info, Kind}
};
use normpath::PathExt;
use walkdir::WalkDir;

use std::{
    fs,
    collections::HashMap,
    path::{Path, PathBuf}
};

// Re-export
pub use sass_rs;

const DEFAULT_SASS_DIR: &str = "static/sass";
const DEFAULT_CSS_DIR: &str = "static/css";

pub enum Error {

}

pub fn compile_file(path_buf: PathBuf) -> String {
    sass_rs::compile_file(
        path_buf.as_path(),
        sass_rs::Options::default()
    ).expect(format!("Failed to compile file: '{:?}'", path_buf).as_str())
}

pub struct Context {
    sass_dir: PathBuf,
    css_dir: PathBuf
}

impl Context {
    pub fn initialize(sass_dir: &Path, css_dir: &Path) -> Option<Self> {
        let sass_dir_buf = match sass_dir.normalize() {
            Ok(dir) => dir.into_path_buf(),
            Err(e) => {
                rocket::error!("Invalid sass directory '{}': {}.", sass_dir.display(), e);
                return None;
            }
        };
        
        let css_dir_buf = match css_dir.normalize() {
            Ok(dir) => dir.into_path_buf(),
            Err(e) => {
                rocket::error_!("Invalid css directory '{}': {}.", css_dir.display(), e);
                return None;
            }
        };

        Some(Self { sass_dir: sass_dir_buf, css_dir: css_dir_buf })
    }
}

pub struct ContextManager(Context);

impl ContextManager {
    pub fn new(ctx: Context) -> Self {
        Self(ctx)
    }

    pub fn context<'a>(&'a self) -> impl std::ops::Deref<Target=Context> + 'a {
        &self.0
    }

    pub fn compile_all(&self) -> Result<HashMap<String, String>, ()> {
        let mut compiled: HashMap<String, String> = HashMap::new();

        for entry in WalkDir::new(&*self.context().sass_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.metadata().unwrap().is_file() {
                let file_path = entry.path().to_str().unwrap().to_string();
                let result = compile_file(entry.into_path());

                compiled.insert(file_path, result);
            }
        }

        Ok(compiled)
    }

    pub fn write_compiled(compiled_files: HashMap<String, String>) {
        //TODO do the thing
    }

    pub fn compile_all_and_write(&self) -> Result<(), ()> {
        let compiled_files = self.compile_all();
        println!("{:#?}", compiled_files);

    //    self.write_compiled(compiled_files);

       Ok(())
    }
}

pub struct SassFairing;

#[rocket::async_trait]
impl Fairing for SassFairing {
    fn info(&self) -> Info {
        Info {
            name: "Sass Compiler",
            kind: Kind::Ignite | Kind::Liftoff | Kind::Singleton
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
        use rocket::{figment::Source, log::PaintExt, yansi::Paint};

        let ctx_manager = rocket.state::<ContextManager>()
            .expect("Sass Context not registered in on_ignite");

        rocket::info!("{}{}:", Paint::emoji("✨ "), Paint::magenta("Sassing"));
        rocket::info_!("sass directory: {}", Paint::white(&*ctx_manager.context().sass_dir.to_str().unwrap()));
        rocket::info_!("css directory: {}", Paint::white(&*ctx_manager.context().css_dir.to_str().unwrap()));

        match ctx_manager.compile_all_and_write() {
            Ok(_) => rocket::info!("Compiled sass files on liftoff"), 
            Err(e) => rocket::error!("Failed to compile sass files on liftoff: {}", "ERRROR") //FIXME
        };
    } 
}