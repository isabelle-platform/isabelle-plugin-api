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
    user: &Option<Item>,
    collection: &str,
    old_itm: Option<Item>,
    itm: &mut Item,
    del: bool,
    merge: bool) -> ProcessResult;
pub type IsabelleRouteItemPostEditHook = fn(
    collection: &str,
    id: u64,
    del: bool);
pub type IsabelleRouteItemAuthHook = fn(
    user: &Option<Item>,
    collection: &str,
    id: u64,
    new_item: Option<Item>,
    del: bool) -> bool;

pub type IsabelleRouteItemListFilterHook = fn(
    user: &Option<Item>,
    collection: &str,
    context: &str,
    map: &mut HashMap<u64, Item>);

pub type IsabelleRouteUrlHook = fn(
    user: &Option<Item>,
    query: &str) -> WebResponse;

pub type IsabelleRouteUnprotectedUrlHook = fn(
    user: &Option<Item>,
    query: &str) -> WebResponse;

pub type IsabelleRouteUnprotectedUrlPostHook = fn(
    user: &Option<Item>,
    query: &str,
    itm: &Item) -> WebResponse;

pub type IsabelleRouteCollectionReadHook = fn(
    collection: &str,
    itm: &mut Item) -> bool;

pub type IsabelleRouteCallOtpHook = fn(
    itm: &Item);

pub struct PluginApi {
    /* database */
    pub db_get_all_items: Box<dyn Fn(&str, &str, &str) -> Item>,
    pub db_get_items: Box<dyn Fn(&str, u64, u64, &str, &str, u64, u64) -> ListResult>,
    pub db_get_item: Box<dyn Fn(&str, u64) -> Item>,
    pub db_set_item: Box<dyn Fn(&str, &Item, &bool)>,
    pub db_del_item: Box<dyn Fn(&str, u64)>,

    /* globals */
    pub globals_get_public_url: Box<dyn Fn() -> String>,

    /* exposed functions */
    pub fn_send_email: Box<dyn Fn(&str, &str, &str)>,

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
        IsabelleRouteUnprotectedUrlHook)>,
    pub route_register_unprotected_url_post_hook: Box<dyn Fn(
        &str,
        IsabelleRouteUnprotectedUrlPostHook)>,

    pub route_register_collection_read_hook: Box<dyn Fn(
        &str,
        IsabelleRouteCollectionReadHook)>,
    pub route_register_call_otp_hook: Box<dyn Fn(
        &str,
        IsabelleRouteCallOtpHook)>,
}

pub type IsabellePluginRegisterFn = fn(
    api: &PluginApi);

