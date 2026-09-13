//! What a plugin learns about the deployment's feature set.
//!
//! Core answers `FeaturesGetAll` out of the `features.js` it read at startup.
//! These tests stand a fake core behind the channel, because what is worth
//! pinning here is not the file — that is core's business — but the shape of
//! the answer a plugin gets, including the answer it gets when nobody is
//! listening.

use isabelle_plugin_api::actor::{CoreHandle, CoreMessage};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use tokio::sync::mpsc;

/// A declaration with the three descriptor shapes that behave differently:
/// an object, a bare scalar and `null`.
fn declared() -> BTreeMap<String, Value> {
    let mut map = BTreeMap::new();
    map.insert(
        "reports".to_string(),
        json!({ "formats": ["pdf", "csv"], "retention_days": 90 }),
    );
    map.insert("sso".to_string(), json!(true));
    map.insert("beta_ui".to_string(), Value::Null);
    map
}

/// Stand a core behind the handle that answers every `FeaturesGetAll` with
/// `features` and ignores everything else.
fn core_answering(features: BTreeMap<String, Value>) -> CoreHandle {
    let (tx, mut rx) = mpsc::channel::<CoreMessage>(8);
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let CoreMessage::FeaturesGetAll { reply } = msg {
                let _ = reply.send(features.clone());
            }
        }
    });
    CoreHandle::new(tx)
}

/// The descriptor crosses intact. A plugin is the code that implements the
/// feature, so it is the one thing that has to see the whole of what the
/// deployment said about it.
#[tokio::test]
async fn a_plugin_receives_descriptors_unchanged() {
    let core = core_answering(declared());

    assert_eq!(
        core.features_get("reports").await,
        Some(json!({ "formats": ["pdf", "csv"], "retention_days": 90 }))
    );
    assert_eq!(core.features_get("sso").await, Some(json!(true)));
    assert_eq!(core.features_all().await, declared());
}

/// A feature declared with a `null` descriptor is declared. Asking whether
/// it exists and asking what it says are different questions, and only
/// `features_has` answers the first one correctly.
#[tokio::test]
async fn a_null_descriptor_is_still_a_declared_feature() {
    let core = core_answering(declared());

    assert!(core.features_has("beta_ui").await);
    assert_eq!(core.features_get("beta_ui").await, Some(Value::Null));

    assert!(!core.features_has("nothing_like_it").await);
    assert_eq!(core.features_get("nothing_like_it").await, None);
}

/// The names come back sorted, because the map they come out of is ordered.
/// A plugin putting them on a screen should not have to sort them again to
/// get the same screen twice.
#[tokio::test]
async fn the_names_are_sorted() {
    let mut map = BTreeMap::new();
    for name in ["zeta", "alpha", "Mixed", "beta"] {
        map.insert(name.to_string(), Value::Null);
    }
    let core = core_answering(map);

    assert_eq!(
        core.features_list().await,
        vec!["Mixed", "alpha", "beta", "zeta"]
    );
}

/// A deployment that declares nothing is not an error, and neither is a core
/// that has gone away: both mean no feature is available. The second one is
/// what a plugin sees during shutdown, and it must not be the moment a
/// plugin decides everything is permitted.
#[tokio::test]
async fn nothing_is_available_when_there_is_nothing_to_ask() {
    let empty = core_answering(BTreeMap::new());
    assert!(empty.features_all().await.is_empty());
    assert!(empty.features_list().await.is_empty());
    assert!(!empty.features_has("reports").await);
    assert_eq!(empty.features_get("reports").await, None);

    // Core gone: the channel is closed, so the request cannot even be sent.
    let (tx, rx) = mpsc::channel::<CoreMessage>(1);
    drop(rx);
    let gone = CoreHandle::new(tx);
    assert!(gone.features_all().await.is_empty());
    assert!(!gone.features_has("reports").await);
    assert_eq!(gone.features_get("reports").await, None);
}
