#![allow(dead_code)]

use isabelle_dm::data_model::data_object_action::DataObjectAction;
use isabelle_dm::data_model::item::Item;
use isabelle_dm::data_model::list_result::ListResult;
use isabelle_dm::data_model::process_result::ProcessResult;
use isabelle_plugin_api::api::*;
use std::any::Any;
use std::collections::HashMap;

/// Test plugin that records how often `ping_test` was called and exercises
/// only the default trait impls for the rest. Using the defaults lets us
/// assert their return values without re-implementing every hook.
pub struct CountingPlugin {
    pub pings: u64,
}

impl CountingPlugin {
    pub fn new() -> Self {
        Self { pings: 0 }
    }
}

impl Plugin for CountingPlugin {
    fn ping_test(&mut self) {
        self.pings += 1;
    }

    fn item_pre_edit_hook(
        &mut self,
        _api: &Box<dyn PluginApi>,
        _hndl: &str,
        _user: &Option<Item>,
        _collection: &str,
        _old_itm: Option<Item>,
        _itm: &mut Item,
        _action: DataObjectAction,
        _merge: bool,
    ) -> ProcessResult {
        ProcessResult {
            succeeded: true,
            data: HashMap::new(),
            error: "".to_string(),
        }
    }

    fn item_post_edit_hook(
        &mut self,
        _api: &Box<dyn PluginApi>,
        _hndl: &str,
        _collection: &str,
        _old_itm: Option<Item>,
        _id: u64,
        _action: DataObjectAction,
    ) {
    }

    fn item_auth_hook(
        &mut self,
        _api: &Box<dyn PluginApi>,
        _hndl: &str,
        _user: &Option<Item>,
        _collection: &str,
        _id: u64,
        _new_item: Option<Item>,
        _del: bool,
    ) -> bool {
        true
    }

    fn item_list_filter_hook(
        &mut self,
        _api: &Box<dyn PluginApi>,
        _hndl: &str,
        _user: &Option<Item>,
        _collection: &str,
        _context: &str,
        _map: &mut HashMap<u64, Item>,
    ) {
    }

    fn route_url_hook(
        &mut self,
        _api: &Box<dyn PluginApi>,
        _hndl: &str,
        _user: &Option<Item>,
        _query: &str,
    ) -> WebResponse {
        WebResponse::Ok
    }

    fn route_unprotected_url_hook(
        &mut self,
        _api: &Box<dyn PluginApi>,
        _hndl: &str,
        _user: &Option<Item>,
        _query: &str,
    ) -> WebResponse {
        WebResponse::Ok
    }

    fn route_unprotected_url_post_hook(
        &mut self,
        _api: &Box<dyn PluginApi>,
        _hndl: &str,
        _user: &Option<Item>,
        _query: &str,
        _itm: &Item,
    ) -> WebResponse {
        WebResponse::Ok
    }

    fn collection_read_hook(
        &mut self,
        _api: &Box<dyn PluginApi>,
        _hndl: &str,
        _collection: &str,
        _itm: &mut Item,
    ) -> bool {
        true
    }

    fn call_otp_hook(&mut self, _api: &Box<dyn PluginApi>, _hndl: &str, _itm: &Item) {}
}

/// Minimal `PluginApi` implementor that relies on the default impls for
/// everything optional (secrets, periodic jobs, etc.). The required-method
/// impls below are intentionally trivial — they exist to satisfy the trait,
/// not to be exercised by the default-impl tests.
pub struct StubApi;

impl PluginApi for StubApi {
    fn db_get_all_items(&self, _collection: &str, _sort_key: &str, _filter: &str) -> ListResult {
        ListResult {
            map: HashMap::new(),
            total_count: 0,
        }
    }
    fn db_get_items(
        &self,
        _collection: &str,
        _id_min: u64,
        _id_max: u64,
        _sort_key: &str,
        _filter: &str,
        _skip: u64,
        _limit: u64,
    ) -> ListResult {
        ListResult {
            map: HashMap::new(),
            total_count: 0,
        }
    }
    fn db_get_item(&self, _collection: &str, _id: u64) -> Option<Item> {
        None
    }
    fn db_set_item(&self, _collection: &str, _itm: &Item, _merge: bool) -> u64 {
        0
    }
    fn db_del_item(&self, _collection: &str, _id: u64) -> bool {
        false
    }

    fn globals_get_public_url(&self) -> String {
        String::new()
    }
    fn globals_get_settings(&self) -> Item {
        Item::new()
    }
    fn globals_set_settings(&self, _itm: &Item) {}

    fn auth_check_role(&self, _itm: &Option<Item>, _role: &str) -> bool {
        false
    }
    fn auth_get_new_salt(&self) -> String {
        String::new()
    }
    fn auth_get_password_hash(&self, _pw: &str, _salt: &str) -> String {
        String::new()
    }
    fn auth_verify_password(&self, _pw: &str, _pw_hash: &str) -> bool {
        false
    }

    fn auth_login(&self, _login: &str, _password: &str) -> ProcessResult {
        ProcessResult {
            succeeded: false,
            data: HashMap::new(),
            error: "".to_string(),
        }
    }
    fn auth_logout(&self, _login: &str) -> ProcessResult {
        ProcessResult {
            succeeded: false,
            data: HashMap::new(),
            error: "".to_string(),
        }
    }
    fn auth_register(&self, _login: &str, _email: &str) -> ProcessResult {
        ProcessResult {
            succeeded: false,
            data: HashMap::new(),
            error: "".to_string(),
        }
    }
    fn auth_gen_otp(&self, _login: &str) -> ProcessResult {
        ProcessResult {
            succeeded: false,
            data: HashMap::new(),
            error: "".to_string(),
        }
    }

    fn fn_send_email(&self, _to: &str, _subject: &str, _body: &str) {}
    fn fn_init_google(&self) -> String {
        String::new()
    }
    fn fn_sync_with_google(&self, _add: bool, _name: String, _date_time: String) {}

    fn fn_get_state(&self, _hndl: &str) -> &mut Option<Box<dyn Any + Send>> {
        // Leak a 'static slot so we can hand out a mutable reference with
        // the lifetime the trait requires; fine for tests.
        Box::leak(Box::new(None))
    }
    fn fn_set_state(&self, _hndl: &str, _value: Option<Box<dyn Any + Send>>) {}
}
