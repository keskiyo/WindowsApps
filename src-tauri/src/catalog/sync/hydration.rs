//! Background icon/metadata hydration: a single worker drains the shared hydration queue for a
//! generation, emits patch batches to the frontend, then merges them back into the cache document.

use crate::app_state::AppState;
use crate::catalog;
use crate::catalog::cache::{self, CatalogCache};
use std::collections::HashMap;
use tauri::{Emitter, Manager};

pub(crate) fn enqueue_hydration(
    app: tauri::AppHandle,
    app_data_dir: std::path::PathBuf,
    generation: u64,
    ids: Vec<String>,
    priority: bool,
) {
    let should_start = {
        let state = app.state::<AppState>();
        state
            .hydration_queue
            .lock()
            .is_ok_and(|mut queue| queue.enqueue(generation, ids, priority))
    };
    if !should_start {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let hydration_dir = app_data_dir.clone();
        let worker_app = app.clone();
        let hydrated = tauri::async_runtime::spawn_blocking(move || {
            let state = worker_app.state::<AppState>();
            let Some(document) = cache::read_document(&hydration_dir) else {
                return Vec::new();
            };
            if document.generation != generation {
                return Vec::new();
            }
            let apps = document
                .apps
                .into_iter()
                .map(|app| (app.id.clone(), app))
                .collect::<HashMap<_, _>>();
            // Patches are emitted in batches rather than one event per icon: the frontend
            // rebuilds its whole app list per `catalog://patches` event, so ~N events for N
            // apps caused O(N^2) work and main-thread jank (cards flickering in one-by-one,
            // delayed hover animations). Batching collapses ~N events into ~N/BATCH.
            const BATCH: usize = 24;
            let mut patches = Vec::new();
            let mut batch: Vec<catalog::hydration::AppHydrationPatch> = Vec::new();
            let mut written_icons: Vec<(String, String)> = Vec::new();
            loop {
                let id = {
                    let Ok(mut queue) = state.hydration_queue.lock() else {
                        break;
                    };
                    let id = queue.pop(generation);
                    if id.is_none() {
                        queue.finish(generation);
                    }
                    id
                };
                let Some(id) = id else {
                    break;
                };
                if let Some(app_info) = apps.get(&id) {
                    let outcome =
                        catalog::hydration::hydrate_one(&hydration_dir, app_info, generation);
                    if let Some(written) = outcome.written_icon {
                        written_icons.push(written);
                    }
                    batch.push(outcome.patch.clone());
                    patches.push(outcome.patch);
                    if batch.len() >= BATCH {
                        let _ = worker_app.emit("catalog://patches", &batch);
                        batch.clear();
                    }
                }
                if let Ok(mut queue) = state.hydration_queue.lock() {
                    queue.complete(generation, &id);
                }
            }
            if !batch.is_empty() {
                let _ = worker_app.emit("catalog://patches", &batch);
            }
            // One directory pass for the whole batch. Sweeping per written icon re-read the
            // entire icons directory N times for N icons.
            catalog::icon_cache::sweep_superseded(&hydration_dir, &written_icons);
            patches
        })
        .await;
        let Ok(patches) = hydrated else {
            return;
        };
        if patches.is_empty() {
            return;
        }
        let post_state = app.state::<AppState>();
        let Ok(_guard) = post_state.sync_lock.lock() else {
            return;
        };
        let Some(mut document) = cache::read_document(&app_data_dir) else {
            return;
        };
        if document.generation != generation {
            return;
        }
        apply_hydration_patches_to_document(&mut document, patches);
        let _ = cache::write_document(&app_data_dir, &document);
    });
}

fn apply_hydration_patches_to_document(
    document: &mut CatalogCache,
    patches: Vec<catalog::hydration::AppHydrationPatch>,
) {
    let patches = patches
        .into_iter()
        .map(|patch| (patch.id.clone(), patch))
        .collect::<HashMap<_, _>>();
    for target in &mut document.apps {
        let Some(patch) = patches.get(&target.id) else {
            continue;
        };
        target.description = patch.description.clone();
        target.version = patch.version.clone();
        target.publisher = patch.publisher.clone();
        target.product_name = patch.product_name.clone();
        target.original_filename = patch.original_filename.clone();
        target.install_location = patch.install_location.clone();
        target.can_uninstall = patch.can_uninstall.unwrap_or(target.can_uninstall);
        if patch.icon_base64.is_some() {
            target.icon_base64 = patch.icon_base64.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hydration_patches_persist_icons_without_erasing_existing_icons() {
        use crate::app_state::cached_app;

        let mut first = cached_app("Code", r"C:\Code.exe");
        first.id = "code".into();
        let mut second = cached_app("Claude", r"C:\Claude.exe");
        second.id = "claude".into();
        second.icon_base64 = Some("data:image/png;base64,old".into());
        let mut document = CatalogCache {
            apps: vec![first, second],
            ..CatalogCache::default()
        };

        apply_hydration_patches_to_document(
            &mut document,
            vec![
                catalog::hydration::AppHydrationPatch {
                    id: "code".into(),
                    generation: 1,
                    icon_base64: Some("data:image/png;base64,new".into()),
                    description: None,
                    version: None,
                    publisher: None,
                    product_name: None,
                    original_filename: None,
                    install_location: None,
                    can_uninstall: None,
                },
                catalog::hydration::AppHydrationPatch {
                    id: "claude".into(),
                    generation: 1,
                    icon_base64: None,
                    description: None,
                    version: None,
                    publisher: None,
                    product_name: None,
                    original_filename: None,
                    install_location: None,
                    can_uninstall: None,
                },
            ],
        );

        assert_eq!(
            document.apps[0].icon_base64.as_deref(),
            Some("data:image/png;base64,new")
        );
        assert_eq!(
            document.apps[1].icon_base64.as_deref(),
            Some("data:image/png;base64,old")
        );
    }
}
