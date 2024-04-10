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
            let file_name = path.unwrap().path().file_name().unwrap().to_string_lossy().to_string();
            info!("File name: {}", file_name);
            if file_name.starts_with("libisabelle_plugin_") {
                info!("Library: {}", file_name);
                unsafe {
                    let lib = Library::new(file_name).unwrap();
                    let func: Symbol<IsabellePluginRegisterFn> = lib.get(b"register").unwrap();
                    info!("Registering");
                    func(api);
                    info!("Registration done");
                }
            }
        }
    }
}
