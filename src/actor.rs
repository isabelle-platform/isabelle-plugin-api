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
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
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
    Otp { hndl: String, item: Item },

    /// Periodic job — fired by core's scheduler. `timing` is "sec" or "min".
    PeriodicJob { timing: String },

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
        /// Request headers: names lowercased, credential-bearing ones removed
        /// by the host. Duplicated names collapse to the last value seen.
        ///
        /// This is what makes a signed request verifiable. A caller that signs
        /// its body — a payment provider's webhook is the usual one — puts the
        /// signature in a header, and a hook that can see the body but not the
        /// header can only choose between trusting an unauthenticated POST and
        /// refusing every delivery.
        headers: HashMap<String, String>,
        reply: oneshot::Sender<WebResponse>,
    },

    /// Liveness probe at startup. Replaces the old `ping_test`.
    Ping { reply: oneshot::Sender<()> },

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
// Disk budget contract
// ---------------------------------------------------------------------------
//
// How the data filesystem is shared between the host's reserve and the
// platform's consumers (MongoDB, uploads, per-job scratch). The policy is
// settable and the computed budget and live metrics are readable, all through
// the CoreHandle methods below. Core owns the statvfs, the minute monitor and
// the enforcement (see its `state::diskbudget`); these are the shared shapes
// and the one pure computation both sides agree on.

/// The settable disk-budget policy: the deployment's choice of how the data
/// filesystem is shared. Fractions are of the *usable* space (total minus the
/// reserve) and are independent ceilings, not a partition, so they need not sum
/// to one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiskPolicy {
    /// Keep at least this many bytes free for the host, whatever the fraction
    /// works out to — a floor for small disks.
    pub root_reserve_floor: u64,
    /// …or this fraction of the total, whichever is larger — scales with big
    /// disks.
    pub root_reserve_frac: f64,
    /// Ceiling for MongoDB, as a fraction of usable space.
    pub mongo_frac: f64,
    /// Ceiling for uploads, as a fraction of usable space.
    pub upload_frac: f64,
    /// Ceiling for all concurrent job scratch, as a fraction of usable space.
    pub job_frac: f64,
    /// Worst-case concurrent jobs the per-job cap must divide the job share by.
    pub max_concurrent_jobs: u64,
}

impl Default for DiskPolicy {
    /// The shipped defaults: keep 5 GiB or 10% free; of the rest, half to the
    /// database, a third to uploads, a fifth to jobs; four jobs at once.
    fn default() -> Self {
        Self {
            root_reserve_floor: 5 * 1024 * 1024 * 1024,
            root_reserve_frac: 0.10,
            mongo_frac: 0.50,
            upload_frac: 0.30,
            job_frac: 0.20,
            max_concurrent_jobs: 4,
        }
    }
}

/// The caps a [`DiskPolicy`] works out to for a filesystem of a given size.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiskBudget {
    /// Free space kept for the host; the disk is "low" at or below this.
    pub root_reserve: u64,
    /// Ceiling for MongoDB data.
    pub mongo_cap: u64,
    /// Ceiling for the uploads directory.
    pub upload_cap: u64,
    /// Ceiling for all concurrent job scratch together.
    pub job_scratch_total: u64,
    /// Ceiling for one job's scratch.
    pub per_job_disk: u64,
}

/// Live measurement of the data filesystem, plus the current verdict the
/// monitor maintains.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiskMetrics {
    /// Total size of the filesystem.
    pub total_bytes: u64,
    /// Bytes available to the server right now.
    pub avail_bytes: u64,
    /// Whether available space is at or below the reserve (work is being shed).
    pub disk_low: bool,
}

/// Everything a plugin needs to know about disk: what is measured now, the caps
/// that follow, and the policy that produced them.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiskReport {
    pub metrics: DiskMetrics,
    pub budget: DiskBudget,
    pub policy: DiskPolicy,
}

/// Compute the [`DiskBudget`] for a filesystem of `total_bytes` under `policy`.
///
/// Pure and total: the reserve is the larger of the floor and the fraction,
/// clamped to the disk; the usable remainder is split by the fractions; the
/// per-job cap divides the job share by the concurrency (never by zero). This
/// is the one computation core and plugins agree on.
pub fn plan_budget(total_bytes: u64, policy: &DiskPolicy) -> DiskBudget {
    let frac_reserve = (total_bytes as f64 * policy.root_reserve_frac) as u64;
    let root_reserve = policy.root_reserve_floor.max(frac_reserve).min(total_bytes);
    let usable = total_bytes.saturating_sub(root_reserve);
    let share = |f: f64| (usable as f64 * f) as u64;
    let job_scratch_total = share(policy.job_frac);
    let jobs = policy.max_concurrent_jobs.max(1);
    DiskBudget {
        root_reserve,
        mongo_cap: share(policy.mongo_frac),
        upload_cap: share(policy.upload_frac),
        job_scratch_total,
        per_job_disk: job_scratch_total / jobs,
    }
}

// ---------------------------------------------------------------------------
// Maintenance
// ---------------------------------------------------------------------------

/// Whether the installation takes new operations, as the core keeps it.
///
/// An update of an installation stops its service. So an update is asked
/// for, and the core waits for a moment when no plugin has an operation
/// open, closes in that same step, and reopens when told or after a set
/// time. An *operation* is whatever a plugin says must not be cut off by a
/// restart and must not be started just before one: a machine being built, a
/// test run being collected. A plugin brackets each with
/// [`CoreHandle::operation_begin`] and [`CoreHandle::operation_end`]; the
/// core counts them, and that count is the only thing it waits for.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MaintenanceInfo {
    /// `open`, `waiting` (asked for, waiting for a quiet moment) or `closed`.
    pub state: String,
    /// Who asked.
    pub asked_by: String,
    /// What for, in the asker's words; shown to an administrator.
    pub reason: String,
    /// While waiting: when the wait is given up, RFC 3339.
    pub wait_until: String,
    /// While closed: when it reopens by itself, RFC 3339.
    pub reopen_at: String,
    /// How the last request ended, when it ended by itself.
    pub note: String,
    /// What is open now, in the words each was opened with.
    pub running: Vec<String>,
}

impl MaintenanceInfo {
    /// Whether new operations are refused.
    pub fn is_closed(&self) -> bool {
        self.state == "closed"
    }

    pub fn is_waiting(&self) -> bool {
        self.state == "waiting"
    }
}

/// What a caller is told when an operation is refused because the
/// installation is closed. One sentence, the same everywhere.
pub const MAINTENANCE_REFUSED: &str =
    "This installation is being updated. Try again in a few minutes.";

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
    /// The disk situation: live metrics, the computed budget, and the active
    /// policy (see [`DiskReport`]).
    GlobalsGetDiskReport {
        reply: oneshot::Sender<DiskReport>,
    },
    /// Open an operation, unless closed for maintenance. `resumed` is one
    /// that was already running before a restart: it is counted and never
    /// refused. The reply is the token to end it with, or `None` when
    /// refused.
    OperationBegin {
        title: String,
        resumed: bool,
        reply: oneshot::Sender<Option<u64>>,
    },
    OperationEnd {
        token: u64,
    },
    MaintenanceGet {
        reply: oneshot::Sender<MaintenanceInfo>,
    },
    /// Close at the next moment no operation is open.
    MaintenanceAsk {
        by: String,
        reason: String,
        reply: oneshot::Sender<MaintenanceInfo>,
    },
    MaintenanceReopen {
        by: String,
        reply: oneshot::Sender<MaintenanceInfo>,
    },
    /// Replace the active disk-budget policy. The monitor uses the new one from
    /// its next tick, and the change is persisted so it survives a restart.
    /// Reply is whether it was accepted and stored.
    GlobalsSetDiskPolicy {
        policy: DiskPolicy,
        reply: oneshot::Sender<bool>,
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
    /// Every stored secret by id and name, sorted by name. Names only: the
    /// material is read with `SecretGet` by whoever needs it.
    SecretList {
        reply: oneshot::Sender<Vec<(u64, String)>>,
    },
    /// Store a secret in the system's own name space (`global.`), replacing
    /// whatever is there under that name. That space is closed to the HTTP
    /// secret endpoints, so an entry in it is one only server code wrote.
    /// Any other name is refused.
    SystemSecretSet {
        item: Item,
        reply: oneshot::Sender<Result<(), String>>,
    },
    /// Remove the system secret of this name. `Ok(false)`: there was none.
    SystemSecretDel {
        name: String,
        reply: oneshot::Sender<Result<bool, String>>,
    },

    // --- Features ---
    /// What the deployment is allowed to do, as declared in `features.js`
    /// beside its settings: feature name → the free-form descriptor the
    /// feature carries. Core reads that file at startup and never writes it,
    /// so the answer is the same for the life of the process and there is no
    /// message here that sets one — a plugin cannot grant itself a feature.
    ///
    /// The whole document comes back in one reply rather than a lookup per
    /// name, because it is a handful of entries read from a file and because
    /// a plugin that asks about one feature usually asks about several.
    FeaturesGetAll {
        reply: oneshot::Sender<BTreeMap<String, Value>>,
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
    async fn request<T>(&self, build: impl FnOnce(oneshot::Sender<T>) -> CoreMessage) -> Option<T> {
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

    /// The deployment's disk situation: live metrics, the computed budget and
    /// the active policy. On a channel error it is the all-zero default, whose
    /// `disk_low` is false and whose caps are zero — the safe no-op for a
    /// caller that uses it to size or gate work.
    pub async fn globals_get_disk_report(&self) -> DiskReport {
        self.request(|reply| CoreMessage::GlobalsGetDiskReport { reply })
            .await
            .unwrap_or_default()
    }

    /// Replace the active disk-budget policy; returns whether it was stored.
    pub async fn globals_set_disk_policy(&self, policy: DiskPolicy) -> bool {
        self.request(|reply| CoreMessage::GlobalsSetDiskPolicy { policy, reply })
            .await
            .unwrap_or(false)
    }

    // --- Maintenance ---

    /// Open an operation: work a restart must not cut off. `Ok(token)` to
    /// pass to [`operation_end`](Self::operation_end); `Err` with
    /// [`MAINTENANCE_REFUSED`] when the installation is closed for an
    /// update, and the work must not be started.
    ///
    /// A core that cannot be reached refuses nothing: the token is `0`,
    /// which ending is a no-op for. Work is not held up by a gate that is
    /// not there.
    pub async fn operation_begin(&self, title: &str) -> Result<u64, String> {
        match self
            .request(|reply| CoreMessage::OperationBegin {
                title: title.into(),
                resumed: false,
                reply,
            })
            .await
        {
            Some(Some(token)) => Ok(token),
            Some(None) => Err(MAINTENANCE_REFUSED.to_string()),
            None => Ok(0),
        }
    }

    /// Count an operation that was running before a restart and is being
    /// carried on. Never refused: letting it finish is what a closing waits
    /// for.
    pub async fn operation_resume(&self, title: &str) -> u64 {
        self.request(|reply| CoreMessage::OperationBegin {
            title: title.into(),
            resumed: true,
            reply,
        })
        .await
        .flatten()
        .unwrap_or(0)
    }

    /// Close an operation. Safe to call for a token of `0`.
    pub async fn operation_end(&self, token: u64) {
        if token != 0 {
            let _ = self.tx.send(CoreMessage::OperationEnd { token }).await;
        }
    }

    /// The same, from where nothing can be awaited — a destructor. Best
    /// effort: a full mailbox loses it, and the core then counts the
    /// operation until the next restart.
    pub fn operation_end_blocking(&self, token: u64) {
        if token != 0 {
            let _ = self.tx.try_send(CoreMessage::OperationEnd { token });
        }
    }

    /// Whether the installation takes new operations, and what is open.
    pub async fn maintenance(&self) -> MaintenanceInfo {
        self.request(|reply| CoreMessage::MaintenanceGet { reply })
            .await
            .unwrap_or_default()
    }

    /// Ask for a closing at the next moment no operation is open. How long
    /// to wait and how long to stay closed are the installation's settings.
    pub async fn maintenance_ask(&self, by: &str, reason: &str) -> MaintenanceInfo {
        self.request(|reply| CoreMessage::MaintenanceAsk {
            by: by.into(),
            reason: reason.into(),
            reply,
        })
        .await
        .unwrap_or_default()
    }

    /// Take operations again, and drop a request that is still waiting.
    pub async fn maintenance_reopen(&self, by: &str) -> MaintenanceInfo {
        self.request(|reply| CoreMessage::MaintenanceReopen {
            by: by.into(),
            reply,
        })
        .await
        .unwrap_or_default()
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

    /// Every stored secret as `(id, name)`, sorted by name. `None` from a
    /// core that predates the message, which drops it unanswered.
    pub async fn secret_list(&self) -> Option<Vec<(u64, String)>> {
        self.request(|reply| CoreMessage::SecretList { reply })
            .await
    }

    /// Store `item` as the system secret named by its `name`, which must be
    /// in the `global.` space. Replaces the entry of that name, if any.
    pub async fn system_secret_set(&self, item: Item) -> Result<(), String> {
        self.request(|reply| CoreMessage::SystemSecretSet { item, reply })
            .await
            .unwrap_or_else(|| Err("this core cannot store system secrets".to_string()))
    }

    /// Remove the system secret `name`. `Ok(false)` when there was none.
    pub async fn system_secret_del(&self, name: &str) -> Result<bool, String> {
        let name = name.to_string();
        self.request(|reply| CoreMessage::SystemSecretDel { name, reply })
            .await
            .unwrap_or_else(|| Err("this core cannot remove system secrets".to_string()))
    }

    // --- Features ---
    /// Everything `features.js` declares: name → descriptor.
    ///
    /// An empty map means the deployment declares no features — which is
    /// also what a core too old to answer this message, or one shutting
    /// down, produces. That is the safe reading in all three cases: a
    /// feature nobody granted is not available.
    pub async fn features_all(&self) -> BTreeMap<String, Value> {
        self.request(|reply| CoreMessage::FeaturesGetAll { reply })
            .await
            .unwrap_or_default()
    }

    /// The declared feature names, sorted.
    pub async fn features_list(&self) -> Vec<String> {
        self.features_all().await.into_keys().collect()
    }

    /// One feature's descriptor, exactly as the file spells it.
    ///
    /// `None` means the feature is not declared. A declared feature whose
    /// descriptor is `null` answers `Some(Value::Null)`: the file said
    /// something about it, and what it said was nothing in particular.
    pub async fn features_get(&self, name: &str) -> Option<Value> {
        self.features_all().await.remove(name)
    }

    /// Whether a feature is declared at all.
    ///
    /// The question most callers actually have, and the one place where
    /// "not declared" and "declared as `null`" must not be confused — hence
    /// a method rather than `features_get(..).is_some()` at every call site.
    pub async fn features_has(&self, name: &str) -> bool {
        self.features_all().await.contains_key(name)
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

    const GIB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn plan_budget_reserves_the_larger_of_floor_and_fraction() {
        let pol = DiskPolicy::default();
        // 20 GiB: 10% = 2 GiB < 5 GiB floor → floor wins.
        assert_eq!(plan_budget(20 * GIB, &pol).root_reserve, 5 * GIB);
        // 200 GiB: 10% = 20 GiB > floor → fraction wins.
        assert_eq!(plan_budget(200 * GIB, &pol).root_reserve, 20 * GIB);
    }

    #[test]
    fn plan_budget_splits_usable_and_divides_per_job_by_concurrency() {
        let pol = DiskPolicy::default();
        let b = plan_budget(200 * GIB, &pol);
        let usable = 180 * GIB; // 200 - 20 reserve
        assert_eq!(b.mongo_cap, usable / 2);
        assert_eq!(b.upload_cap, (usable as f64 * 0.30) as u64);
        assert_eq!(b.job_scratch_total, (usable as f64 * 0.20) as u64);
        assert_eq!(b.per_job_disk, b.job_scratch_total / 4);
    }

    #[test]
    fn plan_budget_is_total_on_a_tiny_disk_and_never_divides_by_zero() {
        // Smaller than the floor: the whole disk is the reserve, caps are zero.
        let b = plan_budget(2 * GIB, &DiskPolicy::default());
        assert_eq!(b.root_reserve, 2 * GIB);
        assert_eq!(b.per_job_disk, 0);
        // Zero concurrency is treated as one job.
        let pol = DiskPolicy {
            max_concurrent_jobs: 0,
            ..DiskPolicy::default()
        };
        let b = plan_budget(200 * GIB, &pol);
        assert_eq!(b.per_job_disk, b.job_scratch_total);
    }

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
