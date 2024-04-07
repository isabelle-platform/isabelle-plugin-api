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
type IsabelleDbGetAllItemsFn = fn(
	collection: &str,
	sort_key: &str,
	filter: &str) -> Item;
type IsabelleDbGetItemsFn = fn(
	collection: &str,
	id_min: u64,
    id_max: u64,
    sort_key: &str,
    filter: &str,
    skip: u64,
    limit: u64) -> ListResult;
type IsabelleDbGetItemFn = fn(
	collection: &str,
	id: u64) -> Item;
type IsabelleDbDelItemFn = fn(
	collection: &str,
	id: u64) -> bool;
type IsabelleDbSetItemFn = fn(
	collection: &str,
	itm: &Item,
	merge: &bool);

/* globals */
type IsabelleGlobalsGetPublicUrl = fn() -> String;

/* exposed functions */
type IsabelleFnSendEmail = fn(
	to: &str,
	subject: &str,
	body: &str);

/* route API */
type IsabelleRouteItemPreEditHook = fn(
    user: &Option<Item>,
    collection: &str,
    old_itm: Option<Item>,
    itm: &mut Item,
    del: bool,
    merge: bool) -> ProcessResult;
type IsabelleRouteRegisterItemPreEditHook = fn(
	name: &str,
	hook: IsabelleRouteItemPreEditHook) -> bool;

type IsabelleRouteItemPostEditHook = fn(
    collection: &str,
    id: u64,
    del: bool);
type IsabelleRouteRegisterItemPostEditHook = fn(
	name: &str,
	hook: IsabelleRouteItemPostEditHook) -> bool;

type IsabelleRouteItemAuthHook = fn(
	user: &Option<Item>,
    collection: &str,
    id: u64,
    new_item: Option<Item>,
    del: bool) -> bool;
type IsabelleRouteRegisterItemAuthHook = fn(
	name: &str,
	hook: IsabelleRouteItemAuthHook) -> bool;

type IsabelleRouteItemListFilterHook = fn(
	user: &Option<Item>,
    collection: &str,
    context: &str,
    map: &mut HashMap<u64, Item>);
type IsabelleRouteRegisterItemListFilterHook = fn(
	name: &str,
	hook: IsabelleRouteItemListFilterHook) -> bool;

type IsabelleRouteUrlHook = fn(
	user: &Option<Item>,
	query: &str) -> WebResponse;
type IsabelleRouteRegisterUrlHook = fn(
	name: &str,
	hook: IsabelleRouteUrlHook);

type IsabelleRouteUnprotectedUrlHook = fn(
	user: &Option<Item>,
	query: &str) -> WebResponse;
type IsabelleRouteRegisterUnprotectedUrlHook = fn(
	name: &str,
	hook: IsabelleRouteUnprotectedUrlHook);

type IsabelleRouteUnprotectedUrlPostHook = fn(
	user: &Option<Item>,
	query: &str,
	itm: &Item) -> WebResponse;
type IsabelleRouteRegisterUnprotectedUrlPostHook = fn(
	name: &str,
	hook: IsabelleRouteUnprotectedUrlPostHook);

type IsabelleRouteCollectionReadHook = fn(
	collection: &str,
	itm: &mut Item) -> bool;
type IsabelleRouteRegisterCollectionReadHook = fn(
	name: &str,
	hook: IsabelleRouteCollectionReadHook);

type IsabelleRouteCallOtpHook = fn(
	itm: &Item);
type IsabelleRouteRegisterCallOtpHook = fn(
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

