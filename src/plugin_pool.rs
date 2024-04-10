use crate::api::*;
use libloading::{Library, Symbol};
use std::fs;
use log::info;

pub struct PluginPool {
}


impl PluginPool {

    pub fn load_plugins(&self, api: &PluginApi, path: &str) {
        let paths = fs::read_dir(path).unwrap();
        info!("Loading plugins from {}", path);
        for path in paths {
            let file_name = path.as_ref().unwrap().path().file_name().unwrap().to_string_lossy().to_string();
            info!("File name: {}", file_name);
            if file_name.starts_with("libisabelle_plugin_") {
                info!("Library: {}", file_name);
                unsafe {
                    let full = path.as_ref().unwrap().path().canonicalize().unwrap().to_string_lossy().to_string();
                    info!("Loading library {}", full);
                    match Library::new(file_name.clone()) {
                        Ok(lib) => {
                            info!("Library loaded");
                            match lib.get::<Symbol<IsabellePluginRegisterFn>>(b"register") {
                                Ok(func) => {
                                    info!("Registering");
                                    func(api);
                                    info!("Registration done");
                                }
                                Err(e) => {
                                    info!("Symbol error: {}", e);
                                }
                            };
                        }
                        Err(e) => {
                            info!("Error: {}", e);
                        }
                    }
                }
            }
        }
    }
}
