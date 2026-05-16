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
use log::{error, info, warn};
use std::fs;

#[repr(C)]
/// Plugin pool structure
pub struct PluginPool {
    pub plugins: Vec<Box<dyn Plugin>>,
}

/// Description of a single plugin-loading failure.
#[derive(Debug, Clone)]
pub struct PluginLoadFailure {
    /// Path of the file that failed to load (best effort: canonical when
    /// available, otherwise the path as returned by the directory iterator).
    pub path: String,
    /// Human-readable reason for the failure.
    pub error: String,
}

/// Outcome of a `load_plugins` call.
#[derive(Debug, Clone, Default)]
pub struct PluginLoadResult {
    /// Number of files that matched the plugin naming convention and were
    /// attempted.
    pub considered: usize,
    /// Number of plugins that were successfully registered.
    pub loaded: usize,
    /// Per-file failures encountered while loading.
    pub failures: Vec<PluginLoadFailure>,
}

impl PluginLoadResult {
    /// Number of plugins that failed to load.
    pub fn failed(&self) -> usize {
        self.failures.len()
    }

    /// True when no plugin failed to load.
    pub fn is_ok(&self) -> bool {
        self.failures.is_empty()
    }
}

impl PluginPoolApi for PluginPool {
    fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }
}

impl PluginPool {
    /// Load plugins from the given path, pass the API to them.
    ///
    /// Returns a [`PluginLoadResult`] describing how many candidates were
    /// considered, how many were successfully registered, and the per-file
    /// failures encountered (if any). All failures are also logged.
    pub fn load_plugins(&mut self, path: &str) -> PluginLoadResult {
        let mut result = PluginLoadResult::default();
        info!("Loading plugins from {}", path);

        let paths = match fs::read_dir(path) {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("Failed to read plugin directory {}: {}", path, e);
                error!("{}", msg);
                result.failures.push(PluginLoadFailure {
                    path: path.to_string(),
                    error: msg,
                });
                return result;
            }
        };

        for entry in paths {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    let msg = format!("Failed to read directory entry in {}: {}", path, e);
                    warn!("{}", msg);
                    result.failures.push(PluginLoadFailure {
                        path: path.to_string(),
                        error: msg,
                    });
                    continue;
                }
            };
            let entry_path = entry.path();
            let file_name = match entry_path.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => {
                    warn!(
                        "Skipping entry without a file name in {}: {}",
                        path,
                        entry_path.display()
                    );
                    continue;
                }
            };

            if !file_name.starts_with("libisabelle_plugin_") {
                continue;
            }

            result.considered += 1;
            info!("Found plugin candidate: {}", file_name);

            let full = match entry_path.canonicalize() {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(e) => {
                    let path_str = entry_path.to_string_lossy().to_string();
                    let msg =
                        format!("Failed to canonicalize plugin path {}: {}", path_str, e);
                    error!("{}", msg);
                    result.failures.push(PluginLoadFailure {
                        path: path_str,
                        error: msg,
                    });
                    continue;
                }
            };

            info!("Loading library {}", full);
            unsafe {
                match Library::new(&full) {
                    Ok(l) => {
                        let lib = Box::leak(Box::new(l));
                        match lib
                            .get::<Symbol<unsafe extern "C" fn(&mut dyn PluginPoolApi) -> ()>>(
                                b"register",
                            ) {
                            Ok(func) => {
                                func(self);
                                result.loaded += 1;
                                info!("Plugin registered from {}", full);
                            }
                            Err(e) => {
                                let msg = format!(
                                    "Plugin {} is missing the `register` symbol: {}",
                                    full, e
                                );
                                error!("{}", msg);
                                result.failures.push(PluginLoadFailure {
                                    path: full.clone(),
                                    error: msg,
                                });
                            }
                        };
                    }
                    Err(e) => {
                        let msg = format!("Failed to load plugin library {}: {}", full, e);
                        error!("{}", msg);
                        result.failures.push(PluginLoadFailure {
                            path: full.clone(),
                            error: msg,
                        });
                    }
                }
            }
        }

        info!(
            "Plugin loading from {} finished: {} candidate(s), {} loaded, {} failed",
            path,
            result.considered,
            result.loaded,
            result.failed()
        );
        if !result.is_ok() {
            warn!(
                "{} plugin(s) failed to load from {}",
                result.failed(),
                path
            );
        }
        result
    }

    pub fn ping_plugins(&mut self) {
        for plugin in &mut self.plugins {
            plugin.ping_test();
        }
    }
}
