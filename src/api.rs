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

/* database */
pub type IsabelleDbGetAllItemsFn = fn(
    collection: &str,
    sort_key: &str,
    filter: &str) -> Item;
pub type IsabelleDbGetItemsFn = fn(
    collection: &str,
    id_min: u64,
    id_max: u64,
    sort_key: &str,
    filter: &str,
    skip: u64,
    limit: u64) -> ListResult;
pub type IsabelleDbGetItemFn = fn(
    collection: &str,
    id: u64) -> Item;
pub type IsabelleDbDelItemFn = fn(
    collection: &str,
    id: u64) -> bool;
pub type IsabelleDbSetItemFn = fn(
    collection: &str,
    itm: &Item,
    merge: &bool);

/* globals */
pub type IsabelleGlobalsGetPublicUrl = fn() -> String;

/* exposed functions */
pub type IsabelleFnSendEmail = fn(
    to: &str,
    subject: &str,
    body: &str);

/* route API */
pub type IsabelleRouteItemPreEditHook = fn(
    user: &Option<Item>,
    collection: &str,
    old_itm: Option<Item>,
    itm: &mut Item,
    del: bool,
    merge: bool) -> ProcessResult;
pub type IsabelleRouteRegisterItemPreEditHook = fn(
    name: &str,
    hook: IsabelleRouteItemPreEditHook) -> bool;

pub type IsabelleRouteItemPostEditHook = fn(
    collection: &str,
    id: u64,
    del: bool);
pub type IsabelleRouteRegisterItemPostEditHook = fn(
    name: &str,
    hook: IsabelleRouteItemPostEditHook) -> bool;

pub type IsabelleRouteItemAuthHook = fn(
    user: &Option<Item>,
    collection: &str,
    id: u64,
    new_item: Option<Item>,
    del: bool) -> bool;
pub type IsabelleRouteRegisterItemAuthHook = fn(
    name: &str,
    hook: IsabelleRouteItemAuthHook) -> bool;

pub type IsabelleRouteItemListFilterHook = fn(
    user: &Option<Item>,
    collection: &str,
    context: &str,
    map: &mut HashMap<u64, Item>);
pub type IsabelleRouteRegisterItemListFilterHook = fn(
    name: &str,
    hook: IsabelleRouteItemListFilterHook) -> bool;

pub type IsabelleRouteUrlHook = fn(
    user: &Option<Item>,
    query: &str) -> WebResponse;
pub type IsabelleRouteRegisterUrlHook = fn(
    name: &str,
    hook: IsabelleRouteUrlHook);

pub type IsabelleRouteUnprotectedUrlHook = fn(
    user: &Option<Item>,
    query: &str) -> WebResponse;
pub type IsabelleRouteRegisterUnprotectedUrlHook = fn(
    name: &str,
    hook: IsabelleRouteUnprotectedUrlHook);

pub type IsabelleRouteUnprotectedUrlPostHook = fn(
    user: &Option<Item>,
    query: &str,
    itm: &Item) -> WebResponse;
pub type IsabelleRouteRegisterUnprotectedUrlPostHook = fn(
    name: &str,
    hook: IsabelleRouteUnprotectedUrlPostHook);

pub type IsabelleRouteCollectionReadHook = fn(
    collection: &str,
    itm: &mut Item) -> bool;
pub type IsabelleRouteRegisterCollectionReadHook = fn(
    name: &str,
    hook: IsabelleRouteCollectionReadHook);

pub type IsabelleRouteCallOtpHook = fn(
    itm: &Item);
pub type IsabelleRouteRegisterCallOtpHook = fn(
    name: &str,
    hook: IsabelleRouteCallOtpHook);

pub struct PluginApi {
    /* database */
    pub db_get_all_items: IsabelleDbGetAllItemsFn,
    pub db_get_items: IsabelleDbGetItemsFn,
    pub db_get_item: IsabelleDbGetItemFn,
    pub db_set_item: IsabelleDbSetItemFn,
    pub db_del_item: IsabelleDbDelItemFn,

    /* globals */
    pub globals_get_public_url: IsabelleGlobalsGetPublicUrl,

    /* exposed functions */
    pub fn_send_email: IsabelleFnSendEmail,

    /* routes */
    pub route_register_item_pre_edit_hook: IsabelleRouteRegisterItemPreEditHook,
    pub route_register_item_post_edit_hook: IsabelleRouteRegisterItemPostEditHook,
    pub route_register_item_auth_hook: IsabelleRouteRegisterItemAuthHook,
    pub route_register_item_list_filter_hook: IsabelleRouteRegisterItemListFilterHook,
    pub route_register_url_hook: IsabelleRouteRegisterUrlHook,
    pub route_register_unprotected_url_hook: IsabelleRouteRegisterUnprotectedUrlHook,
    pub route_register_unprotected_url_post_hook: IsabelleRouteRegisterUnprotectedUrlPostHook,

    pub route_register_collection_read_hook: IsabelleRouteRegisterCollectionReadHook,
    pub route_register_call_otp_hook: IsabelleRouteRegisterCallOtpHook,
}

pub type IsabellePluginRegisterFn = fn(
    api: &PluginApi);

