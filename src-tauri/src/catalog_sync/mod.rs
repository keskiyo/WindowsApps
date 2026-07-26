//! Catalog synchronization glue, split by responsibility: reading the sanitized cache document
//! ([`document`]), background icon/metadata [`hydration`], coordinated [`scan`]ning, and the
//! filesystem-change [`watcher`]. All run against the shared `AppState` resolved from a Tauri
//! `AppHandle`. Public entry points are re-exported so `crate::catalog_sync::<name>` paths stay
//! stable for `commands/*` and `lib.rs`.

mod document;
mod hydration;
mod scan;
mod watcher;

pub(crate) use document::{load_sanitized_cache, load_sanitized_document};
pub(crate) use hydration::enqueue_hydration;
pub(crate) use scan::run_coordinated_scan;
pub(crate) use watcher::restart_change_watcher;
