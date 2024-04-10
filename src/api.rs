use std::collections::HashMap;
use isabelle_dm::data_model::item::Item;
use isabelle_dm::data_model::list_result::ListResult;
use isabelle_dm::data_model::process_result::ProcessResult;

pub enum WebResponse {
    Ok,
    NotFound,
    Unauthorized,
    BadRequest,
    Forbidden,
}

pub type IsabelleRouteItemPreEditHook = fn(
    api: &PluginApi,
    user: &Option<Item>,
    collection: &str,
    old_itm: Option<Item>,
    itm: &mut Item,
    del: bool,
    merge: bool) -> ProcessResult;
pub type IsabelleRouteItemPostEditHook = fn(
    api: &PluginApi,
    collection: &str,
    id: u64,
    del: bool);
pub type IsabelleRouteItemAuthHook = fn(
    api: &PluginApi,
    user: &Option<Item>,
    collection: &str,
    id: u64,
    new_item: Option<Item>,
    del: bool) -> bool;

pub type IsabelleRouteItemListFilterHook = fn(
    api: &PluginApi,
    user: &Option<Item>,
    collection: &str,
    context: &str,
    map: &mut HashMap<u64, Item>);

pub type IsabelleRouteUrlHook = fn(
    api: &PluginApi,
    user: &Option<Item>,
    query: &str) -> WebResponse;

pub type IsabelleRouteUnprotectedUrlHook = fn(
    api: &PluginApi,
    user: &Option<Item>,
    query: &str) -> WebResponse;

pub type IsabelleRouteUnprotectedUrlPostHook = fn(
    api: &PluginApi,
    user: &Option<Item>,
    query: &str,
    itm: &Item) -> WebResponse;

pub type IsabelleRouteCollectionReadHook = fn(
    api: &PluginApi,
    collection: &str,
    itm: &mut Item) -> bool;

pub type IsabelleRouteCallOtpHook = fn(
    api: &PluginApi,
    itm: &Item);

pub struct PluginApi {
    /* database */
    pub db_get_all_items: Box<dyn Fn(&str, &str, &str) -> ListResult>,
    pub db_get_items: Box<dyn Fn(&str, u64, u64, &str, &str, u64, u64) -> ListResult>,
    pub db_get_item: Box<dyn Fn(&str, u64) -> Option<Item>>,
    pub db_set_item: Box<dyn Fn(&str, &Item, &bool)>,
    pub db_del_item: Box<dyn Fn(&str, u64) -> bool>,

    /* globals */
    pub globals_get_public_url: Box<dyn Fn() -> String>,
    pub globals_get_settings: Box<dyn Fn() -> Item>,

    /* auth */
    pub auth_check_role: Box<dyn Fn(&Option<Item>, &str) -> bool>,
    pub auth_get_new_salt: Box<dyn Fn() -> String>,
    pub auth_get_password_hash: Box<dyn Fn(&str, &str) -> String>,
    pub auth_verify_password: Box<dyn Fn(&str, &str) -> bool>,

    /* exposed functions */
    pub fn_send_email: Box<dyn Fn(&str, &str, &str)>,
    pub fn_init_google: Box<dyn Fn() -> String>,
    pub fn_sync_with_google: Box<dyn Fn(bool, String, String)>,

    /* routes */
    pub route_register_item_pre_edit_hook: Box<dyn Fn(&str,
        IsabelleRouteItemPreEditHook) -> bool>,
    pub route_register_item_post_edit_hook: Box<dyn Fn(&str,
        IsabelleRouteItemPostEditHook) -> bool>,
    pub route_register_item_auth_hook: Box<dyn Fn(
        &str,
        IsabelleRouteItemAuthHook) -> bool>,
    pub route_register_item_list_filter_hook: Box<dyn Fn(
        &str,
        IsabelleRouteItemListFilterHook) -> bool>,
    pub route_register_url_hook: Box<dyn Fn(
        &str,
        IsabelleRouteUrlHook) -> bool>,
    pub route_register_unprotected_url_hook: Box<dyn Fn(
        &str,
        IsabelleRouteUnprotectedUrlHook) -> bool>,
    pub route_register_unprotected_url_post_hook: Box<dyn Fn(
        &str,
        IsabelleRouteUnprotectedUrlPostHook) -> bool>,

    pub route_register_collection_read_hook: Box<dyn Fn(
        &str,
        IsabelleRouteCollectionReadHook) -> bool>,
    pub route_register_call_otp_hook: Box<dyn Fn(
        &str,
        IsabelleRouteCallOtpHook) -> bool>,
}

impl PluginApi {
    pub fn new() -> Self {
        Self {
            /* database */
            db_get_all_items: Box::new(|_collection, _sort_key, _filter| {
                return ListResult {
                    map: HashMap::new(),
                    total_count: 0,
                };
            }),
            db_get_items: Box::new(|_collection, _id_min, _id_max, _sort_key, _filter, _skip, _limit| {
                return ListResult {
                    map: HashMap::new(),
                    total_count: 0,
                };
            }),
            db_get_item: Box::new(|_collection, _id| {
                return None;
            }),
            db_set_item: Box::new(|_collection, _itm, _merge| {
            }),
            db_del_item: Box::new(|_collection, _id| {
                return false;
            }),

            /* auth */
            auth_check_role: Box::new(|_user, _role| {
                return false;
            }),

            auth_get_new_salt: Box::new(|| {
                return "".to_string();
            }),

            auth_get_password_hash: Box::new(|_old, _salt| {
                return "".to_string();
            }),

            auth_verify_password: Box::new(|_pw, _hash| {
                return false;
            }),

            /* globals */
            globals_get_public_url: Box::new(|| {
                return "".to_string();
            }),

            globals_get_settings: Box::new(|| {
                return Item::new();
            }),

            /* exposed functions */

            fn_send_email: Box::new(|_to, _subject, _body| {
            }),

            fn_init_google: Box::new(|| {
                return "".to_string();
            }),

            fn_sync_with_google: Box::new(|_add, _name, _date_time| {
            }),

            /* routes */
            route_register_item_pre_edit_hook: Box::new(|_name, _hook| {
                return false;
            }),

            route_register_item_post_edit_hook: Box::new(|_name, _hook| {
                return false;
            }),

            route_register_item_auth_hook: Box::new(|_name, _hook| {
                return false;
            }),

            route_register_item_list_filter_hook: Box::new(|_name, _hook| {
                return false;
            }),

            route_register_url_hook: Box::new(|_name, _hook| {
                return false;
            }),

            route_register_unprotected_url_hook: Box::new(|_name, _hook| {
                return false;
            }),

            route_register_unprotected_url_post_hook: Box::new(|_name, _hook| {
                return false;
            }),

            route_register_collection_read_hook: Box::new(|_name, _hook| {
                return false;
            }),

            route_register_call_otp_hook: Box::new(|_name, _hook| {
                return false;
            }),
        }
    }
}

pub type IsabellePluginRegisterFn = fn(
    api: &PluginApi);

unsafe impl Send for PluginApi {}

