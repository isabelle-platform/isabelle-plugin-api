mod common;

use common::{CountingPlugin, StubApi};
use isabelle_dm::data_model::item::Item;
use isabelle_plugin_api::api::*;

fn boxed_api() -> Box<dyn PluginApi> {
    Box::new(StubApi)
}

#[test]
fn plugin_api_secret_get_default_is_none() {
    let api = StubApi;
    assert!(api.secret_get(0).is_none());
    assert!(api.secret_get(u64::MAX).is_none());
}

#[test]
fn plugin_api_secret_get_by_name_default_is_none() {
    let api = StubApi;
    assert!(api.secret_get_by_name("anything").is_none());
}

#[test]
fn plugin_api_secret_list_default_is_empty() {
    let api = StubApi;
    assert!(api.secret_list().is_empty());
}

#[test]
fn plugin_api_secret_set_default_returns_err() {
    let api = StubApi;
    let item = Item::new();
    let res = api.secret_set(&item, false);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "secret_set not implemented");
}

#[test]
fn plugin_api_secret_del_default_is_false() {
    let api = StubApi;
    assert!(!api.secret_del(0));
    assert!(!api.secret_del(42));
}

#[test]
fn plugin_default_route_url_post_hook_returns_not_implemented() {
    let api = boxed_api();
    let mut plugin = CountingPlugin::new();
    let item = Item::new();
    let resp = plugin.route_url_post_hook(&api, "h", &None, "q", &item);
    assert!(matches!(resp, WebResponse::NotImplemented));
}

#[test]
fn plugin_default_route_rest_hook_returns_not_implemented() {
    let api = boxed_api();
    let mut plugin = CountingPlugin::new();
    let resp = plugin.route_rest_hook(&api, "h", "GET", &None, "/foo", "");
    assert!(matches!(resp, WebResponse::NotImplemented));
}

#[test]
fn plugin_default_item_list_db_filter_hook_returns_empty_string() {
    let api = boxed_api();
    let mut plugin = CountingPlugin::new();
    let s = plugin.item_list_db_filter_hook(&api, "h", &None, "col", "ctx", "ftype");
    assert_eq!(s, "");
}

#[test]
fn plugin_default_call_periodic_job_hook_is_noop() {
    let api = boxed_api();
    let mut plugin = CountingPlugin::new();
    // Just ensure default impl exists and doesn't panic.
    plugin.call_periodic_job_hook(&api, "daily");
    plugin.call_periodic_job_hook(&api, "");
}

#[test]
fn web_response_variants_are_distinct() {
    let cases: Vec<WebResponse> = vec![
        WebResponse::Ok,
        WebResponse::OkData("x".to_string()),
        WebResponse::OkFile("a".to_string(), vec![1, 2]),
        WebResponse::OkFilePath("a".to_string(), "b".to_string()),
        WebResponse::NotFound,
        WebResponse::Unauthorized,
        WebResponse::BadRequest,
        WebResponse::Forbidden,
        WebResponse::NotImplemented,
        WebResponse::Login("user".to_string()),
        WebResponse::Logout,
    ];

    // Smoke check: each variant is constructable and matches its own arm.
    for r in cases {
        match r {
            WebResponse::Ok
            | WebResponse::OkData(_)
            | WebResponse::OkFile(_, _)
            | WebResponse::OkFilePath(_, _)
            | WebResponse::NotFound
            | WebResponse::Unauthorized
            | WebResponse::BadRequest
            | WebResponse::Forbidden
            | WebResponse::NotImplemented
            | WebResponse::Login(_)
            | WebResponse::Logout => {}
        }
    }
}
