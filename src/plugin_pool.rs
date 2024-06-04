/*
 * Isabelle project
 *
 * Copyright 2024 Maxim Menshikov
 *
 * Permission is hereby granted, free of charge, to any person obtaining
 * a copy of this software and associated documentation files (the “Software”),
 * to deal in the Software without restriction, including without limitation
 * the rights to use, copy, modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software, and to permit persons to whom the
 * Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included
 * in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS
 * OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
 * DEALINGS IN THE SOFTWARE.
 */
use crate::api::*;
use libloading::{Library, Symbol};
use std::fs;
use log::info;

#[repr(C)]
/// Plugin pool structure
pub struct PluginPool {
}


impl PluginPool {

    /// Load plugins from the given path, pass the API to them
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
