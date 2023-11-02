use normpath::PathExt;

use std::path::{Path, PathBuf};
use crate::SassBackend;

/// A Shared reference containing configuration data
pub struct Context {
    pub sass_dir: PathBuf,
    pub css_dir: PathBuf,
    pub backend: SassBackend,
}

impl Context {
    /// Initializes the `Context` while checking for bad configuration
    pub fn initialize(sass_dir: &Path, css_dir: &Path, backend: SassBackend) -> Option<Self> {
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

        Some(Self { sass_dir: sass_dir_buf, css_dir: css_dir_buf, backend })
    }
}

pub use self::manager::ContextManager;

#[cfg(not(debug_assertions))]
mod manager {
    use std::ops::Deref;
    use crate::Context;

    pub struct ContextManager(Context);

    impl ContextManager {
        pub fn new(ctx: Context) -> ContextManager {
            ContextManager(ctx)
        }

        pub fn context<'a>(&'a self) -> impl Deref<Target=Context> + 'a {
            &self.0
        }

        pub fn is_reloading(&self) -> bool {
            false
        }

        // This method is just a quickfix to get rid of not-defined errors
        pub fn compile_all_and_write(&self) {}
    }
}

#[cfg(debug_assertions)]
mod manager {
    use std::sync::{RwLock, Mutex, mpsc};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::fs;

    use std::io::Write;

    use notify::{raw_watcher, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
    use walkdir::WalkDir;

    use super::Context;

    /// Manages the `Context`
    pub struct ContextManager{
        context: RwLock<Context>,
        watcher: Option<(RecommendedWatcher, Mutex<mpsc::Receiver<RawEvent>>)>
    }

    impl ContextManager {
        pub fn new(ctx: Context) -> Self {
            let (tx, rx) = mpsc::channel();
            let watcher = raw_watcher(tx).and_then(|mut watcher| {
                watcher.watch(ctx.sass_dir.canonicalize()?, RecursiveMode::Recursive)?;

                Ok(watcher)
            });

            let watcher = match watcher {
                Ok(watcher) => Some((watcher, Mutex::new(rx))),
                Err(e) => {
                    rocket::warn!("Failed to enable live sass compiling: {}", e);
                    rocket::debug_!("Reload error: {:?}", e);
                    rocket::warn_!("Live sass compiling is unawailable.");

                    None
                }
            };

            Self { context: RwLock::new(ctx), watcher }
        }

        /// Returns `Context` as read only
        pub fn context(&self) -> impl std::ops::Deref<Target=Context> + '_ {
            self.context.read().unwrap()
        } 
        
        /// Returns `Context` as mutable
        pub fn context_mut(&self) -> impl std::ops::DerefMut<Target=Context> + '_ {
            self.context.write().unwrap()
        } 

        /// Compiles all files in `sass_dir`
        pub fn compile_all(&self) -> Result<HashMap<String, String>, ()> {
            let mut compiled: HashMap<String, String> = HashMap::new();
            let sass_dir = &*self.context().sass_dir;
            let backend = &self.context().backend;

            for entry in WalkDir::new(sass_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.metadata().unwrap().is_file() {
                    let file_name = entry.path().file_name().unwrap().to_str().unwrap().to_string();
                    let result = match crate::compile_file(entry.into_path(), backend) {
                        Ok(result) => result,
                        Err(e) => {
                            rocket::error!("Failed to compile file '{}'", file_name);
                            rocket::error!("Sass error: {:?}", e);

                            break;
                        }
                    };

                    compiled.insert(file_name, result);
                }
            }

            Ok(compiled)
        }

        /// Writes all compiled files to `css_dir`
        pub fn write_compiled(&self, compiled_files: HashMap<String, String>) {
            let css_dir = &*self.context().css_dir;

            for (sass_file_name, compiled) in compiled_files {
                let mut sass_file_name_path = PathBuf::new();

                sass_file_name_path.push(sass_file_name);
                sass_file_name_path.set_extension("css");

                let css_file_path = css_dir.join(sass_file_name_path);

                let mut file = fs::File::create(&css_file_path)
                    .expect(format!("Failed to create css file: '{:?}'", css_file_path).as_str());

                file.write_all(compiled.as_bytes())
                    .expect(format!("Failed to write file: {:?}", css_file_path).as_str());
            }
        }

        /// Shorthand for `compile_all` + `write_compiled`
        pub fn compile_all_and_write(&self) {
            if let Ok(compiled_files) = self.compile_all() {
                self.write_compiled(compiled_files);
            }

        }

        /// Returns `true` if reloading
        pub fn is_reloading(&self) -> bool {
            self.watcher.is_some()
        }

        /// Checks for any changes on `sass_dir`. 
        /// If found, compiles again (reloads)
        pub fn reload_if_needed(&self) {
            let sass_changes = self.watcher.as_ref()
                .map(|(_, rx)| rx.lock().expect("Failed to lock receiver").try_iter().count() > 0 );

            if let Some(true) = sass_changes {
                rocket::info_!("Change detected: compiling sass files.");
                
                self.compile_all_and_write();
            }
        }
    }
}
