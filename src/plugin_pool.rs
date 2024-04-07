use crate::api::*;
use libloading::{Library, Symbol};
use std::fs;

pub struct PluginPool {
}


impl PluginPool {

    pub fn load_plugins(self, api: &PluginApi, path: &str) {
        let paths = fs::read_dir(path).unwrap();

        for path in paths {
            let file_name = path.unwrap().path().display().to_string();
            if file_name.starts_with("libisabelle-plugin-") {
                unsafe {
                    let lib = Library::new(file_name).unwrap();
                    let func: Symbol<IsabellePluginRegisterFn> = lib.get(b"register").unwrap();
                    func(api);
                }
            }
        }
    }
}
