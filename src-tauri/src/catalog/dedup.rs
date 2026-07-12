use crate::catalog::{AppCategory, AppInfo, LaunchKind, SourceKind};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Component, Path};

#[derive(Clone, Debug)]
pub(super) struct AppCandidate {
    app: AppInfo,
    family: String,
    identity: CandidateIdentity,
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

// ---------------------------------------------------------------------------
// Dev-only deduplication diagnostics (never surfaced to end users). Enabled in debug
// builds or via WINAPPS_DEDUP_REPORT=1; writes %LOCALAPPDATA%\WindowsApps\dedup-report.json
// so the developer can see what merged (and why) and which same-family entries stayed
// separate (potential confusion) to tune the rules.
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct DedupMember {
    name: String,
    path: String,
    source: String,
}

#[derive(Serialize)]
struct DedupGroup {
    canonical_id: String,
    name: String,
    members: Vec<DedupMember>,
    evidence: Vec<String>,
}

#[derive(Serialize)]
struct PossibleConfusion {
    family: String,
    entries: Vec<String>,
}

#[derive(Serialize)]
struct DedupReport {
    input: usize,
    output: usize,
    merged_groups: Vec<DedupGroup>,
    possible_confusions: Vec<PossibleConfusion>,
}

pub(super) fn dev_report_enabled() -> bool {
    cfg!(debug_assertions) || std::env::var("WINAPPS_DEDUP_REPORT").is_ok()
}

pub(super) fn write_dev_report(apps: &[AppInfo]) {
    // Skip tiny incremental sub-lists so a full-catalog report isn't clobbered by a
    // background single-source sync. Dev diagnostic only.
    if apps.len() < 30 {
        return;
    }
    let report = analyze(apps.to_vec());
    let Ok(base) = std::env::var("LOCALAPPDATA") else {
        return;
    };
    let path = Path::new(&base)
        .join("WindowsApps")
        .join("dedup-report.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&report) {
        let _ = std::fs::write(path, json);
    }
}

fn analyze(apps: Vec<AppInfo>) -> DedupReport {
    let input = apps.len();
    let mut report = ResolverReport::default();
    let resolved = resolve_apps(apps, &mut report);
    let output = resolved.len();
    let merged_groups = resolved
        .iter()
        .filter(|group| group.candidates.len() > 1)
        .map(|group| DedupGroup {
            canonical_id: resolved_canonical_id(group),
            name: group.app.name.clone(),
            members: group
                .candidates
                .iter()
                .map(|candidate| DedupMember {
                    name: candidate.app.name.clone(),
                    path: candidate.app.path.clone(),
                    source: format!("{:?}", candidate.app.source_kind),
                })
                .collect(),
            evidence: group
                .evidence
                .iter()
                .map(|item| format!("{:?} ({})", item.reason, item.score))
                .collect(),
        })
        .collect();
    let mut by_family: HashMap<String, Vec<String>> = HashMap::new();
    for group in &resolved {
        let family =
            launcher_product_family(&normalized_product_family(&group.app.name)).to_string();
        by_family
            .entry(family)
            .or_default()
            .push(format!("{} [{}]", group.app.name, group.app.path));
    }
    let mut possible_confusions = by_family
        .into_iter()
        .filter(|(_, entries)| entries.len() > 1)
        .map(|(family, entries)| PossibleConfusion { family, entries })
        .collect::<Vec<_>>();
    possible_confusions.sort_by(|left, right| left.family.cmp(&right.family));
    DedupReport {
        input,
        output,
        merged_groups,
        possible_confusions,
    }
}

pub(super) fn deduplicate(
    apps: Vec<AppInfo>,
    classify: impl Fn(&str, &str) -> AppCategory,
) -> Vec<AppInfo> {
    let mut report = ResolverReport::default();
    let resolved = resolve_apps(apps, &mut report);
    let mut apps = resolved
        .into_iter()
        .map(|mut resolved| {
            resolved.app.id = resolved_canonical_id(&resolved);
            resolved.app.canonical_identity = Some(preference_identity(&resolved.app));
            resolved.app
        })
        .collect::<Vec<_>>();
    for app in &mut apps {
        app.name = app.name.split_whitespace().collect::<Vec<_>>().join(" ");
        app.category = classify(&app.name, &app.path);
    }
    apps.sort_by_cached_key(|app| (category_rank(app.category), app.name.to_lowercase()));
    apps
}

fn resolve_apps(apps: Vec<AppInfo>, report: &mut ResolverReport) -> Vec<ResolvedApp> {
    report.candidates = apps.len();
    let mut resolved = Vec::<ResolvedApp>::new();
    for app in apps {
        let candidate = AppCandidate::from(app);
        if let Some(index) = resolved
            .iter()
            .position(|existing| should_merge(existing, &candidate))
        {
            let existing = resolved.remove(index);
            let merged = merge_resolved(existing, candidate, report);
            resolved.insert(index, merged);
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

impl From<AppInfo> for AppCandidate {
    fn from(app: AppInfo) -> Self {
        Self {
            family: normalized_product_family(&app.name),
            identity: CandidateIdentity::from_app(&app),
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
    let best = existing
        .candidates
        .iter()
        .map(|left| score_evidence(left, candidate))
        .max_by_key(|(_, score)| *score);
    let Some((evidence, score)) = best else {
        return false;
    };
    if score < 75
        && existing
            .candidates
            .iter()
            .any(|left| conflicting_install_roots(left, candidate))
    {
        return false;
    }
    if score >= 80 {
        return true;
    }
    if score >= 50 && !publishers_conflict(&existing.app, &candidate.app) {
        return true;
    }
    evidence.iter().any(|item| {
        item.reason == EvidenceReason::SameLaunchTarget
            || item.reason == EvidenceReason::ShortcutTargetsExecutable
            || item.reason == EvidenceReason::SameSteamAppId
            || item.reason == EvidenceReason::SameAumid
    })
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
    mut existing: ResolvedApp,
    candidate: AppCandidate,
    report: &mut ResolverReport,
) -> ResolvedApp {
    let (evidence, _score) = existing
        .candidates
        .iter()
        .map(|left| score_evidence(left, &candidate))
        .max_by_key(|(_, score)| *score)
        .unwrap_or_default();
    report.merged += 1;
    report.evidence.extend(evidence.iter().cloned());
    existing.evidence.extend(evidence);
    existing.app = merge_app(existing.app, candidate.app.clone());
    existing.candidates.push(candidate);
    existing
}

fn score_evidence(left: &AppCandidate, right: &AppCandidate) -> (Vec<Evidence>, u16) {
    let mut evidence = Vec::new();
    let mut add = |reason: EvidenceReason, score: u16| evidence.push(Evidence { reason, score });

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
        && !normalized_publisher(left.app.publisher.as_deref()).is_empty()
        && normalized_publisher(left.app.publisher.as_deref())
            == normalized_publisher(right.app.publisher.as_deref())
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
    if same_folder_helper_variant(left, right) {
        add(EvidenceReason::SameFolderHelperVariant, 80);
    }
    if left.family == right.family {
        add(EvidenceReason::SameFamily, 60);
    } else if launcher_product_family(&left.family) == launcher_product_family(&right.family) {
        add(EvidenceReason::NamePrefixOnly, 10);
    }

    let score = evidence.iter().map(|item| item.score).max().unwrap_or(0);
    (evidence, score)
}

fn same_folder_and_family(left: &AppCandidate, right: &AppCandidate) -> bool {
    parent_path(&left.identity.path).is_some_and(|left_parent| {
        parent_path(&right.identity.path).is_some_and(|right_parent| {
            left_parent == right_parent
                && launcher_product_family(&left.family) == launcher_product_family(&right.family)
        })
    })
}

fn nested_install_root_and_family(left: &AppCandidate, right: &AppCandidate) -> bool {
    let left_root = left.identity.install_root.as_ref();
    let right_root = right.identity.install_root.as_ref();
    let same_family =
        launcher_product_family(&left.family) == launcher_product_family(&right.family);
    same_family
        && matches!(
            (left_root, right_root),
            (Some(left), Some(right))
                if left.starts_with(&format!("{right}\\"))
                    || right.starts_with(&format!("{left}\\"))
        )
}

fn same_folder_helper_variant(left: &AppCandidate, right: &AppCandidate) -> bool {
    let left_parent = parent_path(&left.identity.path);
    let right_parent = parent_path(&right.identity.path);
    left_parent.is_some()
        && left_parent == right_parent
        && helper_variant_family(&left.family) == helper_variant_family(&right.family)
        && (is_helper_candidate(&left.app) || is_helper_candidate(&right.app))
}

fn shortcut_same_family(left: &AppCandidate, right: &AppCandidate) -> bool {
    (left.app.launch_kind == LaunchKind::Shortcut || right.app.launch_kind == LaunchKind::Shortcut)
        && launcher_product_family(&left.family) == launcher_product_family(&right.family)
}

fn versioned_portable_copy(left: &AppCandidate, right: &AppCandidate) -> bool {
    left.app.source_kind == SourceKind::Portable
        && right.app.source_kind == SourceKind::Portable
        && left.family == right.family
        && (left.app.version.is_some() || right.app.version.is_some())
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
            "--profile-directory" | "--user-data-dir" | "--app" | "--app-id" | "--class" | "-p"
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
        if inline {
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
                .is_some_and(|root| executable.identity.path.starts_with(root))
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

fn normalized_publisher(value: Option<&str>) -> String {
    value
        .unwrap_or_default()
        .to_lowercase()
        .replace("corporation", "")
        .replace("incorporated", "")
        .replace("limited", "")
        .replace("company", "")
        .replace("corp", "")
        .replace("inc", "")
        .replace("llc", "")
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn launch_target(app: &AppInfo) -> Option<&str> {
    app.resolved_path.as_deref()
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
        return format!("target:{target}");
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
        return format!("target:{target}");
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

fn merge_app(left: AppInfo, right: AppInfo) -> AppInfo {
    let prefer_right = candidate_score(&right) > candidate_score(&left)
        || (candidate_score(&right) == candidate_score(&left)
            && left.source_kind == SourceKind::Portable
            && right.source_kind == SourceKind::Portable
            && version_key(right.version.as_deref()) > version_key(left.version.as_deref()));
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

pub(super) fn normalized_product_family(name: &str) -> String {
    let mut value = normalize_name(name);
    for marker in [
        " (64bit)",
        " (32bit)",
        " (64-bit)",
        " (32-bit)",
        " x64",
        " x86",
    ] {
        if value.ends_with(marker) {
            value.truncate(value.len() - marker.len());
        }
    }
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
        AppCategory::Browsers => 4,
        AppCategory::Media => 5,
        AppCategory::Communication => 6,
        AppCategory::Utilities => 7,
        AppCategory::System => 8,
        AppCategory::WindowsFeatures => 9,
        AppCategory::Other => 10,
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
            visibility_class: Default::default(),
            visibility_score: 0,
            visibility_reasons: Vec::new(),
        }
    }

    fn resolve(apps: Vec<AppInfo>) -> Vec<AppInfo> {
        deduplicate(apps, |_name, _path| AppCategory::Other)
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

        let merged = resolve(vec![shortcut, start_app]);

        assert_eq!(merged.len(), 1, "same target → one card");
        assert_eq!(merged[0].name, "Просмотр событий", "keep localized name");
        assert_eq!(
            merged[0].launch_kind,
            LaunchKind::AppUserModelId,
            "keep the localized Start-App as the launch/icon source",
        );
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

        let merged = resolve(vec![shortcut, start_app]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "Проводник");
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

        let merged = resolve(vec![shortcut, start_app]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "Инструменты Windows");
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
