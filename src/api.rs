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
use std::collections::HashMap;
use isabelle_dm::data_model::item::Item;
use isabelle_dm::data_model::list_result::ListResult;
use isabelle_dm::data_model::process_result::ProcessResult;

#[repr(C)]
/// Canonical web responses
pub enum WebResponse {
    Ok,
    OkData(String),
    NotFound,
    Unauthorized,
    BadRequest,
    Forbidden,
}

pub trait Plugin {
    fn item_pre_edit_hook(&mut self,
        api: Box<&dyn PluginApi>,
        user: &Option<Item>,
        collection: &str,
        old_itm: Option<Item>,
        itm: &mut Item,
        del: bool,
        merge: bool) -> ProcessResult;
    fn item_post_edit_hook(&mut self,
        api: Box<&dyn PluginApi>,
        collection: &str,
        id: u64,
        del: bool);
    fn item_auth_hook(&mut self,
        api: Box<&dyn PluginApi>,
        user: &Option<Item>,
        collection: &str,
        id: u64,
        new_item: Option<Item>,
        del: bool) -> bool;
    fn item_list_filter_hook(&mut self,
        api: Box<&dyn PluginApi>,
        user: &Option<Item>,
        collection: &str,
        context: &str,
        map: &mut HashMap<u64, Item>);

    fn route_url_hook(&mut self,
        api: Box<&dyn PluginApi>,
        user: &Option<Item>,
        query: &str) -> WebResponse;
    fn route_unprotected_url_hook(&mut self,
        api: Box<&dyn PluginApi>,
        user: &Option<Item>,
        query: &str) -> WebResponse;
    fn route_unprotected_url_post_hook(&mut self,
        api: Box<&dyn PluginApi>,
        user: &Option<Item>,
        query: &str,
        itm: &Item) -> WebResponse;
    fn route_collection_read_hook(&mut self,
        api: Box<&dyn PluginApi>,
        collection: &str,
        itm: &mut Item) -> bool;
    fn route_call_otp_hook(&mut self,
        api: Box<&dyn PluginApi>,
        itm: &Item);
}

pub trait PluginApi {
    fn db_get_all_items(&self, collection: &str, sort_key: &str, filter: &str)
        -> ListResult;
    fn db_get_items(&self, collection: &str,
        id_min: u64,
        id_max: u64,
        sort_key: &str,
        filter: &str,
        skip: u64,
        limit: u64) -> ListResult;
    fn db_get_item(&self, collection: &str, id: u64) -> Option<Item>;
    fn db_set_item(&self, collection: &str, itm: &Item, merge: bool);
    fn db_del_item(&self, collection: &str, id: u64) -> bool;

    fn globals_get_public_url(&self) -> String;
    fn global_get_settings(&self) -> Item;

    fn auth_check_role(&self, itm: &Option<Item>, role: &str) -> bool;
    fn auth_get_new_salt(&self) -> String;
    fn auth_get_password_hash(&self, pw: &str, salt: &str) -> String;
    fn auth_verify_password(&self, pw: &str, pw_hash: &str) -> bool;

    fn fn_send_email(&self, to: &str, subject: &str, body: &str);
    fn fn_init_google(&self) -> String;
    fn fn_sync_with_google(&self, add: bool, name: String, date_time: String);
}

pub trait PluginPoolApi {
    fn register(&mut self, plugin: Box<dyn Plugin>);
}
