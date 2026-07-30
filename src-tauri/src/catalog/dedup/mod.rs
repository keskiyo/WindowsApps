use crate::catalog::{AppCategory, AppInfo, LaunchKind, SourceKind};
use crate::platform::windows::NameScript;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Component, Path};

mod report;

pub(crate) use report::{dev_report_enabled, write_dev_report};

#[derive(Clone, Debug)]
pub(super) struct AppCandidate {
    app: AppInfo,
    family: String,
    identity: CandidateIdentity,
    /// Derived once per candidate. Each of these used to be recomputed inside the pairwise
    /// predicates — that is, O(n) times per candidate — and every one of them allocates.
    launcher_family: String,
    helper_family: String,
    publisher_key: String,
    parent: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct CandidateIdentity {
    steam_app_id: Option<String>,
    aumid: Option<String>,
    launch_target: Option<String>,
    launch_mode: Option<String>,
    install_root: Option<String>,
    registry_product: Option<String>,
    portable_product: Option<String>,
    path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Evidence {
    reason: EvidenceReason,
    score: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum EvidenceReason {
    SamePath,
    SameLaunchTarget,
    SameSystemToolAlias,
    ShortcutTargetsExecutable,
    SameSteamAppId,
    SameAumid,
    SameFolderAndFamily,
    SameInstallRootAndFamily,
    NestedInstallRootAndFamily,
    RegistryInstallContainsExecutable,
    SamePublisherAndFamily,
    SamePackagedFamily,
    ShortcutSameFamily,
    VersionedPortableCopy,
    SameVersionPortableCopy,
    SameFolderHelperVariant,
    SameFamily,
    NamePrefixOnly,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct ResolverReport {
    pub candidates: usize,
    pub merged: usize,
    pub evidence: Vec<Evidence>,
    pub possible_duplicates: usize,
}

#[derive(Clone, Debug)]
pub(super) struct ResolvedApp {
    app: AppInfo,
    candidates: Vec<AppCandidate>,
    evidence: Vec<Evidence>,
}

/// Total ordering key that canonicalizes the resolver's input (see `deduplicate`). Strongest
/// candidate first (so it anchors its group), then every field that can decide whether two entries
/// merge, so the processing order is fixed by catalog content and never by scan order. Any residual
/// tie is between entries that share a normalized path — those merge regardless of order.
fn canonical_order_key(app: &AppInfo) -> (u8, String) {
    let key = format!(
        "{}\u{1}{}\u{1}{:?}\u{1}{:?}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}",
        normalize_path(&app.path),
        app.name.to_lowercase(),
        app.source_kind,
        app.launch_kind,
        app.resolved_path
            .as_deref()
            .map(normalize_path)
            .unwrap_or_default(),
        app.install_location
            .as_deref()
            .map(normalize_path)
            .unwrap_or_default(),
        app.publisher.as_deref().unwrap_or_default().to_lowercase(),
        app.version.as_deref().unwrap_or_default(),
        app.launch_arguments.as_deref().unwrap_or_default(),
    );
    (u8::MAX - candidate_score(app), key)
}

pub(super) fn deduplicate(
    mut apps: Vec<AppInfo>,
    classify: impl Fn(&AppInfo) -> AppCategory,
    os_script: NameScript,
) -> Vec<AppInfo> {
    // Canonicalize the input order so deduplication is deterministic and idempotent. The resolver
    // is order-sensitive ("first matching group wins") and merge relations are not transitive, so
    // the same catalog scanned in a different source order used to merge differently; a re-dedup
    // then reduced further. The live scan and a reload disagreed, and the frontend delta path
    // accumulated the least-merged variant — the duplicate cards that reappeared after a background
    // sync. The sort key must be *total*: strongest candidate first (so it anchors each group),
    // then every field that could make two entries merge or not, so any residual tie is between
    // entries that share a path and therefore merge anyway.
    //
    // Run to a fixed point. A single pass is not idempotent: "first matching group wins" lets a
    // candidate join the first group it matches while a later group it also matched stays separate,
    // so a second pass reduces further. In production the live scan dedups once into the cache and a
    // reload dedups again — they must agree. A pass that does not reduce the count performed no
    // merges, so the result is stable; that is the fixed point.
    loop {
        let before = apps.len();
        apps.sort_by_cached_key(canonical_order_key);
        let mut report = ResolverReport::default();
        apps = resolve_apps(apps, &mut report)
            .into_iter()
            .map(|mut resolved| {
                // Pick the card's display name by the user's OS language. `resolved.app` already
                // holds the highest-scored source (its launch target and icon), but its name may be
                // in the wrong script for the user — a Russian shortcut on an English machine. The
                // name is chosen independently so a non-Russian user reads a Latin name when one
                // exists, without changing which source actually launches.
                resolved.app.name = choose_display_name(
                    &resolved.app.name,
                    resolved
                        .candidates
                        .iter()
                        .map(|candidate| candidate.app.name.as_str()),
                    os_script,
                );
                resolved.app.id = resolved_canonical_id(&resolved);
                let product_identity = preference_identity(&resolved.app);
                resolved.app.preference_identity =
                    Some(card_preference_identity(&resolved.app, &product_identity));
                resolved.app.canonical_identity = Some(product_identity);
                resolved.app
            })
            .collect::<Vec<_>>();
        if apps.len() == before {
            break;
        }
    }
    for app in &mut apps {
        app.name = app.name.split_whitespace().collect::<Vec<_>>().join(" ");
        app.category = classify(app);
    }
    apps.sort_by_cached_key(|app| (category_rank(app.category), app.name.to_lowercase()));
    apps
}

fn resolve_apps(apps: Vec<AppInfo>, report: &mut ResolverReport) -> Vec<ResolvedApp> {
    report.candidates = apps.len();
    let mut resolved = Vec::<ResolvedApp>::new();
    let mut index = BlockingIndex::default();
    for app in apps {
        let candidate = AppCandidate::from(app);
        let keys = equality_keys(&candidate);
        // Only groups sharing at least one blocking key can possibly merge, so the scan no
        // longer touches every group. `candidate_groups` yields ascending indices, which keeps
        // the original "first matching group wins" behaviour.
        let matched = index
            .candidate_groups(&candidate, &keys)
            .into_iter()
            .find(|&group| should_merge(&resolved[group], &candidate));
        match matched {
            Some(group) => {
                // The group is matched against *all* of its candidates, so it inherits the
                // newcomer's keys too.
                index.insert(&candidate, &keys, group);
                // Merge in place: `remove` + `insert` at the same index shifted the whole tail
                // of the vector twice per merge, moving every following `ResolvedApp` for
                // nothing.
                merge_resolved(&mut resolved[group], candidate, report);
            }
            None => {
                let group = resolved.len();
                index.insert(&candidate, &keys, group);
                resolved.push(ResolvedApp {
                    app: candidate.app.clone(),
                    candidates: vec![candidate],
                    evidence: Vec::new(),
                });
            }
        }
    }
    resolved
}

/// Narrows the resolver's search. Every relation in `collect_evidence` that can reach the merge
/// threshold is either an equality on one of the keys below, or directory containment between a
/// registry install root and an executable — which, now that containment stops at a component
/// boundary, is exactly "one path is an ancestor of the other". Two candidates that share no
/// key therefore cannot merge, and skipping them changes nothing.
///
/// The differential test in this module pins that claim: it compares this against an
/// unnarrowed reference resolver on generated catalogs.
#[derive(Default)]
struct BlockingIndex {
    equality: HashMap<String, Vec<usize>>,
    /// Registry install roots, looked up by an executable's ancestor directories.
    install_roots: HashMap<String, Vec<usize>>,
    /// Executable ancestor directories, looked up by a registry install root.
    executable_ancestors: HashMap<String, Vec<usize>>,
}

impl BlockingIndex {
    fn candidate_groups(&self, candidate: &AppCandidate, keys: &[String]) -> Vec<usize> {
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

    fn insert(&mut self, candidate: &AppCandidate, keys: &[String], group: usize) {
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

/// Prefixed so unrelated fields cannot collide on the same string.
fn equality_keys(candidate: &AppCandidate) -> Vec<String> {
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

impl From<AppInfo> for AppCandidate {
    fn from(app: AppInfo) -> Self {
        let family = normalized_product_family(&app.name);
        let identity = CandidateIdentity::from_app(&app);
        Self {
            launcher_family: launcher_product_family(&family).to_string(),
            helper_family: helper_variant_family(&family),
            publisher_key: normalized_publisher(app.publisher.as_deref()),
            parent: parent_path(&identity.path),
            family,
            identity,
            app,
        }
    }
}

impl CandidateIdentity {
    fn from_app(app: &AppInfo) -> Self {
        let family = normalized_product_family(&app.name);
        let publisher = normalized_publisher(app.publisher.as_deref());
        let install_root = app.install_location.as_deref().map(normalize_path);
        Self {
            steam_app_id: steam_app_id(app).map(str::to_string),
            aumid: (app.launch_kind == LaunchKind::AppUserModelId)
                .then(|| app.path.trim().to_lowercase()),
            launch_target: launch_target(app).map(normalize_path),
            launch_mode: meaningful_launch_arguments(app.launch_arguments.as_deref()),
            registry_product: (app.source_kind == SourceKind::Registry).then(|| {
                format!(
                    "{}|{}|{}",
                    publisher,
                    family,
                    install_root.clone().unwrap_or_default()
                )
            }),
            portable_product: (app.source_kind == SourceKind::Portable)
                .then(|| format!("{}|{}", install_root.clone().unwrap_or_default(), family)),
            install_root,
            path: normalize_path(&app.path),
        }
    }
}

fn should_merge(existing: &ResolvedApp, candidate: &AppCandidate) -> bool {
    if existing.candidates.iter().any(|left| {
        both_unversioned_portable_copies(left, candidate) && !same_portable_root(left, candidate)
    }) {
        return false;
    }
    if existing.candidates.iter().any(|left| {
        left.family == candidate.family
            && matches!(
                (
                    left.identity.launch_mode.as_ref(),
                    candidate.identity.launch_mode.as_ref()
                ),
                (Some(left), Some(right)) if left != right
            )
    }) {
        return false;
    }
    let best = existing
        .candidates
        .iter()
        .map(|left| summarize_evidence(left, candidate))
        .max_by_key(|summary| summary.score);
    let Some(summary) = best else {
        return false;
    };
    // Weak, name-only evidence (score < 75) must not merge across a conflicting install root or a
    // conflicting version: those are two different installs, or two different releases the user
    // treats as different applications (7-Zip 22 vs 24, an old and new portable copy). Strong
    // structural evidence (>= 75: same or nested install root, same product folder, identity)
    // overrides — a launcher and its game exe in one install tree carry different versions but are
    // one application, so they still merge.
    if summary.score < 75
        && existing.candidates.iter().any(|left| {
            conflicting_install_roots(left, candidate) || conflicting_versions(left, candidate)
        })
    {
        return false;
    }
    if summary.score >= 80 {
        return true;
    }
    if summary.score >= 50 && !publishers_conflict(&existing.app, &candidate.app) {
        return true;
    }
    summary.has_identity_match
}

/// Both entries carry a version and the two differ — they are distinct releases of the product,
/// which the user treats as distinct applications, so a name-level merge must not collapse them.
fn conflicting_versions(left: &AppCandidate, right: &AppCandidate) -> bool {
    matches!(
        (
            normalized_version_string(&left.app),
            normalized_version_string(&right.app)
        ),
        (Some(left), Some(right)) if left != right
    )
}

fn conflicting_install_roots(left: &AppCandidate, right: &AppCandidate) -> bool {
    matches!(
        (
            left.identity.install_root.as_ref(),
            right.identity.install_root.as_ref()
        ),
        (Some(left), Some(right))
            if left != right
                && !left.starts_with(&format!("{right}\\"))
                && !right.starts_with(&format!("{left}\\"))
    )
}

fn merge_resolved(
    existing: &mut ResolvedApp,
    candidate: AppCandidate,
    report: &mut ResolverReport,
) {
    // Only the winning pair needs itemized evidence, so this is the one place that builds it.
    let (evidence, _score) = existing
        .candidates
        .iter()
        .map(|left| score_evidence(left, &candidate))
        .max_by_key(|(_, score)| *score)
        .unwrap_or_default();
    report.merged += 1;
    report.evidence.extend(evidence.iter().cloned());
    existing.evidence.extend(evidence);
    existing.app = merge_app(existing.app.clone(), candidate.app.clone());
    existing.candidates.push(candidate);
}

/// Score without building an evidence list. `should_merge` runs this for every candidate pair,
/// so allocating a `Vec` per comparison cost one allocation per pair across the whole catalog;
/// only the pair that actually wins a merge needs the itemized evidence.
fn summarize_evidence(left: &AppCandidate, right: &AppCandidate) -> EvidenceSummary {
    let mut summary = EvidenceSummary::default();
    collect_evidence(left, right, |reason, score| {
        summary.score = summary.score.max(score);
        summary.has_identity_match |= matches!(
            reason,
            EvidenceReason::SamePath
                | EvidenceReason::SameLaunchTarget
                | EvidenceReason::ShortcutTargetsExecutable
                | EvidenceReason::SameSteamAppId
                | EvidenceReason::SameAumid
        );
    });
    summary
}

/// The strongest signals, which merge a pair even when the numeric score alone would not.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct EvidenceSummary {
    score: u16,
    has_identity_match: bool,
}

fn score_evidence(left: &AppCandidate, right: &AppCandidate) -> (Vec<Evidence>, u16) {
    let mut evidence = Vec::new();
    collect_evidence(left, right, |reason, score| {
        evidence.push(Evidence { reason, score })
    });
    let score = evidence.iter().map(|item| item.score).max().unwrap_or(0);
    (evidence, score)
}

/// Single source of truth for what counts as evidence. Both the allocating and the
/// non-allocating consumers above feed off this, so the two can never drift apart.
fn collect_evidence(
    left: &AppCandidate,
    right: &AppCandidate,
    mut add: impl FnMut(EvidenceReason, u16),
) {
    if left.identity.path == right.identity.path {
        add(EvidenceReason::SamePath, 100);
    }
    if shared(
        left.identity.launch_target.as_ref(),
        right.identity.launch_target.as_ref(),
    ) && left.identity.launch_mode == right.identity.launch_mode
    {
        add(EvidenceReason::SameLaunchTarget, 100);
    }
    if shortcut_targets_executable(&left.app, &right.app) {
        add(EvidenceReason::ShortcutTargetsExecutable, 100);
    }
    if let (Some(left_alias), Some(right_alias)) =
        (system_tool_alias(&left.app), system_tool_alias(&right.app))
    {
        if left_alias == right_alias {
            add(EvidenceReason::SameSystemToolAlias, 90);
        }
    }
    if shared(
        left.identity.steam_app_id.as_ref(),
        right.identity.steam_app_id.as_ref(),
    ) {
        add(EvidenceReason::SameSteamAppId, 100);
    }
    if shared(left.identity.aumid.as_ref(), right.identity.aumid.as_ref()) {
        add(EvidenceReason::SameAumid, 100);
    }
    if left.family == right.family
        && shared(
            left.identity.install_root.as_ref(),
            right.identity.install_root.as_ref(),
        )
    {
        add(EvidenceReason::SameInstallRootAndFamily, 80);
    }
    if nested_install_root_and_family(left, right) {
        add(EvidenceReason::NestedInstallRootAndFamily, 75);
    }
    if same_folder_and_family(left, right) {
        add(EvidenceReason::SameFolderAndFamily, 80);
    }
    if registry_install_contains_exe(left, right) {
        add(EvidenceReason::RegistryInstallContainsExecutable, 75);
    }
    if left.family == right.family
        && !left.publisher_key.is_empty()
        && left.publisher_key == right.publisher_key
    {
        add(EvidenceReason::SamePublisherAndFamily, 60);
    }
    if one_is_aumid(left, right) && left.family == right.family {
        add(EvidenceReason::SamePackagedFamily, 80);
    }
    if shortcut_same_family(left, right) {
        add(EvidenceReason::ShortcutSameFamily, 60);
    }
    if versioned_portable_copy(left, right) {
        add(EvidenceReason::VersionedPortableCopy, 60);
    }
    // Same product, same exact version, with a portable copy involved: one program in two places.
    // Score 80 so it merges outright — past the install-root veto (roots differ by definition) and
    // past a vendor-name variant. Different versions are different programs and never reach here.
    if same_version_portable_copy(left, right) {
        add(EvidenceReason::SameVersionPortableCopy, 80);
    }
    if same_folder_helper_variant(left, right) {
        add(EvidenceReason::SameFolderHelperVariant, 80);
    }
    if left.family == right.family {
        add(EvidenceReason::SameFamily, 60);
    } else if left.launcher_family == right.launcher_family {
        add(EvidenceReason::NamePrefixOnly, 10);
    }
}

fn same_folder_and_family(left: &AppCandidate, right: &AppCandidate) -> bool {
    left.parent.is_some()
        && left.parent == right.parent
        && left.launcher_family == right.launcher_family
}

fn nested_install_root_and_family(left: &AppCandidate, right: &AppCandidate) -> bool {
    let left_root = left.identity.install_root.as_ref();
    let right_root = right.identity.install_root.as_ref();
    let same_family = left.launcher_family == right.launcher_family;
    same_family
        && matches!(
            (left_root, right_root),
            (Some(left), Some(right))
                if left.starts_with(&format!("{right}\\"))
                    || right.starts_with(&format!("{left}\\"))
        )
}

fn same_folder_helper_variant(left: &AppCandidate, right: &AppCandidate) -> bool {
    left.parent.is_some()
        && left.parent == right.parent
        && left.helper_family == right.helper_family
        && (is_helper_candidate(&left.app) || is_helper_candidate(&right.app))
}

fn shortcut_same_family(left: &AppCandidate, right: &AppCandidate) -> bool {
    (left.app.launch_kind == LaunchKind::Shortcut || right.app.launch_kind == LaunchKind::Shortcut)
        && left.launcher_family == right.launcher_family
}

fn versioned_portable_copy(left: &AppCandidate, right: &AppCandidate) -> bool {
    left.app.source_kind == SourceKind::Portable
        && right.app.source_kind == SourceKind::Portable
        && left.family == right.family
        && (left.app.version.is_some() || right.app.version.is_some())
}

/// The same product at the same exact version where at least one side is a loose portable copy:
/// a program placed in two locations (a Desktop copy beside its installed shortcut, or the same
/// portable in two folders). One version is one program, so merge across install roots and even
/// a vendor-name variant ("Mozilla Corporation" vs "Mozilla Foundation"). Requires a portable
/// side so two distinct installed products that merely share a version are never merged this way.
fn same_version_portable_copy(left: &AppCandidate, right: &AppCandidate) -> bool {
    (left.app.source_kind == SourceKind::Portable || right.app.source_kind == SourceKind::Portable)
        && left.family == right.family
        && matches!(
            (
                normalized_version_string(&left.app),
                normalized_version_string(&right.app)
            ),
            (Some(left), Some(right)) if left == right
        )
}

/// Version compared as its exact normalized string, not numerically: "5.3.7.0 Beta" and
/// "5.3.7.0" are treated as different releases even though their digits match.
fn normalized_version_string(app: &AppInfo) -> Option<String> {
    app.version
        .as_deref()
        .map(|value| {
            value
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        })
        .filter(|value| !value.is_empty())
}

fn both_unversioned_portable_copies(left: &AppCandidate, right: &AppCandidate) -> bool {
    left.app.source_kind == SourceKind::Portable
        && right.app.source_kind == SourceKind::Portable
        && left.family == right.family
        && left.app.version.is_none()
        && right.app.version.is_none()
}

fn same_portable_root(left: &AppCandidate, right: &AppCandidate) -> bool {
    shared(
        left.identity.install_root.as_ref(),
        right.identity.install_root.as_ref(),
    )
}

fn parent_path(path: &str) -> Option<String> {
    path.rsplit_once('\\').map(|(parent, _)| parent.to_string())
}

fn helper_variant_family(value: &str) -> String {
    let mut family = launcher_product_family(value).to_string();
    for suffix in [
        " helper",
        " updater",
        " update",
        " crash reporter",
        " crashhandler",
        " service",
    ] {
        if family.ends_with(suffix) {
            family.truncate(family.len() - suffix.len());
            break;
        }
    }
    family.trim().to_string()
}

fn is_helper_candidate(app: &AppInfo) -> bool {
    let value = format!("{} {}", normalize_name(&app.name), app.path.to_lowercase());
    [
        " helper",
        " updater",
        " update.exe",
        " crash reporter",
        " crashhandler",
        " service.exe",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

fn shared(left: Option<&String>, right: Option<&String>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if !left.is_empty() && left == right)
}

fn shortcut_targets_executable(left: &AppInfo, right: &AppInfo) -> bool {
    let (shortcut, executable) = if left.launch_kind == LaunchKind::Shortcut {
        (left, right)
    } else if right.launch_kind == LaunchKind::Shortcut {
        (right, left)
    } else {
        return false;
    };
    let Some(target) = shortcut.resolved_path.as_deref() else {
        return false;
    };
    if meaningful_launch_arguments(shortcut.launch_arguments.as_deref()).is_some() {
        return false;
    }
    normalize_path(target) == normalize_path(&executable.path)
}

fn meaningful_launch_arguments(value: Option<&str>) -> Option<String> {
    let tokens = tokenize_quoted_arguments(value?);
    let mut meaningful = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index].trim_matches('"').to_lowercase();
        let takes_value = matches!(
            token.as_str(),
            "--profile-directory"
                | "--user-data-dir"
                | "--app"
                | "--app-id"
                | "--class"
                | "-p"
                | "/k"
                | "/c"
                | "-c"
                | "-command"
                | "-file"
        );
        let inline = [
            "--profile-directory=",
            "--user-data-dir=",
            "--app=",
            "--app-id=",
            "--class=",
        ]
        .iter()
        .any(|prefix| token.starts_with(prefix));
        let standalone = matches!(
            token.as_str(),
            "--safe-mode" | "--incognito" | "--private-window" | "--guest" | "--kiosk"
        );
        if standalone {
            meaningful.push(token);
        } else if inline {
            let (key, value) = token.split_once('=').expect("inline argument has equals");
            meaningful.push(format!("{key}={}", normalize_argument_value(key, value)));
        } else if takes_value {
            meaningful.push(token);
            if let Some(next) = tokens.get(index + 1) {
                meaningful.push(normalize_argument_value(
                    meaningful.last().expect("argument key was added"),
                    next,
                ));
                index += 1;
            }
        }
        index += 1;
    }
    (!meaningful.is_empty()).then(|| meaningful.join(" "))
}

fn normalize_argument_value(key: &str, value: &str) -> String {
    let value = value.trim_matches('"');
    if key == "--user-data-dir" {
        normalize_path(value)
    } else {
        value.to_lowercase()
    }
}

fn tokenize_quoted_arguments(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut quoted = false;
    for character in value.chars() {
        match character {
            '"' => quoted = !quoted,
            character if character.is_whitespace() && !quoted => {
                if !token.is_empty() {
                    tokens.push(std::mem::take(&mut token));
                }
            }
            character => token.push(character),
        }
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}

fn registry_install_contains_exe(left: &AppCandidate, right: &AppCandidate) -> bool {
    let pairs = [(left, right), (right, left)];
    pairs.iter().any(|(registry, executable)| {
        registry.app.source_kind == SourceKind::Registry
            && executable.app.path.to_lowercase().ends_with(".exe")
            && registry
                .identity
                .install_root
                .as_ref()
                // Containment must stop at a component boundary: with a raw `starts_with` an
                // install root of `C:\Prog` "contained" `C:\Program Files\other.exe`, and this
                // evidence requires neither a matching name nor a matching publisher, so
                // nothing else in the scoring would have vetoed the merge.
                .is_some_and(|root| super::path_is_within(&executable.identity.path, root))
    })
}

fn one_is_aumid(left: &AppCandidate, right: &AppCandidate) -> bool {
    left.app.launch_kind == LaunchKind::AppUserModelId
        || right.app.launch_kind == LaunchKind::AppUserModelId
}

fn publishers_conflict(left: &AppInfo, right: &AppInfo) -> bool {
    let left = normalized_publisher(left.publisher.as_deref());
    let right = normalized_publisher(right.publisher.as_deref());
    !left.is_empty() && !right.is_empty() && left != right
}

/// Legal-form suffixes stripped so "Foo" and "Foo Inc." compare equal. Matched as whole tokens,
/// never as substrings: the old `.replace("inc", "")` mangled real publisher names — "Vincent
/// Labs" became "vt labs", "Incredible" became "redible", and a publisher of literally "Inc."
/// collapsed to the empty string, which silently disabled `publishers_conflict` (the empty
/// publisher matches everything). Token matching keeps the intended stripping while leaving any
/// name that merely contains these letters intact.
const PUBLISHER_LEGAL_SUFFIXES: &[&str] = &[
    "corporation",
    "incorporated",
    "limited",
    "company",
    "corp",
    "inc",
    "llc",
];

fn normalized_publisher(value: Option<&str>) -> String {
    value
        .unwrap_or_default()
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty() && !PUBLISHER_LEGAL_SUFFIXES.contains(token))
        .collect()
}

fn launch_target(app: &AppInfo) -> Option<&str> {
    let target = app.resolved_path.as_deref()?;
    // A generic interpreter host (cmd.exe, powershell.exe, python.exe, …) is not an identifying
    // target. Distinct tools that merely share an interpreter — a Node.js prompt and a VS
    // Developer Command Prompt, both `cmd.exe /k <different>.bat` — would otherwise resolve to
    // the same target, collide on one canonical identity, and over-merge into a single card.
    // Fall back to the shortcut's own path for those. Self-contained targets (`.msc`, `.cpl`,
    // real application exes) are unaffected.
    (!is_generic_interpreter_host(target)).then_some(target)
}

/// Interpreter/host executables whose behaviour is defined by their arguments, so the host path
/// alone does not identify the tool. Mirrors `start_apps::is_generic_host_target` (see FRAG-1 in
/// the audit: these lists should eventually share one source).
fn is_generic_interpreter_host(path: &str) -> bool {
    const HOSTS: &[&str] = &[
        "cmd.exe",
        "powershell.exe",
        "pwsh.exe",
        "wscript.exe",
        "cscript.exe",
        "rundll32.exe",
        "mshta.exe",
        "conhost.exe",
        "control.exe",
        "explorer.exe",
        "mmc.exe",
        "python.exe",
        "pythonw.exe",
        "py.exe",
        "node.exe",
        "java.exe",
        "javaw.exe",
        "mysql.exe",
        "wsl.exe",
        "bash.exe",
        "sh.exe",
    ];
    let file = Path::new(path.trim().trim_matches('"'))
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(path)
        .to_ascii_lowercase();
    HOSTS.contains(&file.as_str())
}

fn steam_app_id(app: &AppInfo) -> Option<&str> {
    if app.source_kind != SourceKind::Steam {
        return None;
    }
    app.path.strip_prefix("steam://rungameid/")
}

pub(super) fn canonical_id(app: &AppInfo) -> String {
    let identity = CandidateIdentity::from_app(app);
    if let Some(app_id) = identity.steam_app_id {
        return format!("steam:{}", app_id.to_lowercase());
    }
    if let Some(aumid) = identity.aumid {
        return format!("aumid:{aumid}");
    }
    if let Some(target) = identity.launch_target {
        return canonical_target_id(&target, app.launch_arguments.as_deref());
    }
    if let Some(registry_product) = identity.registry_product {
        if !registry_product.trim_matches('|').is_empty() {
            return format!("registry:{registry_product}");
        }
    }
    if let Some(portable_product) = identity.portable_product {
        if !portable_product.trim_matches('|').is_empty() {
            return format!("portable:{portable_product}");
        }
    }
    format!("path:{}", identity.path)
}

fn resolved_canonical_id(resolved: &ResolvedApp) -> String {
    let identities = resolved
        .candidates
        .iter()
        .map(|candidate| &candidate.identity)
        .collect::<Vec<_>>();
    if let Some(app_id) = identities
        .iter()
        .find_map(|identity| identity.steam_app_id.as_ref())
    {
        return format!("steam:{}", app_id.to_lowercase());
    }
    if let Some(aumid) = identities
        .iter()
        .find_map(|identity| identity.aumid.as_ref())
    {
        return format!("aumid:{aumid}");
    }
    if let Some(target) = identities
        .iter()
        .find_map(|identity| identity.launch_target.as_ref())
    {
        return canonical_target_id(target, resolved.app.launch_arguments.as_deref());
    }
    if let Some(registry_product) = identities
        .iter()
        .find_map(|identity| identity.registry_product.as_ref())
        .filter(|value| !value.trim_matches('|').is_empty())
    {
        return format!("registry:{registry_product}");
    }
    if let Some(portable_product) = identities
        .iter()
        .find_map(|identity| identity.portable_product.as_ref())
        .filter(|value| !value.trim_matches('|').is_empty())
    {
        return format!("portable:{portable_product}");
    }
    canonical_id(&resolved.app)
}

fn canonical_target_id(target: &str, arguments: Option<&str>) -> String {
    match meaningful_launch_arguments(arguments) {
        Some(mode) => format!("target:{target}|mode:{:x}", Sha256::digest(mode.as_bytes())),
        None => format!("target:{target}"),
    }
}

pub(super) fn normalize_path(value: &str) -> String {
    let expanded = expand_windows_env(value.trim().trim_matches('"'));
    let separated = expanded.replace('/', "\\");
    let mut normalized = std::path::PathBuf::new();
    for component in Path::new(&separated).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
        .to_string_lossy()
        .trim_end_matches('\\')
        .to_lowercase()
}

fn expand_windows_env(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        result.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            result.push_str(&rest[start..]);
            return result;
        };
        let name = &after[..end];
        if let Ok(replacement) = std::env::var(name) {
            result.push_str(&replacement);
        } else {
            result.push('%');
            result.push_str(name);
            result.push('%');
        }
        rest = &after[end + 1..];
    }
    result.push_str(rest);
    result
}

pub(super) fn preference_identity(app: &AppInfo) -> String {
    let raw = if let Some(app_id) = steam_app_id(app) {
        format!("steam:{}", app_id.to_lowercase())
    } else if app.launch_kind == LaunchKind::AppUserModelId {
        format!("aumid:{}", app.path.trim().to_lowercase())
    } else {
        let product = app
            .product_name
            .as_deref()
            .map(normalized_product_family)
            .filter(|value| !value.is_empty());
        let publisher = normalized_publisher(app.publisher.as_deref());
        let install_root = app.install_location.as_deref().map(normalize_path);
        if let (Some(product), Some(root)) = (product, install_root.filter(|root| !root.is_empty()))
        {
            if !publisher.is_empty() {
                format!("product:{publisher}|{product}|{root}")
            } else if app.source_kind == SourceKind::Portable {
                format!("portable:{product}|{root}")
            } else if let Some(target) = preference_target(app) {
                format!("target:{target}")
            } else {
                format!("path:{}", normalize_path(&app.path))
            }
        } else if let Some(target) = preference_target(app) {
            format!("target:{target}")
        } else {
            format!("path:{}", normalize_path(&app.path))
        }
    };
    format!("identity:{:x}", Sha256::digest(raw.as_bytes()))
}

fn card_preference_identity(app: &AppInfo, product_identity: &str) -> String {
    let launch_role = match (app.source_kind, app.launch_kind) {
        (SourceKind::Steam, _) => "steam".to_owned(),
        (_, LaunchKind::AppUserModelId) => "app_user_model_id".to_owned(),
        _ => {
            let target = app.resolved_path.as_deref().unwrap_or(&app.path);
            Path::new(target.trim().trim_matches('"'))
                .file_stem()
                .and_then(|value| value.to_str())
                .map(str::to_lowercase)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| normalized_product_family(&app.name))
        }
    };
    let launch_mode =
        meaningful_launch_arguments(app.launch_arguments.as_deref()).unwrap_or_default();
    let raw = format!("{product_identity}|role:{launch_role}|mode:{launch_mode}");
    format!("preference:{:x}", Sha256::digest(raw.as_bytes()))
}

fn preference_target(app: &AppInfo) -> Option<String> {
    let target = normalize_path(launch_target(app)?);
    Some(
        match meaningful_launch_arguments(app.launch_arguments.as_deref()) {
            Some(mode) => format!("{target}|mode:{mode}"),
            None => target,
        },
    )
}

fn launcher_product_family(name: &str) -> &str {
    name.strip_suffix(" launcher").unwrap_or(name).trim()
}

/// A 32-bit build of a tool, recognized so a merged x86/x64 pair can surface the 64-bit one.
fn is_32bit_variant(app: &AppInfo) -> bool {
    let name = app.name.to_lowercase();
    let arch_token = name
        .split(|character: char| !character.is_alphanumeric())
        .any(|token| matches!(token, "x86" | "wow" | "wow64"));
    let paths_are_32bit = [
        app.path.as_str(),
        app.resolved_path.as_deref().unwrap_or(""),
    ]
    .iter()
    .any(|value| {
        let lower = value.to_lowercase();
        lower.contains(r"\syswow64\") || lower.contains(r"\program files (x86)\")
    });
    arch_token || name.contains("32-bit") || paths_are_32bit
}

fn merge_app(left: AppInfo, right: AppInfo) -> AppInfo {
    let scores_tie = candidate_score(&right) == candidate_score(&left);
    let prefer_right = candidate_score(&right) > candidate_score(&left)
        || (scores_tie
            && left.source_kind == SourceKind::Portable
            && right.source_kind == SourceKind::Portable
            && version_key(right.version.as_deref()) > version_key(left.version.as_deref()))
        // On a tie, keep the 64-bit build when the two differ only by architecture.
        || (scores_tie && is_32bit_variant(&left) && !is_32bit_variant(&right));
    let (mut primary, secondary) = if prefer_right {
        (right, left)
    } else {
        (left, right)
    };
    if primary.description.is_none() {
        primary.description = secondary.description;
    }
    if primary.version.is_none() {
        primary.version = secondary.version;
    }
    if (primary.publisher.is_none()
        || primary
            .publisher
            .as_deref()
            .is_some_and(|value| value.starts_with("CN=")))
        && secondary
            .publisher
            .as_deref()
            .is_some_and(|value| !value.starts_with("CN="))
    {
        primary.publisher = secondary.publisher;
    }
    if primary.product_name.is_none() {
        primary.product_name = secondary.product_name;
    }
    if primary.original_filename.is_none() {
        primary.original_filename = secondary.original_filename;
    }
    if primary.install_location.is_none() {
        primary.install_location = secondary.install_location;
    }
    if primary.icon_base64.is_none() {
        primary.icon_base64 = secondary.icon_base64;
    }
    if primary.uninstall.is_none() {
        primary.uninstall = secondary.uninstall;
    }
    if primary.resolved_path.is_none() {
        primary.resolved_path = secondary.resolved_path;
    }
    if primary.shortcut_icon_path.is_none() {
        primary.shortcut_icon_path = secondary.shortcut_icon_path;
    }
    if primary.launch_arguments.is_none() {
        primary.launch_arguments = secondary.launch_arguments;
    }
    if visibility_rank(secondary.visibility_class) > visibility_rank(primary.visibility_class) {
        primary.visibility_class = secondary.visibility_class;
    }
    primary.visibility_score = primary.visibility_score.max(secondary.visibility_score);
    for reason in secondary.visibility_reasons {
        if !primary.visibility_reasons.contains(&reason) {
            primary.visibility_reasons.push(reason);
        }
    }
    // A command environment or product component stays auxiliary through a merge. Otherwise an
    // AUMID sibling (Primary by the launch-kind fast-path, since it carries no arguments to reveal
    // itself) promoted the whole card back to Primary — this is why a merged IDLE / Python prompt
    // reappeared in the main catalog.
    use crate::catalog::VisibilityReason::{CommandEnvironment, ProductComponent};
    if primary.visibility_class == crate::catalog::VisibilityClass::Primary
        && primary
            .visibility_reasons
            .iter()
            .any(|reason| matches!(reason, CommandEnvironment | ProductComponent))
    {
        primary.visibility_class = crate::catalog::VisibilityClass::Auxiliary;
    }
    primary.can_uninstall |= secondary.can_uninstall || primary.uninstall.is_some();
    primary
}

fn visibility_rank(class: crate::catalog::VisibilityClass) -> u8 {
    match class {
        crate::catalog::VisibilityClass::Rejected => 0,
        crate::catalog::VisibilityClass::Auxiliary => 1,
        crate::catalog::VisibilityClass::Primary => 2,
    }
}

fn version_key(version: Option<&str>) -> Vec<u64> {
    version
        .unwrap_or_default()
        .split(|character: char| !character.is_ascii_digit())
        .filter_map(|segment| segment.parse().ok())
        .collect()
}

pub(super) fn normalize_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Architecture markers are noise for identity: the 64-bit and 32-bit build of one tool are a
/// single application to a launcher. Stripped as whole tokens (surrounding parentheses ignored)
/// from anywhere in the name, so `Windows PowerShell ISE (x86)`, `x64 Native Tools …` and
/// `Foo x64` all reduce to the same family as their 64-bit sibling.
fn strip_architecture_markers(name: &str) -> String {
    const MARKERS: &[&str] = &[
        "x86", "x64", "x86_x64", "x64_x86", "amd64", "arm64", "ia64", "win32", "win64", "32bit",
        "64bit", "32-bit", "64-bit", "wow", "wow64",
    ];
    name.split_whitespace()
        .filter(|token| {
            let cleaned = token.trim_matches(|character| character == '(' || character == ')');
            !MARKERS.contains(&cleaned)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn normalized_product_family(name: &str) -> String {
    let value = strip_architecture_markers(&normalize_name(name));
    if let Some((family, suffix)) = value.split_once(" - ") {
        let generic_suffix = ["proxy utility", "desktop app", "application"]
            .iter()
            .any(|marker| suffix.starts_with(marker));
        let has_version = suffix.chars().any(|character| character.is_ascii_digit());
        if generic_suffix && has_version {
            return family.trim().to_string();
        }
    }
    let mut family = version_family(&value).trim().to_string();
    if family.ends_with(" version") {
        family.truncate(family.len() - " version".len());
    }
    if let Some(rest) = family.strip_prefix("mozilla firefox") {
        let rest = rest.trim();
        if rest.is_empty() || rest.starts_with('(') {
            family = "firefox".into();
        }
    }
    canonical_windows_tool_family(&family)
}

fn canonical_windows_tool_family(family: &str) -> String {
    const ALIASES: [(&str, &[&str]); 10] = [
        ("task manager", &["task manager", "диспетчер задач"]),
        ("control panel", &["control panel", "панель управления"]),
        ("registry editor", &["registry editor", "редактор реестра"]),
        ("device manager", &["device manager", "диспетчер устройств"]),
        ("services", &["services", "службы"]),
        ("event viewer", &["event viewer", "просмотр событий"]),
        (
            "computer management",
            &["computer management", "управление компьютером"],
        ),
        (
            "disk management",
            &["disk management", "управление дисками"],
        ),
        (
            "system information",
            &["system information", "сведения о системе"],
        ),
        ("command prompt", &["command prompt", "командная строка"]),
    ];
    ALIASES
        .iter()
        .find_map(|(canonical, aliases)| aliases.contains(&family).then_some(*canonical))
        .unwrap_or(family)
        .to_string()
}

fn version_family(name: &str) -> &str {
    let Some((family, suffix)) = name.rsplit_once(' ') else {
        return name;
    };
    if !suffix.starts_with(|character: char| character.is_ascii_digit()) {
        return name;
    }
    let numeric_segments = suffix
        .split(|character: char| !character.is_ascii_digit())
        .filter(|segment| !segment.is_empty())
        .count();
    if numeric_segments >= 2 {
        family
    } else {
        name
    }
}

/// Writing system of a display name — only the distinction the locale rule acts on. Any Cyrillic
/// letter marks a localized name; otherwise a Latin letter marks an English/Latin name.
fn name_script(name: &str) -> NameScript {
    let mut has_latin = false;
    for character in name.chars() {
        if ('\u{0400}'..='\u{052F}').contains(&character) {
            return NameScript::Cyrillic;
        }
        if character.is_ascii_alphabetic() {
            has_latin = true;
        }
    }
    if has_latin {
        NameScript::Latin
    } else {
        NameScript::Other
    }
}

/// Chooses the display name for a merged card from all of its candidate names, given the OS UI
/// script. `primary` is the highest-scored source's name (kept when it already matches). Otherwise
/// a candidate in the OS script wins; failing that, any Latin (English) name — so a non-Cyrillic
/// user never ends up reading a Cyrillic card when a Latin alternative exists. Purely a naming
/// choice: the launching source, icon, and target are unchanged.
fn choose_display_name<'a>(
    primary: &str,
    candidates: impl Iterator<Item = &'a str>,
    os_script: NameScript,
) -> String {
    if name_script(primary) == os_script {
        return primary.to_string();
    }
    let names = candidates.collect::<Vec<_>>();
    if let Some(matched) = names.iter().find(|name| name_script(name) == os_script) {
        return (*matched).to_string();
    }
    if name_script(primary) == NameScript::Latin {
        return primary.to_string();
    }
    if let Some(latin) = names
        .iter()
        .find(|name| name_script(name) == NameScript::Latin)
    {
        return (*latin).to_string();
    }
    primary.to_string()
}

fn candidate_score(app: &AppInfo) -> u8 {
    if is_helper_candidate(app) {
        return 1;
    }
    if app.source_kind == SourceKind::Steam {
        return 5;
    }
    // A localized Start-App for a built-in Windows tool (Event Viewer / Просмотр событий,
    // etc.) should win over its English Start-Menu shortcut so the merged card keeps the
    // OS-language name and the working shell icon. Scoped to system targets only, so normal
    // app merges (registry/shortcut/portable) are unaffected.
    if app.launch_kind == LaunchKind::AppUserModelId
        && app.source_kind == SourceKind::StartApps
        && (is_system_tool_target(app) || system_tool_alias(app).is_some())
    {
        return 6;
    }
    match Path::new(&app.path)
        .extension()
        .and_then(|value| value.to_str())
    {
        Some(extension) if extension.eq_ignore_ascii_case("lnk") => return 4,
        Some(extension) if extension.eq_ignore_ascii_case("exe") => return 3,
        _ => {}
    }
    if app.launch_kind == LaunchKind::AppUserModelId {
        2
    } else {
        0
    }
}

/// Curated equivalence for built-in Windows shell items whose localized Start-App and English
/// shortcut resolve to different, non-comparable targets (a shell CLSID vs a PIDL-only shortcut,
/// or control.exe with an applet name). Returns a shared token so the pair collapses into one
/// card. Language-independent: keyed on stable AUMIDs and applet names, not display text.
fn system_tool_alias(app: &AppInfo) -> Option<&'static str> {
    if app.launch_kind == LaunchKind::AppUserModelId {
        match app.path.trim().to_lowercase().as_str() {
            "microsoft.windows.explorer" => return Some("windows:explorer"),
            "microsoft.windows.administrativetools" => return Some("windows:admintools"),
            "microsoft.windows.controlpanel" => return Some("windows:controlpanel"),
            "microsoft.windows.remotedesktop" => return Some("windows:remotedesktop"),
            "microsoft.windows.shell.rundialog" => return Some("windows:run"),
            _ => {}
        }
    }
    let target = app
        .resolved_path
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    let args = app
        .launch_arguments
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    // "Administrative Tools" launches control.exe /name Microsoft.AdministrativeTools.
    if target.ends_with("control.exe") && args.contains("microsoft.administrativetools") {
        return Some("windows:admintools");
    }
    // "File Explorer" ships as a PIDL-only shortcut (no readable target); its .lnk file name
    // is English on every locale, so it is a safe key for this fixed shell item.
    if target.is_empty() && normalize_name(&app.name) == "file explorer" {
        return Some("windows:explorer");
    }
    // The Run dialog also ships as a PIDL-only "Run" shortcut with no readable target, matching the
    // localized `Microsoft.Windows.Shell.RunDialog` Start-App above.
    if target.is_empty() && normalize_name(&app.name) == "run" {
        return Some("windows:run");
    }
    None
}

/// True when the app's resolved launch target is a built-in Windows tool: a `.msc`/`.cpl`
/// snap-in, a binary under the Windows system directories, or a shell CLSID target
/// (`::{…}`, e.g. Control Panel). Used to scope localized-name preference to system tools.
fn is_system_tool_target(app: &AppInfo) -> bool {
    let Some(target) = app.resolved_path.as_deref() else {
        return false;
    };
    let normalized = normalize_path(target);
    normalized.starts_with("::{")
        || normalized.ends_with(".msc")
        || normalized.ends_with(".cpl")
        || normalized.contains("\\windows\\system32\\")
        || normalized.contains("\\windows\\syswow64\\")
}

fn category_rank(category: AppCategory) -> u8 {
    match category {
        AppCategory::Games => 0,
        AppCategory::Ai => 1,
        AppCategory::Editors => 2,
        AppCategory::Development => 3,
        AppCategory::Productivity => 4,
        AppCategory::Browsers => 5,
        AppCategory::Media => 6,
        AppCategory::Communication => 7,
        AppCategory::FileCloud => 8,
        AppCategory::Security => 9,
        AppCategory::Utilities => 10,
        AppCategory::System => 11,
        AppCategory::WindowsFeatures => 12,
        AppCategory::Other => 13,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, path: &str) -> AppInfo {
        AppInfo {
            id: String::new(),
            name: name.into(),
            path: path.into(),
            icon_base64: None,
            category: AppCategory::Other,
            launch_kind: LaunchKind::Executable,
            source_kind: SourceKind::Registry,
            description: None,
            version: None,
            publisher: None,
            product_name: None,
            original_filename: None,
            install_location: None,
            can_uninstall: false,
            uninstall: None,
            resolved_path: None,
            shortcut_icon_path: None,
            launch_arguments: None,
            canonical_identity: None,
            preference_identity: None,
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
        }
    }

    fn resolve(apps: Vec<AppInfo>) -> Vec<AppInfo> {
        deduplicate(apps, |_app| AppCategory::Other, NameScript::Latin)
    }

    #[test]
    fn name_script_detects_writing_system() {
        assert_eq!(name_script("Kaspersky"), NameScript::Latin);
        assert_eq!(name_script("7-Zip"), NameScript::Latin);
        assert_eq!(name_script("Касперский"), NameScript::Cyrillic);
        assert_eq!(name_script("Проводник"), NameScript::Cyrillic);
        assert_eq!(name_script("日本語"), NameScript::Other);
    }

    #[test]
    fn choose_display_name_prefers_os_script_then_latin_fallback() {
        // English machine: a Latin name wins over a Cyrillic primary.
        assert_eq!(
            choose_display_name(
                "Касперский",
                ["Касперский", "Kaspersky"].into_iter(),
                NameScript::Latin,
            ),
            "Kaspersky"
        );
        // Russian machine: the localized name is kept.
        assert_eq!(
            choose_display_name(
                "Kaspersky",
                ["Kaspersky", "Касперский"].into_iter(),
                NameScript::Cyrillic,
            ),
            "Касперский"
        );
        // English machine but only a Cyrillic name exists: nothing better, keep it.
        assert_eq!(
            choose_display_name("Проводник", ["Проводник"].into_iter(), NameScript::Latin),
            "Проводник"
        );
    }

    // The merged card launches via the higher-scored source but reads in the user's OS language:
    // a Russian shortcut plus an English registry entry for one product show "Kaspersky" on an
    // English machine and "Касперский" on a Russian one.
    #[test]
    fn merged_display_name_follows_the_os_language() {
        let candidates = || {
            let mut shortcut = app("Касперский", r"C:\Menu\Касперский.lnk");
            shortcut.launch_kind = LaunchKind::Shortcut;
            shortcut.source_kind = SourceKind::StartMenu;
            shortcut.resolved_path = Some(r"C:\Program Files\Kaspersky\avp.exe".into());
            let mut registry = app("Kaspersky", r"C:\Program Files\Kaspersky\avp.exe");
            registry.can_uninstall = true;
            vec![shortcut, registry]
        };

        let english = deduplicate(candidates(), |_app| AppCategory::Other, NameScript::Latin);
        assert_eq!(english.len(), 1, "the two entries must merge into one card");
        assert_eq!(english[0].name, "Kaspersky");

        let russian = deduplicate(
            candidates(),
            |_app| AppCategory::Other,
            NameScript::Cyrillic,
        );
        assert_eq!(russian.len(), 1);
        assert_eq!(russian[0].name, "Касперский");
    }

    // Legal-form suffixes are stripped as whole tokens. The old substring `.replace` mangled real
    // names whose letters merely contained a suffix, and — worse — emptied a publisher that was
    // only a suffix, which disables `publishers_conflict` and lets unrelated apps merge.
    #[test]
    fn normalized_publisher_strips_only_whole_suffix_tokens() {
        assert_eq!(
            normalized_publisher(Some("Microsoft Corporation")),
            "microsoft"
        );
        assert_eq!(normalized_publisher(Some("Valve Inc.")), "valve");
        assert_eq!(normalized_publisher(Some("Acme, LLC")), "acme");

        // Real names that merely contain the letters of a suffix must survive intact.
        assert_eq!(normalized_publisher(Some("Vincent Labs")), "vincentlabs");
        assert_eq!(
            normalized_publisher(Some("Incredible Software")),
            "incrediblesoftware"
        );
        assert_eq!(normalized_publisher(Some("Sinclair")), "sinclair");
    }

    // A publisher that is nothing but a legal suffix carries no identifying information, so it
    // normalizes to empty and — as before — cannot establish a conflict on its own.
    #[test]
    fn a_bare_legal_suffix_publisher_normalizes_to_empty() {
        assert_eq!(normalized_publisher(Some("Inc.")), "");
        assert_eq!(normalized_publisher(None), "");
    }

    // Regression for the substring bug: two distinct real publishers used to collapse to the same
    // mangled string ("Vincent" and "Vt" both became "vt") and stopped conflicting, which allowed
    // unrelated applications to merge. Token stripping keeps them distinct.
    #[test]
    fn distinct_publishers_no_longer_collide_after_stripping() {
        let mut vincent = app("Tool", r"C:\A\tool.exe");
        vincent.publisher = Some("Vincent Labs Inc".into());
        let mut vt = app("Tool", r"C:\B\tool.exe");
        vt.publisher = Some("Vt Corp".into());

        assert!(publishers_conflict(&vincent, &vt));
    }

    // Architecture is not part of a product's identity: the x86 and x64 builds are one app.
    #[test]
    fn architecture_variants_share_a_product_family() {
        assert_eq!(
            normalized_product_family("Windows PowerShell ISE (x86)"),
            normalized_product_family("Windows PowerShell ISE"),
        );
        assert_eq!(
            normalized_product_family("x64 Native Tools Command Prompt for VS 2019"),
            normalized_product_family("x86 Native Tools Command Prompt for VS 2019"),
        );
        // But a genuinely different tool sharing the vendor suffix stays distinct: "Native Tools"
        // and "Cross Tools" are not the same prompt.
        assert_ne!(
            normalized_product_family("x64 Native Tools Command Prompt for VS 2019"),
            normalized_product_family("x64_x86 Cross Tools Command Prompt for VS 2019"),
        );
    }

    fn aumid_ise(name: &str, guid: &str, resolved: &str) -> AppInfo {
        let mut app = app(
            name,
            &format!(r"{guid}\WindowsPowerShell\v1.0\PowerShell_ISE.exe"),
        );
        app.launch_kind = LaunchKind::AppUserModelId;
        app.source_kind = SourceKind::StartApps;
        app.resolved_path = Some(resolved.into());
        app
    }

    // The user's two "Windows PowerShell ISE" cards are the 64-bit (System32) and 32-bit
    // (SysWOW64) builds. They must collapse into one card, keeping the 64-bit build.
    #[test]
    fn architecture_variants_merge_and_keep_the_64bit_build() {
        let x86 = aumid_ise(
            "Windows PowerShell ISE (x86)",
            "{D65231B0-B2F1-4857-A4CE-A8E7C6EA7D27}",
            r"C:\Windows\syswow64\WindowsPowerShell\v1.0\PowerShell_ISE.exe",
        );
        let x64 = aumid_ise(
            "Windows PowerShell ISE",
            "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}",
            r"C:\Windows\system32\WindowsPowerShell\v1.0\PowerShell_ISE.exe",
        );

        // 32-bit listed first, so the merge has to actively prefer the 64-bit build.
        let resolved = resolve(vec![x86, x64]);

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "Windows PowerShell ISE");
    }

    fn cmd_shortcut(name: &str, lnk: &str, bat: &str) -> AppInfo {
        let mut app = app(name, lnk);
        app.launch_kind = LaunchKind::Shortcut;
        app.source_kind = SourceKind::StartMenu;
        app.resolved_path = Some(r"C:\Windows\System32\cmd.exe".into());
        app.launch_arguments = Some(format!(r#"/k "{bat}""#));
        app
    }

    // Node.js prompt and a VS command prompt are both `cmd.exe /k <different>.bat`. The shared
    // interpreter must not give them one identity (favorites/hidden would bleed) nor merge them.
    #[test]
    fn generic_host_shortcuts_neither_collide_nor_over_merge() {
        let node = cmd_shortcut(
            "Node.js command prompt",
            r"C:\Menu\Node.js\Node.js command prompt.lnk",
            r"C:\Program Files\nodejs\nodevars.bat",
        );
        let vs = cmd_shortcut(
            "x86_x64 Cross Tools Command Prompt for VS 2019",
            r"C:\Menu\VS\Cross Tools.lnk",
            r"C:\BuildTools\VC\Auxiliary\Build\vcvarsx86_amd64.bat",
        );

        assert_ne!(preference_identity(&node), preference_identity(&vs));
        assert_eq!(resolve(vec![node, vs]).len(), 2);
    }

    fn launch_card(name: &str, target: &str, arguments: Option<&str>) -> AppInfo {
        let mut candidate = app(name, &format!(r"C:\Menu\{name}.lnk"));
        candidate.launch_kind = LaunchKind::Shortcut;
        candidate.source_kind = SourceKind::StartMenu;
        candidate.resolved_path = Some(target.into());
        candidate.launch_arguments = arguments.map(str::to_owned);
        candidate
    }

    #[test]
    fn separate_launch_cards_in_one_product_get_distinct_preference_identities() {
        let cases = [
            (
                launch_card("Python 3.14", r"C:\Python\python.exe", None),
                launch_card(
                    "IDLE",
                    r"C:\Python\pythonw.exe",
                    Some(r#""C:\Python\Lib\idlelib\idle.pyw""#),
                ),
            ),
            (
                launch_card("7-Zip", r"D:\Tools\7-Zip\7z.exe", None),
                launch_card("7-Zip File Manager", r"D:\Tools\7-Zip\7zFM.exe", None),
            ),
            (
                launch_card("chrome", r"C:\Chromium\chrome.exe", None),
                launch_card(
                    "chrome pwa launcher",
                    r"C:\Chromium\chrome_pwa_launcher.exe",
                    None,
                ),
            ),
            (
                launch_card(
                    "LibreOffice Safe Mode",
                    r"C:\Program Files\LibreOffice\program\soffice.exe",
                    Some("--safe-mode"),
                ),
                launch_card(
                    "LibreOffice Help Pack",
                    r"C:\Program Files\LibreOffice\program\gengal.exe",
                    None,
                ),
            ),
        ];

        for (left, right) in cases {
            assert_ne!(
                card_preference_identity(&left, "identity:shared-product"),
                card_preference_identity(&right, "identity:shared-product"),
                "{} and {} must not share durable user preferences",
                left.name,
                right.name,
            );
        }
    }

    #[test]
    fn unknown_launch_modes_still_get_distinct_preference_identities() {
        let ordinary = launch_card(
            "LibreOffice",
            r"C:\Program Files\LibreOffice\program\soffice.exe",
            None,
        );
        let safe_mode = launch_card(
            "LibreOffice Safe Mode",
            r"C:\Program Files\LibreOffice\program\soffice.exe",
            Some("--safe-mode"),
        );

        assert_ne!(
            card_preference_identity(&ordinary, "identity:libreoffice"),
            card_preference_identity(&safe_mode, "identity:libreoffice"),
        );
        let resolved = resolve(vec![ordinary, safe_mode]);
        assert_eq!(resolved.len(), 2);
        assert_ne!(resolved[0].id, resolved[1].id);
    }

    #[test]
    fn the_same_launch_role_keeps_its_preference_identity_across_sources() {
        let mut registry = app("Editor", r"C:\Editor\editor.exe");
        registry.product_name = Some("Editor".into());
        let shortcut = launch_card("Editor", r"C:\Editor\editor.exe", None);

        assert_eq!(
            card_preference_identity(&registry, "identity:editor-product"),
            card_preference_identity(&shortcut, "identity:editor-product"),
        );
    }

    #[test]
    fn visual_studio_command_profiles_keep_distinct_batch_environments() {
        let profiles = vec![
            cmd_shortcut(
                "x64 Native Tools Command Prompt for VS 2019",
                r"C:\Menu\VS\x64 Native Tools.lnk",
                r"C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat",
            ),
            cmd_shortcut(
                "x86 Native Tools Command Prompt for VS 2019",
                r"C:\Menu\VS\x86 Native Tools.lnk",
                r"C:\BuildTools\VC\Auxiliary\Build\vcvars32.bat",
            ),
            cmd_shortcut(
                "x64_x86 Cross Tools Command Prompt for VS 2019",
                r"C:\Menu\VS\x64_x86 Cross Tools.lnk",
                r"C:\BuildTools\VC\Auxiliary\Build\vcvarsamd64_x86.bat",
            ),
            cmd_shortcut(
                "x86_x64 Cross Tools Command Prompt for VS 2019",
                r"C:\Menu\VS\x86_x64 Cross Tools.lnk",
                r"C:\BuildTools\VC\Auxiliary\Build\vcvarsx86_amd64.bat",
            ),
        ];

        assert_eq!(resolve(profiles).len(), 4);
    }

    #[test]
    fn duplicate_sources_for_one_command_profile_still_merge() {
        let shortcut = cmd_shortcut(
            "x64 Native Tools Command Prompt for VS 2019",
            r"C:\Menu\VS\x64 Native Tools.lnk",
            r"C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat",
        );
        let mut start_app = app(
            "x64 Native Tools Command Prompt for VS 2019",
            "Microsoft.AutoGenerated.{FF70D809-A022-5972-850F-19E0AA8C07C2}",
        );
        start_app.launch_kind = LaunchKind::AppUserModelId;
        start_app.source_kind = SourceKind::StartApps;

        let resolved = resolve(vec![start_app, shortcut]);

        assert_eq!(resolved.len(), 1);
        assert_eq!(
            resolved[0].launch_arguments.as_deref(),
            Some(r#"/k "C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat""#)
        );
    }

    // Real corpus: a Start-Menu shortcut and its Start-App (AUMID) that resolve to the same
    // installed exe must be one card. These stayed as two on the user's machine.
    #[test]
    fn start_menu_shortcut_and_aumid_to_same_exe_merge() {
        let mut shortcut = app(
            "CPU-Z MSI",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\CPUID\CPU-Z MSI\CPU-Z MSI.lnk",
        );
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Program Files\CPUID\CPU-Z MSI\cpuz.exe".into());
        shortcut.install_location = Some(r"C:\Program Files\CPUID\CPU-Z MSI\".into());
        shortcut.publisher = Some("CPUID".into());

        let mut aumid = app(
            "CPU-Z MSI",
            r"{6D809377-6AF0-444B-8957-A3773F02200E}\CPUID\CPU-Z MSI\cpuz.exe",
        );
        aumid.launch_kind = LaunchKind::AppUserModelId;
        aumid.source_kind = SourceKind::StartApps;
        aumid.resolved_path = Some(r"C:\Program Files\CPUID\CPU-Z MSI\cpuz.exe".into());
        aumid.install_location = Some(r"C:\Program Files\CPUID\CPU-Z MSI".into());
        aumid.publisher = Some("CPUID".into());

        assert_eq!(resolve(vec![shortcut, aumid]).len(), 1);
    }

    // Two portable copies of the same product in different folders: merge when the version is
    // identical, keep separate when it differs (a different version is a different program).
    #[test]
    fn portable_copies_merge_on_equal_version_only() {
        let make = |folder: &str, version: &str| {
            let mut app = app("HSP Victoria", &format!(r"{folder}\Victoria.exe"));
            app.source_kind = SourceKind::Portable;
            app.version = Some(version.into());
            app.publisher = Some("HDD.BY".into());
            app.install_location = Some(folder.into());
            app
        };

        let same = resolve(vec![
            make(r"D:\разный хлам\Victoria537", "5.3.7.0 Experimental SAS"),
            make(
                r"E:\Програмки\SSD&HDD\Victoria537",
                "5.3.7.0 Experimental SAS",
            ),
        ]);
        assert_eq!(same.len(), 1, "identical versions collapse");

        let different = resolve(vec![
            make(r"D:\A\Victoria537", "5.3.7.0 Experimental SAS"),
            make(r"E:\B\Victoria6", "6.0.0.0"),
        ]);
        assert_eq!(different.len(), 2, "different versions stay separate");
    }

    // An installed app (Start-Menu shortcut) and a loose portable copy on the Desktop, same
    // product and exact version, are one program — merge, keeping the installed entry, even across
    // a vendor-name variant (Mozilla Corporation vs Foundation).
    #[test]
    fn installed_and_portable_same_version_merge_keeping_installed() {
        let mut shortcut = app("Firefox", r"C:\Menu\Firefox.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Program Files\Mozilla Firefox\firefox.exe".into());
        shortcut.install_location = Some(r"C:\Program Files\Mozilla Firefox".into());
        shortcut.version = Some("153.0".into());
        shortcut.publisher = Some("Mozilla Corporation".into());
        let mut portable = app("Firefox", r"C:\Users\Maks\Desktop\Firefox.exe");
        portable.source_kind = SourceKind::Portable;
        portable.install_location = Some(r"C:\Users\Maks\Desktop".into());
        portable.version = Some("153.0".into());
        portable.publisher = Some("Mozilla Foundation".into());

        let merged = resolve(vec![portable, shortcut]);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].path, r"C:\Menu\Firefox.lnk",
            "installed entry wins"
        );
    }

    // The same-version merge is scoped to one product family: two genuinely different products
    // must never collapse just because a version string coincides.
    #[test]
    fn same_version_does_not_merge_different_products() {
        let mut alpha = app("Alpha Tool", r"D:\A\alpha.exe");
        alpha.source_kind = SourceKind::Portable;
        alpha.version = Some("1.0.0".into());
        let mut beta = app("Beta Tool", r"E:\B\beta.exe");
        beta.source_kind = SourceKind::Portable;
        beta.version = Some("1.0.0".into());

        assert_eq!(resolve(vec![alpha, beta]).len(), 2);
    }

    // Same name, different version = different applications. Two "7-Zip" registry entries at
    // different versions (same publisher, no install root to conflict) must stay two cards; the
    // version mismatch alone must block the name/publisher merge.
    #[test]
    fn same_name_different_versions_stay_separate() {
        let make = |path: &str, version: &str| {
            let mut app = app("7-Zip", path);
            app.source_kind = SourceKind::Registry;
            app.version = Some(version.into());
            app.publisher = Some("Igor Pavlov".into());
            app
        };

        let apps = resolve(vec![
            make(r"C:\Apps\A\7zFM.exe", "22.01"),
            make(r"C:\Apps\B\7zFM.exe", "24.08"),
        ]);
        assert_eq!(apps.len(), 2);
    }

    // A version on only one side (a Start-Menu shortcut carries none, its registered product does)
    // must not block the merge — the version veto needs a genuine mismatch, not a missing value.
    #[test]
    fn version_on_only_one_side_does_not_block_a_merge() {
        let mut shortcut = app("Notepad++", r"C:\Menu\Notepad++.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Program Files\Notepad++\notepad++.exe".into());
        let mut registry = app("Notepad++", r"C:\Program Files\Notepad++\notepad++.exe");
        registry.source_kind = SourceKind::Registry;
        registry.version = Some("8.6".into());

        assert_eq!(resolve(vec![shortcut, registry]).len(), 1);
    }

    // An identity match wins over a version mismatch: the same executable reached two ways, with
    // disagreeing metadata versions, is still one application.
    #[test]
    fn same_executable_merges_despite_a_version_mismatch() {
        let mut registry = app("App", r"C:\Program Files\App\app.exe");
        registry.source_kind = SourceKind::Registry;
        registry.version = Some("1.0".into());
        let mut shortcut = app("App", r"C:\Menu\App.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Program Files\App\app.exe".into());
        shortcut.version = Some("1.1".into());

        assert_eq!(resolve(vec![registry, shortcut]).len(), 1);
    }

    // A launcher and its game executable live in one install tree (nested install roots) and carry
    // different versions — the launcher's version vs the game's patch. Nested-root evidence (>= 75)
    // overrides the version veto, so World of Warcraft stays one card, not two.
    #[test]
    fn launcher_and_game_in_one_install_tree_merge_despite_version() {
        let mut launcher = app("World of Warcraft", r"C:\Menu\World of Warcraft.lnk");
        launcher.launch_kind = LaunchKind::Shortcut;
        launcher.source_kind = SourceKind::StartMenu;
        launcher.resolved_path =
            Some(r"D:\Games\World of Warcraft\World of Warcraft Launcher.exe".into());
        launcher.install_location = Some(r"D:\Games\World of Warcraft".into());
        launcher.version = Some("1.18.10.3140".into());
        launcher.publisher = Some("Blizzard Entertainment".into());
        let mut game = app(
            "World of Warcraft",
            r"D:\Games\World of Warcraft\_retail_\Wow.exe",
        );
        game.source_kind = SourceKind::Portable;
        game.install_location = Some(r"D:\Games\World of Warcraft\_retail_".into());
        game.version = Some("12.0.7.68887".into());
        game.publisher = Some("Blizzard Entertainment".into());

        assert_eq!(resolve(vec![launcher, game]).len(), 1);
    }

    // The resolver is order-sensitive, so the same catalog scanned in different source orders used
    // to merge differently and a re-dedup reduced further — the frontend then accumulated the
    // least-merged variant across background syncs. Deduplication must be order-independent and
    // idempotent.
    #[test]
    fn deduplication_is_order_independent_and_idempotent() {
        let mut shortcut = app("CPU-Z MSI", r"C:\Menu\CPU-Z MSI.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Program Files\CPUID\CPU-Z MSI\cpuz.exe".into());
        let mut aumid = app("CPU-Z MSI", r"{6D809377-6AF0}\CPUID\cpuz.exe");
        aumid.launch_kind = LaunchKind::AppUserModelId;
        aumid.source_kind = SourceKind::StartApps;
        aumid.resolved_path = Some(r"C:\Program Files\CPUID\CPU-Z MSI\cpuz.exe".into());
        let extra = app("Notepad", r"C:\Windows\notepad.exe");

        let forward = resolve(vec![shortcut.clone(), aumid.clone(), extra.clone()]);
        let reverse = resolve(vec![extra, aumid, shortcut]);
        assert_eq!(forward.len(), 2);
        assert_eq!(
            reverse.len(),
            forward.len(),
            "order must not change the result"
        );
        assert_eq!(
            resolve(forward.clone()).len(),
            forward.len(),
            "a second pass must change nothing",
        );
    }

    // A merged command environment must stay auxiliary: an AUMID sibling is Primary only by the
    // launch-kind fast-path (it carries no arguments to reveal itself), and used to promote the
    // whole card — the merged IDLE reappeared in the main catalog.
    #[test]
    fn merged_command_environment_stays_auxiliary() {
        use crate::catalog::visibility::apply_visibility;
        use crate::catalog::VisibilityClass;

        let mut shortcut = app("IDLE (Python 3.14 64-bit)", r"C:\Menu\IDLE.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Python314\pythonw.exe".into());
        shortcut.launch_arguments = Some(r#""C:\Python314\Lib\idlelib\idle.pyw""#.into());
        apply_visibility(&mut shortcut);
        let mut aumid = app("IDLE (Python 3.14 64-bit)", "Microsoft.AutoGenerated.{ABC}");
        aumid.launch_kind = LaunchKind::AppUserModelId;
        aumid.source_kind = SourceKind::StartApps;
        // A resolvable target keeps this a genuine Primary sibling (an empty one would now be
        // demoted as a dead AutoGenerated folder entry, which is a separate case).
        aumid.resolved_path = Some(r"C:\Python314\pythonw.exe".into());
        apply_visibility(&mut aumid);

        assert_eq!(shortcut.visibility_class, VisibilityClass::Auxiliary);
        assert_eq!(aumid.visibility_class, VisibilityClass::Primary);

        let merged = resolve(vec![aumid, shortcut]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].visibility_class, VisibilityClass::Auxiliary);
    }

    // "(WOW)" is WoW64 — the 32-bit build. It merges with the "(X64)" build, keeping x64.
    #[test]
    fn wow_and_x64_variants_merge_keeping_x64() {
        let make = |name: &str, resolved: &str| {
            let mut app = app(name, &format!(r"{{GUID}}\{name}.exe"));
            app.launch_kind = LaunchKind::AppUserModelId;
            app.source_kind = SourceKind::StartApps;
            app.resolved_path = Some(resolved.into());
            app
        };
        let merged = resolve(vec![
            make(
                "Application Verifier (WOW)",
                r"C:\Windows\SysWOW64\appverif.exe",
            ),
            make(
                "Application Verifier (X64)",
                r"C:\Windows\System32\appverif.exe",
            ),
        ]);
        assert_eq!(merged.len(), 1);
        assert!(merged[0].name.to_lowercase().contains("x64"));
    }

    #[test]
    fn interpreter_hosts_are_recognized() {
        assert!(is_generic_interpreter_host(r"C:\Windows\System32\cmd.exe"));
        assert!(is_generic_interpreter_host(
            r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"
        ));
        assert!(!is_generic_interpreter_host(
            r"C:\Program Files\Mozilla Firefox\firefox.exe"
        ));
    }

    // ---------------------------------------------------------------------------
    // Differential harness.
    //
    // `resolve_apps` compares each candidate against every existing group. Making that
    // near-linear means narrowing the search with an index, and the failure mode of such a
    // change is silent: a lost merge just looks like an extra catalog entry. The reference
    // resolver below is the straightforward scan; the test asserts the production resolver
    // produces byte-identical groups on generated catalogs, so any narrowing has to prove
    // itself before it can land.
    // ---------------------------------------------------------------------------

    /// The unnarrowed "compare against every group" scan, kept as the oracle.
    fn resolve_apps_reference(apps: Vec<AppInfo>, report: &mut ResolverReport) -> Vec<ResolvedApp> {
        report.candidates = apps.len();
        let mut resolved = Vec::<ResolvedApp>::new();
        for app in apps {
            let candidate = AppCandidate::from(app);
            if let Some(index) = resolved
                .iter()
                .position(|existing| should_merge(existing, &candidate))
            {
                merge_resolved(&mut resolved[index], candidate, report);
            } else {
                resolved.push(ResolvedApp {
                    app: candidate.app.clone(),
                    candidates: vec![candidate],
                    evidence: Vec::new(),
                });
            }
        }
        resolved
    }

    /// Group identity plus membership — what a narrowing change must preserve exactly.
    fn grouping(groups: &[ResolvedApp]) -> Vec<(String, Vec<String>)> {
        groups
            .iter()
            .map(|group| {
                let mut members = group
                    .candidates
                    .iter()
                    .map(|candidate| candidate.app.path.clone())
                    .collect::<Vec<_>>();
                members.sort();
                (resolved_canonical_id(group), members)
            })
            .collect()
    }

    /// Deterministic xorshift: the corpus has to be identical on every machine and every run,
    /// so a failure is always reproducible from its seed.
    struct Rng(u64);

    impl Rng {
        fn next_value(&mut self) -> u64 {
            let mut value = self.0;
            value ^= value << 13;
            value ^= value >> 7;
            value ^= value << 17;
            self.0 = value;
            value
        }

        fn pick(&mut self, limit: usize) -> usize {
            (self.next_value() % limit as u64) as usize
        }
    }

    /// A catalog shaped to exercise every merge relation: repeated product families, shared and
    /// nested install roots, shortcuts resolving onto another entry's executable, packaged and
    /// Steam entries, and publishers that are sometimes missing.
    fn generated_catalog(seed: u64, count: usize) -> Vec<AppInfo> {
        let mut rng = Rng(seed | 1);
        let mut catalog = Vec::with_capacity(count);
        for index in 0..count {
            let family = rng.pick(count / 3 + 1);
            let vendor = rng.pick(8);
            let root = format!(r"C:\Vendor{vendor}\Product{family}");
            let executable = format!(r"{root}\product{family}.exe");
            let mut entry = app(&format!("Product {family}"), &executable);
            entry.install_location = Some(root.clone());
            entry.publisher = (rng.pick(4) != 0).then(|| format!("Vendor {vendor}"));
            match rng.pick(6) {
                0 => {
                    entry.source_kind = SourceKind::Registry;
                }
                1 => {
                    entry.source_kind = SourceKind::StartMenu;
                    entry.launch_kind = LaunchKind::Shortcut;
                    entry.path = format!(r"C:\Menu\Product{family}-{index}.lnk");
                    entry.resolved_path = Some(executable);
                }
                2 => {
                    entry.source_kind = SourceKind::Msix;
                    entry.launch_kind = LaunchKind::AppUserModelId;
                    entry.path = format!("Vendor{vendor}.Product{family}_8wekyb!App");
                }
                3 => {
                    entry.source_kind = SourceKind::Steam;
                    entry.path = format!("steam://rungameid/{}", 1000 + family);
                }
                4 => {
                    entry.source_kind = SourceKind::Portable;
                    entry.version = Some(format!("1.{}", rng.pick(5)));
                    // Nested beneath the product root, which is its own merge relation.
                    entry.path = format!(r"{root}\bin\product{family}-helper.exe");
                }
                _ => {
                    entry.source_kind = SourceKind::Portable;
                }
            }
            catalog.push(entry);
        }
        catalog
    }

    #[test]
    fn the_production_resolver_matches_the_reference_on_generated_catalogs() {
        for seed in [1_u64, 7, 42, 1337, 90210] {
            let catalog = generated_catalog(seed, 240);
            let mut production_report = ResolverReport::default();
            let mut reference_report = ResolverReport::default();

            let production = resolve_apps(catalog.clone(), &mut production_report);
            let reference = resolve_apps_reference(catalog, &mut reference_report);

            assert_eq!(
                grouping(&production),
                grouping(&reference),
                "resolver diverged from the reference for seed {seed}"
            );
            assert_eq!(
                production_report.merged, reference_report.merged,
                "merge count diverged for seed {seed}"
            );
        }
    }

    #[test]
    fn the_generated_corpus_actually_exercises_merging() {
        // A harness that never merges would pass no matter how broken the narrowing is.
        let catalog = generated_catalog(42, 240);
        let input = catalog.len();
        let mut report = ResolverReport::default();

        let resolved = resolve_apps(catalog, &mut report);

        assert!(report.merged > 20, "merged only {}", report.merged);
        assert!(resolved.len() < input);
    }

    fn sorted_ids(apps: &[AppInfo]) -> Vec<String> {
        let mut ids = apps.iter().map(|app| app.id.clone()).collect::<Vec<_>>();
        ids.sort();
        ids
    }

    fn shuffle(items: &mut [AppInfo], rng: &mut Rng) {
        for index in (1..items.len()).rev() {
            items.swap(index, rng.pick(index + 1));
        }
    }

    // The deduplication oscillation — duplicate cards that reappeared after a background sync —
    // came from the resolver being order-sensitive: scanners emit entries in different orders, and
    // the frontend delta path then accumulated the least-merged variant. `deduplicate` now
    // canonicalizes the input order, so this must hold at scale, not only on the tiny case above:
    // every shuffle of one catalog yields the identical set of cards, and a second pass is a no-op.
    #[test]
    fn deduplication_is_order_independent_and_idempotent_at_scale() {
        for seed in [3_u64, 19, 256, 4096] {
            let catalog = generated_catalog(seed, 240);
            let baseline = sorted_ids(&resolve(catalog.clone()));

            let mut rng = Rng(seed ^ 0xA9C7_1D3F);
            for attempt in 0..4 {
                let mut shuffled = catalog.clone();
                shuffle(&mut shuffled, &mut rng);
                assert_eq!(
                    sorted_ids(&resolve(shuffled)),
                    baseline,
                    "ordering changed the result (seed {seed}, shuffle {attempt})"
                );
            }

            assert_eq!(
                sorted_ids(&resolve(resolve(catalog))),
                baseline,
                "a second pass changed the result (seed {seed})"
            );
        }
    }

    /// Invariant guard for a catalog far larger than a developer's own machine. It asserts
    /// semantics, never timings (`AGENTS_backend.md` §12): unrelated applications must survive
    /// deduplication rather than collapse into one another. It is also the base for the
    /// planned blocking/indexing work — the resolver currently compares every candidate
    /// against every existing group, and that change has to keep this green.
    #[test]
    fn a_large_catalog_of_distinct_applications_is_not_collapsed() {
        const COUNT: usize = 5_000;
        let catalog = (0..COUNT)
            .map(|index| {
                let mut entry = app(
                    &format!("Product {index}"),
                    &format!(r"C:\Vendor{index}\Product{index}\product{index}.exe"),
                );
                entry.publisher = Some(format!("Vendor {index}"));
                entry.install_location = Some(format!(r"C:\Vendor{index}\Product{index}"));
                entry
            })
            .collect::<Vec<_>>();

        assert_eq!(resolve(catalog).len(), COUNT);
    }

    // `RegistryInstallContainsExecutable` scores 75 and requires neither a matching name nor a
    // matching publisher, so a prefix match that stopped mid-component merged two unrelated
    // applications and nothing else in the scoring would have vetoed it.
    #[test]
    fn a_registry_install_root_does_not_contain_a_similarly_named_folder() {
        let mut registry = app("Prog", r"C:\Prog\prog.exe");
        registry.source_kind = SourceKind::Registry;
        registry.install_location = Some(r"C:\Prog".into());
        let mut unrelated = app("Other Product", r"C:\Program Files\Other\other.exe");
        unrelated.source_kind = SourceKind::Portable;
        unrelated.publisher = Some("Other Vendor".into());

        assert_eq!(resolve(vec![registry, unrelated]).len(), 2);
    }

    #[test]
    fn a_registry_install_root_still_claims_executables_beneath_it() {
        let mut registry = app("Prog", r"C:\Prog\prog.exe");
        registry.source_kind = SourceKind::Registry;
        registry.install_location = Some(r"C:\Prog".into());
        let mut nested = app("Prog Helper", r"C:\Prog\bin\prog-helper.exe");
        nested.source_kind = SourceKind::Portable;

        assert_eq!(resolve(vec![registry, nested]).len(), 1);
    }

    #[test]
    fn canonical_id_uses_resolved_target_independent_of_selected_shortcut() {
        let mut shortcut = app("Battle.net", r"C:\Menu\Battle.net.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"D:\Games\Battle.net\Battle.net.exe".into());
        let mut executable = app("Battle.net", r"D:/Games/Battle.net/Battle.net.exe");
        executable.source_kind = SourceKind::Portable;

        let merged = resolve(vec![executable.clone(), shortcut]);

        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].id, "target:d:\\games\\battle.net\\battle.net.exe",
            "id should be based on canonical target, not the winning path",
        );
    }

    #[test]
    fn localized_start_app_merges_with_english_shortcut_by_target() {
        // English Start-Menu shortcut → eventvwr.msc.
        let mut shortcut = app(
            "Event Viewer",
            r"C:\Menu\Administrative Tools\Event Viewer.lnk",
        );
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Windows\system32\eventvwr.msc".into());
        // Localized Start-App (opaque AutoGenerated AUMID) → same target file.
        let mut start_app = app("Просмотр событий", "Microsoft.AutoGenerated.{BB044BFD}");
        start_app.launch_kind = LaunchKind::AppUserModelId;
        start_app.source_kind = SourceKind::StartApps;
        start_app.resolved_path = Some(r"C:\Windows\system32\eventvwr.msc".into());

        // English machine: the card reads in English but still launches via the localized
        // Start-App — its working shell icon and target are kept, only the name follows the locale.
        let merged = resolve(vec![shortcut.clone(), start_app.clone()]);
        assert_eq!(merged.len(), 1, "same target → one card");
        assert_eq!(merged[0].name, "Event Viewer");
        assert_eq!(
            merged[0].launch_kind,
            LaunchKind::AppUserModelId,
            "keep the localized Start-App as the launch/icon source",
        );

        // Russian machine: the localized name is kept.
        let localized = deduplicate(
            vec![shortcut, start_app],
            |_app| AppCategory::Other,
            NameScript::Cyrillic,
        );
        assert_eq!(localized[0].name, "Просмотр событий");
    }

    #[test]
    fn file_explorer_and_localized_explorer_merge_via_alias() {
        // English "File Explorer" is a PIDL-only shortcut (no readable target).
        let mut shortcut = app("File Explorer", r"C:\Menu\System Tools\File Explorer.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        // Localized "Проводник" (Microsoft.Windows.Explorer) → File Explorer shell CLSID.
        let mut start_app = app("Проводник", "Microsoft.Windows.Explorer");
        start_app.launch_kind = LaunchKind::AppUserModelId;
        start_app.source_kind = SourceKind::StartApps;
        start_app.resolved_path = Some("::{52205FD8-5DFB-447D-801A-D0B52F2E83E1}".into());

        // English machine: English name; the Cyrillic case is covered by the Event Viewer test.
        let merged = resolve(vec![shortcut, start_app]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "File Explorer");
    }

    #[test]
    fn administrative_tools_merge_via_control_applet_alias() {
        let mut shortcut = app("Administrative Tools", r"C:\Menu\Administrative Tools.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Windows\system32\control.exe".into());
        shortcut.launch_arguments = Some("/name Microsoft.AdministrativeTools".into());
        let mut start_app = app(
            "Инструменты Windows",
            "Microsoft.Windows.AdministrativeTools",
        );
        start_app.launch_kind = LaunchKind::AppUserModelId;
        start_app.source_kind = SourceKind::StartApps;
        start_app.resolved_path = Some(r"C:\Windows\system32\control.exe".into());

        // English machine: the English shortcut name wins over the localized Start-App name.
        let merged = resolve(vec![shortcut, start_app]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "Administrative Tools");
    }

    #[test]
    fn run_dialog_shortcut_and_localized_start_app_merge() {
        // English "Run" is a PIDL-only shortcut (no readable target).
        let mut shortcut = app("Run", r"C:\Menu\System Tools\Run.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        // Localized "Выполнить" (Microsoft.Windows.Shell.RunDialog) → Run dialog shell CLSID.
        let mut start_app = app("Выполнить", "Microsoft.Windows.Shell.RunDialog");
        start_app.launch_kind = LaunchKind::AppUserModelId;
        start_app.source_kind = SourceKind::StartApps;
        start_app.resolved_path = Some("::{2559A1F3-21D7-11D4-BDAF-00C04F60B9F0}".into());

        let merged = resolve(vec![shortcut, start_app]);

        assert_eq!(merged.len(), 1, "the Run dialog is a single card");
        assert_eq!(
            merged[0].name, "Run",
            "English machine shows the Latin name"
        );
        assert_eq!(merged[0].launch_kind, LaunchKind::AppUserModelId);
    }

    #[test]
    fn different_system_tools_stay_separate() {
        let mut events = app("Просмотр событий", "Microsoft.AutoGenerated.{A}");
        events.launch_kind = LaunchKind::AppUserModelId;
        events.source_kind = SourceKind::StartApps;
        events.resolved_path = Some(r"C:\Windows\system32\eventvwr.msc".into());
        let mut computer = app("Управление компьютером", "Microsoft.AutoGenerated.{B}");
        computer.launch_kind = LaunchKind::AppUserModelId;
        computer.source_kind = SourceKind::StartApps;
        computer.resolved_path = Some(r"C:\Windows\system32\compmgmt.msc".into());

        assert_eq!(resolve(vec![events, computer]).len(), 2);
    }

    #[test]
    fn canonical_id_prefers_steam_and_aumid_identities() {
        let mut steam = app("Hearthstone", "steam://rungameid/12345");
        steam.source_kind = SourceKind::Steam;
        let mut packaged = app(
            "Calculator",
            "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
        );
        packaged.launch_kind = LaunchKind::AppUserModelId;
        packaged.source_kind = SourceKind::StartApps;

        assert_eq!(canonical_id(&steam), "steam:12345");
        assert_eq!(
            canonical_id(&packaged),
            "aumid:microsoft.windowscalculator_8wekyb3d8bbwe!app",
        );
    }

    #[test]
    fn evidence_merges_registry_and_shortcut_for_same_install_root() {
        let mut registry = app("Visual Studio Code", r"C:\Program Files\Code\Code.exe");
        registry.source_kind = SourceKind::Registry;
        registry.publisher = Some("Microsoft".into());
        registry.install_location = Some(r"C:\Program Files\Code".into());
        let mut shortcut = app("Code", r"C:\Menu\Visual Studio Code.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Program Files\Code\Code.exe".into());

        let merged = resolve(vec![registry, shortcut]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].launch_kind, LaunchKind::Shortcut);
    }

    #[test]
    fn evidence_does_not_merge_same_name_with_conflicting_publishers() {
        let mut first = app("Studio", r"C:\Alpha\Studio.exe");
        first.publisher = Some("Alpha".into());
        let mut second = app("Studio", r"C:\Beta\Studio.exe");
        second.publisher = Some("Beta".into());

        assert_eq!(resolve(vec![first, second]).len(), 2);
    }

    #[test]
    fn portable_apps_in_different_roots_keep_separate_canonical_ids() {
        let mut first = app("Toolbox", r"D:\Tools\Toolbox\toolbox.exe");
        first.source_kind = SourceKind::Portable;
        first.install_location = Some(r"D:\Tools\Toolbox".into());
        let mut second = app("Toolbox", r"E:\Archive\Toolbox\toolbox.exe");
        second.source_kind = SourceKind::Portable;
        second.install_location = Some(r"E:\Archive\Toolbox".into());

        let merged = resolve(vec![first, second]);

        assert_eq!(merged.len(), 2);
        assert_ne!(merged[0].id, merged[1].id);
    }

    #[test]
    fn helper_executable_does_not_win_over_main_executable() {
        let main = app("Docker Desktop", r"C:\Docker\Docker Desktop.exe");
        let helper = app("Docker Desktop Helper", r"C:\Docker\helper.exe");

        let merged = resolve(vec![helper, main]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].path, r"C:\Docker\Docker Desktop.exe");
    }

    #[test]
    fn merged_launcher_preserves_primary_visibility_over_auxiliary_shortcut_rank() {
        let mut main = app("Tool", r"C:\Tool\Tool.exe");
        main.visibility_class = crate::catalog::VisibilityClass::Primary;
        main.visibility_score = 20;
        let mut helper = app("Tool Diagnostics", r"C:\Tool\Tool Diagnostics.lnk");
        helper.launch_kind = LaunchKind::Shortcut;
        helper.resolved_path = Some(r"C:\Tool\Tool.exe".into());
        helper.visibility_class = crate::catalog::VisibilityClass::Auxiliary;
        helper.visibility_score = -20;

        let merged = resolve(vec![helper, main]);

        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].visibility_class,
            crate::catalog::VisibilityClass::Primary
        );
        assert_eq!(merged[0].visibility_score, 20);
    }

    #[test]
    fn meaningful_arguments_keep_quoted_multi_word_values_together() {
        assert_eq!(
            meaningful_launch_arguments(
                Some(r#"--profile-directory="Profile 1" --ignored value"#,)
            ),
            Some("--profile-directory=profile 1".into())
        );
        assert_eq!(
            meaningful_launch_arguments(Some(r#"--user-data-dir "C:\My Profile""#)),
            Some(r"--user-data-dir c:\my profile".into())
        );
    }

    #[test]
    fn equivalent_user_data_paths_have_the_same_launch_fingerprint() {
        assert_eq!(
            meaningful_launch_arguments(Some(
                r#"--user-data-dir="C:\Users\User Name\App Data\Browser Profile""#,
            )),
            meaningful_launch_arguments(Some(
                r#"--user-data-dir="c:/users/user name/app data/browser profile""#,
            ))
        );
    }

    #[test]
    fn firefox_shortcut_and_registry_entry_merge_by_product_family() {
        let mut shortcut = app(
            "Firefox",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Firefox.lnk",
        );
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.publisher = Some("Mozilla Foundation".into());
        shortcut.resolved_path = Some(r"C:\Program Files\Mozilla Firefox\firefox.exe".into());
        let mut registry = app(
            "Mozilla Firefox (x64 ru)",
            r"C:\Program Files\Mozilla Firefox\firefox.exe",
        );
        registry.source_kind = SourceKind::Registry;
        registry.publisher = Some("Mozilla".into());
        registry.install_location = Some(r"C:\Program Files\Mozilla Firefox".into());

        let merged = resolve(vec![registry, shortcut]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "Firefox");
    }

    #[test]
    fn firefox_full_windows_candidate_set_keeps_main_and_private_entries() {
        let mut registry = app(
            "Mozilla Firefox (x64 ru)",
            r"C:\Program Files\Mozilla Firefox\firefox.exe",
        );
        registry.source_kind = SourceKind::Registry;
        registry.publisher = Some("Mozilla".into());
        registry.install_location = Some(r"C:\Program Files\Mozilla Firefox".into());
        let mut shortcut = app(
            "Firefox",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Firefox.lnk",
        );
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Program Files\Mozilla Firefox\firefox.exe".into());
        let mut private_shortcut = app(
            "Private Browsing Firefox",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Private Browsing Firefox.lnk",
        );
        private_shortcut.launch_kind = LaunchKind::Shortcut;
        private_shortcut.source_kind = SourceKind::StartMenu;
        private_shortcut.resolved_path =
            Some(r"C:\Program Files\Mozilla Firefox\private_browsing.exe".into());
        let mut aumid = app("Firefox", "308046B0AF4A39CB");
        aumid.launch_kind = LaunchKind::AppUserModelId;
        aumid.source_kind = SourceKind::StartApps;
        let mut private_aumid = app(
            "Private Browsing Firefox",
            "308046B0AF4A39CB;PrivateBrowsingAUMID",
        );
        private_aumid.launch_kind = LaunchKind::AppUserModelId;
        private_aumid.source_kind = SourceKind::StartApps;

        let merged = resolve(vec![
            registry,
            shortcut,
            private_shortcut,
            aumid,
            private_aumid,
        ]);

        assert_eq!(merged.len(), 2);
        assert_eq!(
            merged
                .iter()
                .map(|app| app.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Firefox", "Private Browsing Firefox"],
        );
    }

    #[test]
    fn world_of_warcraft_shortcut_and_launcher_merge_by_nested_install_root() {
        let mut shortcut = app(
            "World of Warcraft",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\World of Warcraft\World of Warcraft.lnk",
        );
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.publisher = Some("Blizzard Entertainment".into());
        shortcut.install_location = Some(r"D:\Games\World of Warcraft\_retail_".into());
        let mut launcher = app(
            "World of Warcraft Launcher",
            r"D:\Games\World of Warcraft\World of Warcraft Launcher.exe",
        );
        launcher.source_kind = SourceKind::Portable;
        launcher.publisher = Some("Blizzard Entertainment".into());
        launcher.install_location = Some(r"D:\Games\World of Warcraft".into());

        let merged = resolve(vec![launcher, shortcut]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "World of Warcraft");
    }

    #[test]
    fn command_line_app_shortcut_and_executable_merge_by_resolved_target() {
        let mut shortcut = app(
            "Claude Code",
            r"C:\Users\User\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Claude Code.lnk",
        );
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(r"C:\Users\User\.local\bin\claude.exe".into());
        let mut executable = app("Claude Code", r"C:\Users\User\.local\bin\claude.exe");
        executable.source_kind = SourceKind::Portable;

        let merged = resolve(vec![shortcut, executable]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id, r"target:c:\users\user\.local\bin\claude.exe");
    }

    #[test]
    fn equal_product_names_in_independent_install_roots_stay_separate() {
        let mut installed = app("Agent", r"C:\Program Files\Agent\agent.exe");
        installed.publisher = Some("Example".into());
        installed.install_location = Some(r"C:\Program Files\Agent".into());
        let mut portable = app("Agent", r"D:\Portable\Agent\agent.exe");
        portable.source_kind = SourceKind::Portable;
        portable.publisher = Some("Example".into());
        portable.install_location = Some(r"D:\Portable\Agent".into());

        assert_eq!(resolve(vec![installed, portable]).len(), 2);
    }

    #[test]
    fn localized_windows_tool_names_share_product_family() {
        let mut english = app("Task Manager", r"C:\Windows\System32\taskmgr.exe");
        english.source_kind = SourceKind::StartApps;
        english.launch_kind = LaunchKind::AppUserModelId;
        let mut localized = app(
            "Диспетчер задач",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\System Tools\Task Manager.lnk",
        );
        localized.source_kind = SourceKind::StartMenu;
        localized.launch_kind = LaunchKind::Shortcut;

        let merged = resolve(vec![english, localized]);

        assert_eq!(merged.len(), 1);
        assert_eq!(
            normalized_product_family("Task Manager"),
            normalized_product_family("Диспетчер задач")
        );
    }

    #[test]
    fn preference_identity_survives_a_change_from_registry_to_shortcut_source() {
        let mut registry = app("Example Editor", r"C:\Example\Editor.exe");
        registry.source_kind = SourceKind::Registry;
        registry.product_name = Some("Example Editor".into());
        registry.publisher = Some("Example Software LLC".into());
        registry.install_location = Some(r"C:\Example".into());
        let mut shortcut = app("Editor", r"C:\Menu\Example Editor.lnk");
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.resolved_path = Some(r"C:\Example\Editor.exe".into());
        shortcut.product_name = Some("Example Editor".into());
        shortcut.publisher = Some("Example Software".into());
        shortcut.install_location = Some(r"C:\Example".into());

        assert_eq!(
            preference_identity(&registry),
            preference_identity(&shortcut)
        );
    }

    #[test]
    fn preference_identity_keeps_portable_copies_in_different_roots_separate() {
        let mut first = app("Tool", r"D:\Tools\Tool\Tool.exe");
        first.source_kind = SourceKind::Portable;
        first.product_name = Some("Tool".into());
        first.publisher = Some("Vendor".into());
        first.install_location = Some(r"D:\Tools\Tool".into());
        let mut second = first.clone();
        second.path = r"E:\Archive\Tool\Tool.exe".into();
        second.install_location = Some(r"E:\Archive\Tool".into());

        assert_ne!(preference_identity(&first), preference_identity(&second));
    }

    #[test]
    fn normalized_windows_paths_ignore_quotes_slashes_and_dot_segments() {
        assert_eq!(
            normalize_path(r#""C:/Apps/Tool/./bin/../Tool.exe""#),
            normalize_path(r"c:\apps\tool\tool.exe")
        );
    }

    #[test]
    fn insignificant_shortcut_arguments_do_not_split_the_same_launcher() {
        let mut plain = app("Browser", r"C:\Menu\Browser.lnk");
        plain.launch_kind = LaunchKind::Shortcut;
        plain.resolved_path = Some(r"C:\Browser\browser.exe".into());
        let mut flagged = plain.clone();
        flagged.path = r"C:\Menu\Browser Safe.lnk".into();
        flagged.launch_arguments = Some("--disable-gpu".into());

        assert_eq!(resolve(vec![plain, flagged]).len(), 1);
    }

    #[test]
    fn meaningful_profile_arguments_keep_shortcuts_separate() {
        let mut work = app("Browser Work", r"C:\Menu\Browser Work.lnk");
        work.launch_kind = LaunchKind::Shortcut;
        work.resolved_path = Some(r"C:\Browser\browser.exe".into());
        work.launch_arguments = Some("--profile-directory=Work".into());
        let mut personal = work.clone();
        personal.name = "Browser Personal".into();
        personal.path = r"C:\Menu\Browser Personal.lnk".into();
        personal.launch_arguments = Some("--profile-directory=Personal".into());

        assert_eq!(resolve(vec![work, personal]).len(), 2);
    }
}
