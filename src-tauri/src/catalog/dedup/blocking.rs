//! The index that keeps resolution from comparing every pair of candidates.
//!
//! Separate from `candidate` because it answers a different question: `candidate` decides what two
//! entries *are*, this decides which pairs are even worth asking about. Every relation in
//! `evidence` that can reach the merge threshold is either an equality on one of these keys or
//! directory containment, so a pair sharing no key cannot merge and skipping it changes nothing —
//! a claim the differential test in `mod` pins against an unnarrowed reference resolver.

use super::candidate::AppCandidate;
use super::target::{normalize_path, system_tool_alias};
use crate::catalog::SourceKind;
use std::collections::HashMap;

/// Narrows the resolver's search. Every relation in `collect_evidence` that can reach the merge
/// threshold is either an equality on one of the keys below, or directory containment between a
/// registry install root and an executable — which, now that containment stops at a component
/// boundary, is exactly "one path is an ancestor of the other". Two candidates that share no
/// key therefore cannot merge, and skipping them changes nothing.
///
/// The differential test in this module pins that claim: it compares this against an
/// unnarrowed reference resolver on generated catalogs.
#[derive(Default)]
pub(super) struct BlockingIndex {
    equality: HashMap<String, Vec<usize>>,
    /// Registry install roots, looked up by an executable's ancestor directories.
    install_roots: HashMap<String, Vec<usize>>,
    /// Executable ancestor directories, looked up by a registry install root.
    executable_ancestors: HashMap<String, Vec<usize>>,
}

/// Prefixed so unrelated fields cannot collide on the same string.
pub(super) fn equality_keys(candidate: &AppCandidate) -> Vec<String> {
    let identity = &candidate.identity;
    let mut keys = vec![
        format!("path:{}", identity.path),
        // `launcher_family` is a truncation of `family`, so equal families always share it;
        // every family-based relation is covered by this single key.
        format!("lfam:{}", candidate.launcher_family),
    ];
    // A shortcut merges with the executable it resolves to, so it is indexed under that path
    // as well as its own.
    if let Some(target) = candidate.app.resolved_path.as_deref() {
        keys.push(format!("path:{}", normalize_path(target)));
    }
    if let Some(target) = identity.launch_target.as_deref() {
        keys.push(format!("target:{target}"));
    }
    if let Some(value) = identity.steam_app_id.as_deref() {
        keys.push(format!("steam:{value}"));
    }
    if let Some(value) = identity.aumid.as_deref() {
        keys.push(format!("aumid:{value}"));
    }
    if let Some(value) = candidate.squirrel_package.as_deref() {
        keys.push(format!("squirrel:{value}"));
    }
    if let Some(value) = system_tool_alias(&candidate.app) {
        keys.push(format!("alias:{value}"));
    }
    if let Some(parent) = candidate.parent.as_deref() {
        keys.push(format!("parent:{parent}"));
    }
    keys
}

fn registry_install_root(candidate: &AppCandidate) -> Option<&str> {
    (candidate.app.source_kind == SourceKind::Registry)
        .then(|| {
            candidate
                .identity
                .install_root
                .as_deref()
                .map(|root| root.trim_end_matches('\\'))
                .filter(|root| !root.is_empty())
        })
        .flatten()
}

fn is_executable_path(path: &str) -> bool {
    path.to_lowercase().ends_with(".exe")
}

/// The path itself and every directory above it, without trailing separators — the same
/// notion of containment `path_is_within` implements.
fn path_ancestors(path: &str) -> Vec<&str> {
    let mut ancestors = vec![path];
    let mut rest = path;
    while let Some(separator) = rest.rfind('\\') {
        rest = &rest[..separator];
        if rest.is_empty() {
            break;
        }
        ancestors.push(rest);
    }
    ancestors
}

impl BlockingIndex {
    pub(super) fn candidate_groups(&self, candidate: &AppCandidate, keys: &[String]) -> Vec<usize> {
        let mut groups = Vec::new();
        for key in keys {
            if let Some(found) = self.equality.get(key) {
                groups.extend_from_slice(found);
            }
        }
        if is_executable_path(&candidate.app.path) {
            for ancestor in path_ancestors(&candidate.identity.path) {
                if let Some(found) = self.install_roots.get(ancestor) {
                    groups.extend_from_slice(found);
                }
            }
        }
        if let Some(root) = registry_install_root(candidate) {
            if let Some(found) = self.executable_ancestors.get(root) {
                groups.extend_from_slice(found);
            }
        }
        groups.sort_unstable();
        groups.dedup();
        groups
    }

    pub(super) fn insert(&mut self, candidate: &AppCandidate, keys: &[String], group: usize) {
        for key in keys {
            self.equality.entry(key.clone()).or_default().push(group);
        }
        if is_executable_path(&candidate.app.path) {
            for ancestor in path_ancestors(&candidate.identity.path) {
                self.executable_ancestors
                    .entry(ancestor.to_string())
                    .or_default()
                    .push(group);
            }
        }
        if let Some(root) = registry_install_root(candidate) {
            self.install_roots
                .entry(root.to_string())
                .or_default()
                .push(group);
        }
    }
}
