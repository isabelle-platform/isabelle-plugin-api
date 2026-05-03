mod common;

use common::CountingPlugin;
use isabelle_plugin_api::api::PluginPoolApi;
use isabelle_plugin_api::plugin_pool::PluginPool;
use std::fs;

#[test]
fn register_appends_plugins_in_order() {
    let mut pool = PluginPool {
        plugins: Vec::new(),
    };
    assert_eq!(pool.plugins.len(), 0);

    pool.register(Box::new(CountingPlugin::new()));
    assert_eq!(pool.plugins.len(), 1);

    pool.register(Box::new(CountingPlugin::new()));
    pool.register(Box::new(CountingPlugin::new()));
    assert_eq!(pool.plugins.len(), 3);
}

#[test]
fn ping_plugins_invokes_each_plugin_once_per_call() {
    let mut pool = PluginPool {
        plugins: Vec::new(),
    };
    pool.register(Box::new(CountingPlugin::new()));
    pool.register(Box::new(CountingPlugin::new()));

    pool.ping_plugins();
    pool.ping_plugins();
    pool.ping_plugins();

    // Downcast each plugin back to CountingPlugin via raw pointer trickery
    // would be brittle; instead, check via a side channel the plugin exposes.
    // We can't downcast `Box<dyn Plugin>` without Any, so re-verify by
    // pinging a known instance through a new pool with one plugin we keep
    // a handle to.
    let mut single_pool = PluginPool {
        plugins: Vec::new(),
    };
    let counter = Box::new(CountingPlugin::new());
    single_pool.register(counter);
    single_pool.ping_plugins();
    single_pool.ping_plugins();
    // Inspect via the pool's own slot.
    // SAFETY: we just registered exactly one CountingPlugin.
    let raw: *const CountingPlugin =
        single_pool.plugins[0].as_ref() as *const _ as *const CountingPlugin;
    unsafe {
        assert_eq!((*raw).pings, 2);
    }
}

#[test]
fn load_plugins_with_empty_directory_is_noop() {
    let dir = std::env::temp_dir().join(format!(
        "isabelle-plugin-api-test-empty-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let mut pool = PluginPool {
        plugins: Vec::new(),
    };
    pool.load_plugins(dir.to_str().unwrap());
    assert_eq!(pool.plugins.len(), 0);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_plugins_skips_unrelated_files() {
    let dir = std::env::temp_dir().join(format!(
        "isabelle-plugin-api-test-skip-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    // Files that do not match the `libisabelle_plugin_` prefix must be ignored.
    fs::write(dir.join("README.txt"), b"not a plugin").unwrap();
    fs::write(dir.join("libsomething_else.so"), b"not a plugin").unwrap();

    let mut pool = PluginPool {
        plugins: Vec::new(),
    };
    pool.load_plugins(dir.to_str().unwrap());
    assert_eq!(pool.plugins.len(), 0);

    let _ = fs::remove_dir_all(&dir);
}
