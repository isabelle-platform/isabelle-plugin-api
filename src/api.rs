use isabelle_dm::data_model::item::Item;
use isabelle_dm::data_model::list_result::ListResult;

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
}

