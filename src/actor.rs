/*
 * Isabelle project
 *
 * Copyright 2026 Maxim Menshikov
 *
 * Permission is hereby granted, free of charge, to any person obtaining
 * a copy of this software and associated documentation files (the "Software"),
 * to deal in the Software without restriction, including without limitation
 * the rights to use, copy, modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software, and to permit persons to whom the
 * Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included
 * in all copies or substantial portions of the Software.
 */

//! Actor-model plugin contract.
//!
//! This module is the new plugin interface that replaces the synchronous
//! `Plugin` trait + `PluginApi` trait pair. Plugins are now actors:
//!
//! * Each plugin owns a tokio task and an mpsc receiver of
//!   [`PluginHookMessage`]. The task is a `while let Some(msg) = rx.recv()`
//!   loop that processes events and replies via embedded [`oneshot`]
//!   senders.
//! * To call back into core (database access, auth checks, secrets,
//!   email, …) plugins use a [`CoreHandle`] — a cloneable wrapper around
//!   `mpsc::Sender<CoreMessage>`. `CoreHandle::*` methods package the
//!   request, send it to core's processing task, and await a oneshot
//!   reply.
//!
//! Why messages instead of trait calls:
//!
//! * No shared mutable state between core and plugin → no global mutex.
//! * No FFI/dylib boundary → channel sends are nanoseconds, not the
//!   5-50µs thread-pool-bounce the old `PluginApi` paid for every call.
//! * Each plugin runs in its own task → plugins can be processed in
//!   parallel by tokio.
//! * Adding a new hook variant doesn't break existing plugins —
//!   `#[non_exhaustive]` lets them match `_ => {}`.
//!
//! ### Migration shape
//!
//! ```ignore
//! // In plugin crate:
//! pub fn register(reg: &mut PluginRegistry, core: CoreHandle) {
//!     let (tx, rx) = tokio::sync::mpsc::channel(64);
//!     tokio::spawn(run_my_plugin(rx, core));
//!     reg.add("my-plugin", tx);
//! }
//!
//! async fn run_my_plugin(
//!     mut rx: tokio::sync::mpsc::Receiver<PluginHookMessage>,
//!     core: CoreHandle,
//! ) {
//!     while let Some(msg) = rx.recv().await {
//!         match msg {
//!             PluginHookMessage::ItemPreEdit { reply, .. } => {
//!                 let _ = reply.send(PreEditReply::ok_unchanged());
//!             }
//!             PluginHookMessage::Shutdown => break,
//!             _ => {} // unknown variant — ignore
//!         }
//!     }
//! }
//! ```

use isabelle_dm::data_model::data_object_action::DataObjectAction;
use isabelle_dm::data_model::item::Item;
use isabelle_dm::data_model::list_result::ListResult;
use isabelle_dm::data_model::process_result::ProcessResult;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

use crate::api::WebResponse;

// ---------------------------------------------------------------------------
// Replies
// ---------------------------------------------------------------------------

/// Reply to [`PluginHookMessage::ItemPreEdit`]: outcome + optional in-place
/// edit. If the plugin mutated the item, it sends the modified version back
/// via `modified_item`; the dispatcher merges it into the canonical item.
#[derive(Debug, Clone)]
pub struct PreEditReply {
    pub result: ProcessResult,
    pub modified_item: Option<Item>,
}

impl PreEditReply {
    /// Plugin doesn't care about this hook — pass-through.
    pub fn ok_unchanged() -> Self {
        Self {
            result: ProcessResult {
                succeeded: true,
                error: String::new(),
                data: HashMap::new(),
            },
            modified_item: None,
        }
    }
    /// Plugin rejects the edit — short-circuits the dispatcher.
    pub fn rejected(error: impl Into<String>) -> Self {
        Self {
            result: ProcessResult {
                succeeded: false,
                error: error.into(),
                data: HashMap::new(),
            },
            modified_item: None,
        }
    }
}

/// Reply to [`PluginHookMessage::ItemListFilter`]: the (possibly mutated)
/// items page. Sent back by value; the dispatcher swaps it into the
/// caller-side `ListResult.map`.
#[derive(Debug, Clone, Default)]
pub struct ListFilterReply {
    pub items: HashMap<u64, Item>,
}

// ---------------------------------------------------------------------------
// core -> plugin messages
// ---------------------------------------------------------------------------

/// Messages sent from core to plugin actors. The plugin reads them from its
/// mpsc receiver and processes them in its `run` loop.
///
/// `#[non_exhaustive]` so future variants don't break existing plugins —
/// plugins should always have a `_ => {}` arm.
#[non_exhaustive]
pub enum PluginHookMessage {
    /// Pre-flight before an item is written. Plugin can reject, or mutate
    /// the item before it's persisted.
    ItemPreEdit {
        hndl: String,
        user: Option<Item>,
        collection: String,
        old_item: Option<Item>,
        item: Item,
        action: DataObjectAction,
        merge: bool,
        reply: oneshot::Sender<PreEditReply>,
    },

    /// Notification after an item was written. Fire-and-forget (no reply).
    ItemPostEdit {
        hndl: String,
        collection: String,
        old_item: Option<Item>,
        id: u64,
        action: DataObjectAction,
    },

    /// Authorization check. Plugin returns true to allow, false to deny.
    /// Any plugin denying short-circuits the dispatcher.
    ItemAuth {
        hndl: String,
        user: Option<Item>,
        collection: String,
        id: u64,
        new_item: Option<Item>,
        del: bool,
        reply: oneshot::Sender<bool>,
    },

    /// In-page filter/mutation of a list result. Plugin receives the page,
    /// returns a (possibly mutated/filtered) page.
    ItemListFilter {
        hndl: String,
        user: Option<Item>,
        collection: String,
        context: String,
        items: HashMap<u64, Item>,
        reply: oneshot::Sender<ListFilterReply>,
    },

    /// Build a Mongo-side filter clause that's AND-ed into the query.
    /// Returns either an empty string (no filter) or a JSON object.
    ItemListDbFilter {
        hndl: String,
        user: Option<Item>,
        collection: String,
        context: String,
        filter_type: String,
        reply: oneshot::Sender<String>,
    },

    /// Hook applied during item read (init_checks etc.). The plugin returns
    /// the (possibly mutated) item; the dispatcher decides whether to
    /// persist it.
    CollectionRead {
        hndl: String,
        collection: String,
        item: Item,
        reply: oneshot::Sender<CollectionReadReply>,
    },

    /// One-time password event.
    Otp {
        hndl: String,
        item: Item,
    },

    /// Periodic job — fired by core's scheduler. `timing` is "sec" or "min".
    PeriodicJob {
        timing: String,
    },

    /// Authenticated GET-style route.
    RouteUrl {
        hndl: String,
        user: Option<Item>,
        query: String,
        reply: oneshot::Sender<WebResponse>,
    },

    /// Authenticated POST-style route with a parsed multipart item.
    RouteUrlPost {
        hndl: String,
        user: Option<Item>,
        query: String,
        item: Item,
        reply: oneshot::Sender<WebResponse>,
    },

    /// Public (unauthenticated) GET-style route.
    RouteUnprotectedUrl {
        hndl: String,
        user: Option<Item>,
        query: String,
        reply: oneshot::Sender<WebResponse>,
    },

    /// Public POST route.
    RouteUnprotectedUrlPost {
        hndl: String,
        user: Option<Item>,
        query: String,
        item: Item,
        reply: oneshot::Sender<WebResponse>,
    },

    /// REST-style route with method + raw body.
    RouteRest {
        hndl: String,
        method: String,
        user: Option<Item>,
        query: String,
        payload: String,
        reply: oneshot::Sender<WebResponse>,
    },

    /// Liveness probe at startup. Replaces the old `ping_test`.
    Ping {
        reply: oneshot::Sender<()>,
    },

    /// Graceful shutdown. Plugin task should exit its loop.
    Shutdown,
}

/// Reply to [`PluginHookMessage::CollectionRead`].
#[derive(Debug, Clone, Default)]
pub struct CollectionReadReply {
    /// Whether the item should be persisted by the dispatcher.
    pub should_save: bool,
    /// Possibly-modified item.
    pub item: Option<Item>,
}

// ---------------------------------------------------------------------------
// plugin -> core messages
// ---------------------------------------------------------------------------

/// Requests sent from a plugin back into core. Core's main processing task
/// owns the database/auth/secrets state and answers via oneshot replies.
#[non_exhaustive]
pub enum CoreMessage {
    // --- Database ---
    DbGetAllItems {
        collection: String,
        sort_key: String,
        filter: String,
        reply: oneshot::Sender<ListResult>,
    },
    DbGetItems {
        collection: String,
        id_min: u64,
        id_max: u64,
        sort_key: String,
        filter: String,
        skip: u64,
        limit: u64,
        reply: oneshot::Sender<ListResult>,
    },
    DbGetItem {
        collection: String,
        id: u64,
        reply: oneshot::Sender<Option<Item>>,
    },
    DbSetItem {
        collection: String,
        item: Item,
        merge: bool,
        reply: oneshot::Sender<u64>,
    },
    DbDelItem {
        collection: String,
        id: u64,
        reply: oneshot::Sender<bool>,
    },

    // --- Globals ---
    GlobalsGetPublicUrl {
        reply: oneshot::Sender<String>,
    },
    /// Data path (where the deployment's data directory lives on disk).
    /// Plugins use this to read/write side-channel files like avatars.
    GlobalsGetDataPath {
        reply: oneshot::Sender<String>,
    },
    GlobalsGetSettings {
        reply: oneshot::Sender<Item>,
    },
    GlobalsSetSettings {
        item: Item,
    },

    // --- Auth ---
    AuthCheckRole {
        item: Option<Item>,
        role: String,
        reply: oneshot::Sender<bool>,
    },
    AuthGetNewSalt {
        reply: oneshot::Sender<String>,
    },
    AuthGetPasswordHash {
        password: String,
        salt: String,
        reply: oneshot::Sender<String>,
    },
    AuthVerifyPassword {
        password: String,
        hash: String,
        reply: oneshot::Sender<bool>,
    },
    AuthLogin {
        login: String,
        password: String,
        reply: oneshot::Sender<ProcessResult>,
    },
    AuthLogout {
        login: String,
        reply: oneshot::Sender<ProcessResult>,
    },
    AuthRegister {
        login: String,
        email: String,
        reply: oneshot::Sender<ProcessResult>,
    },
    AuthGenOtp {
        login: String,
        reply: oneshot::Sender<ProcessResult>,
    },

    // --- Notifications ---
    SendEmail {
        to: String,
        subject: String,
        body: String,
    },
    InitGoogle {
        reply: oneshot::Sender<String>,
    },
    SyncWithGoogle {
        add: bool,
        name: String,
        date_time: String,
    },

    // --- Secrets ---
    SecretGet {
        id: u64,
        reply: oneshot::Sender<Option<Item>>,
    },
}

// ---------------------------------------------------------------------------
// CoreHandle: ergonomic API plugins use to talk to core
// ---------------------------------------------------------------------------

/// Cloneable handle plugins use to call back into core. Wraps an mpsc
/// sender and exposes async methods that match the operations the old
/// `PluginApi` trait offered. Each method packages a [`CoreMessage`],
/// sends it, and awaits the oneshot reply.
#[derive(Clone)]
pub struct CoreHandle {
    tx: mpsc::Sender<CoreMessage>,
}

impl CoreHandle {
    pub fn new(tx: mpsc::Sender<CoreMessage>) -> Self {
        Self { tx }
    }

    /// Errors are intentionally swallowed: if core is shutting down, the
    /// channel is closed, and plugins should treat that as a no-op rather
    /// than blowing up mid-request. Same sentinel choice as the old
    /// `PluginApi` impl (which returned defaults on failure).
    async fn request<T>(
        &self,
        build: impl FnOnce(oneshot::Sender<T>) -> CoreMessage,
    ) -> Option<T> {
        let (rtx, rrx) = oneshot::channel();
        self.tx.send(build(rtx)).await.ok()?;
        rrx.await.ok()
    }

    // --- Database ---
    pub async fn db_get_all_items(
        &self,
        collection: &str,
        sort_key: &str,
        filter: &str,
    ) -> ListResult {
        self.request(|reply| CoreMessage::DbGetAllItems {
            collection: collection.into(),
            sort_key: sort_key.into(),
            filter: filter.into(),
            reply,
        })
        .await
        .unwrap_or(ListResult {
            map: HashMap::new(),
            total_count: 0,
        })
    }

    pub async fn db_get_items(
        &self,
        collection: &str,
        id_min: u64,
        id_max: u64,
        sort_key: &str,
        filter: &str,
        skip: u64,
        limit: u64,
    ) -> ListResult {
        self.request(|reply| CoreMessage::DbGetItems {
            collection: collection.into(),
            id_min,
            id_max,
            sort_key: sort_key.into(),
            filter: filter.into(),
            skip,
            limit,
            reply,
        })
        .await
        .unwrap_or(ListResult {
            map: HashMap::new(),
            total_count: 0,
        })
    }

    pub async fn db_get_item(&self, collection: &str, id: u64) -> Option<Item> {
        self.request(|reply| CoreMessage::DbGetItem {
            collection: collection.into(),
            id,
            reply,
        })
        .await
        .flatten()
    }

    pub async fn db_set_item(&self, collection: &str, item: &Item, merge: bool) -> u64 {
        self.request(|reply| CoreMessage::DbSetItem {
            collection: collection.into(),
            item: item.clone(),
            merge,
            reply,
        })
        .await
        .unwrap_or(u64::MAX)
    }

    pub async fn db_del_item(&self, collection: &str, id: u64) -> bool {
        self.request(|reply| CoreMessage::DbDelItem {
            collection: collection.into(),
            id,
            reply,
        })
        .await
        .unwrap_or(false)
    }

    // --- Globals ---
    pub async fn globals_get_public_url(&self) -> String {
        self.request(|reply| CoreMessage::GlobalsGetPublicUrl { reply })
            .await
            .unwrap_or_default()
    }

    pub async fn globals_get_data_path(&self) -> String {
        self.request(|reply| CoreMessage::GlobalsGetDataPath { reply })
            .await
            .unwrap_or_default()
    }

    pub async fn globals_get_settings(&self) -> Item {
        self.request(|reply| CoreMessage::GlobalsGetSettings { reply })
            .await
            .unwrap_or_else(Item::new)
    }

    pub async fn globals_set_settings(&self, item: &Item) {
        let _ = self
            .tx
            .send(CoreMessage::GlobalsSetSettings { item: item.clone() })
            .await;
    }

    // --- Auth ---
    pub async fn auth_check_role(&self, item: &Option<Item>, role: &str) -> bool {
        self.request(|reply| CoreMessage::AuthCheckRole {
            item: item.clone(),
            role: role.into(),
            reply,
        })
        .await
        .unwrap_or(false)
    }

    pub async fn auth_get_new_salt(&self) -> String {
        self.request(|reply| CoreMessage::AuthGetNewSalt { reply })
            .await
            .unwrap_or_default()
    }

    pub async fn auth_get_password_hash(&self, pw: &str, salt: &str) -> String {
        self.request(|reply| CoreMessage::AuthGetPasswordHash {
            password: pw.into(),
            salt: salt.into(),
            reply,
        })
        .await
        .unwrap_or_default()
    }

    pub async fn auth_verify_password(&self, pw: &str, hash: &str) -> bool {
        self.request(|reply| CoreMessage::AuthVerifyPassword {
            password: pw.into(),
            hash: hash.into(),
            reply,
        })
        .await
        .unwrap_or(false)
    }

    pub async fn auth_login(&self, login: &str, password: &str) -> ProcessResult {
        self.request(|reply| CoreMessage::AuthLogin {
            login: login.into(),
            password: password.into(),
            reply,
        })
        .await
        .unwrap_or_else(|| ProcessResult {
            succeeded: false,
            error: "core unavailable".into(),
            data: HashMap::new(),
        })
    }

    pub async fn auth_logout(&self, login: &str) -> ProcessResult {
        self.request(|reply| CoreMessage::AuthLogout {
            login: login.into(),
            reply,
        })
        .await
        .unwrap_or_else(|| ProcessResult {
            succeeded: false,
            error: "core unavailable".into(),
            data: HashMap::new(),
        })
    }

    pub async fn auth_register(&self, login: &str, email: &str) -> ProcessResult {
        self.request(|reply| CoreMessage::AuthRegister {
            login: login.into(),
            email: email.into(),
            reply,
        })
        .await
        .unwrap_or_else(|| ProcessResult {
            succeeded: false,
            error: "core unavailable".into(),
            data: HashMap::new(),
        })
    }

    pub async fn auth_gen_otp(&self, login: &str) -> ProcessResult {
        self.request(|reply| CoreMessage::AuthGenOtp {
            login: login.into(),
            reply,
        })
        .await
        .unwrap_or_else(|| ProcessResult {
            succeeded: false,
            error: "core unavailable".into(),
            data: HashMap::new(),
        })
    }

    // --- Notifications ---
    pub async fn send_email(&self, to: &str, subject: &str, body: &str) {
        let _ = self
            .tx
            .send(CoreMessage::SendEmail {
                to: to.into(),
                subject: subject.into(),
                body: body.into(),
            })
            .await;
    }

    pub async fn init_google(&self) -> String {
        self.request(|reply| CoreMessage::InitGoogle { reply })
            .await
            .unwrap_or_default()
    }

    pub async fn sync_with_google(&self, add: bool, name: String, date_time: String) {
        let _ = self
            .tx
            .send(CoreMessage::SyncWithGoogle {
                add,
                name,
                date_time,
            })
            .await;
    }

    // --- Secrets ---
    pub async fn secret_get(&self, id: u64) -> Option<Item> {
        self.request(|reply| CoreMessage::SecretGet { id, reply })
            .await
            .flatten()
    }
}

// ---------------------------------------------------------------------------
// PluginRegistry: bookkeeping for the dispatcher
// ---------------------------------------------------------------------------

/// Holds the registered plugin actors. Core's hook dispatcher iterates this
/// to fan out [`PluginHookMessage`]s. Each plugin is identified by a name
/// (informational; used in logs) and a `Sender<PluginHookMessage>`.
///
/// Pre-built routing tables (collection→plugin, hndl→plugin) belong in core
/// alongside the existing `RouteCache` and are derived from `internals.js`
/// the same way as before — this registry just owns the senders.
pub struct PluginRegistry {
    plugins: Vec<RegisteredPlugin>,
}

struct RegisteredPlugin {
    #[allow(dead_code)]
    name: String,
    sender: mpsc::Sender<PluginHookMessage>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn add(&mut self, name: impl Into<String>, sender: mpsc::Sender<PluginHookMessage>) {
        self.plugins.push(RegisteredPlugin {
            name: name.into(),
            sender,
        });
    }

    /// Number of registered plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Iterator over senders. Dispatcher uses this to fan out hook events.
    pub fn senders(&self) -> impl Iterator<Item = &mpsc::Sender<PluginHookMessage>> {
        self.plugins.iter().map(|p| &p.sender)
    }

    /// Broadcast `Shutdown` to all plugin tasks. Best-effort; doesn't wait
    /// for them to terminate (use the join handles from `tokio::spawn` for
    /// that on the caller side).
    pub async fn shutdown_all(&self) {
        for p in &self.plugins {
            let _ = p.sender.send(PluginHookMessage::Shutdown).await;
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_edit_reply_helpers_produce_correct_shape() {
        let ok = PreEditReply::ok_unchanged();
        assert!(ok.result.succeeded);
        assert!(ok.modified_item.is_none());

        let bad = PreEditReply::rejected("nope");
        assert!(!bad.result.succeeded);
        assert_eq!(bad.result.error, "nope");
    }

    #[test]
    fn plugin_registry_tracks_senders() {
        let mut reg = PluginRegistry::new();
        assert!(reg.is_empty());

        let (tx1, _rx1) = mpsc::channel(1);
        reg.add("a", tx1);
        let (tx2, _rx2) = mpsc::channel(1);
        reg.add("b", tx2);

        assert_eq!(reg.len(), 2);
        assert_eq!(reg.senders().count(), 2);
    }
}
