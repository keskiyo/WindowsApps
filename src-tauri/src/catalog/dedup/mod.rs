//! Deduplication: turning a scanned catalog into one card per real application.
//!
//! `deduplicate` is the entry point and the only order-dependent part; the submodules own the
//! decisions it composes. Splitting them apart is what lets each be reviewed on its own terms:
//!
//! - [`candidate`] builds a comparable candidate and narrows the search space;
//! - [`evidence`] decides whether two candidates are the same application;
//! - [`identity`] produces the ids a card keeps across scans;
//! - [`family`] normalizes display names and publishers;
//! - [`merge`] combines two records that resolved together.

use crate::catalog::{AppCategory, AppInfo, ArtifactKind};
use crate::platform::windows::NameScript;

mod arguments;
mod blocking;
mod candidate;
mod display_name;
mod evidence;
mod family;
mod identity;
mod merge;
mod report;
mod signals;
mod target;

use blocking::{equality_keys, BlockingIndex};
use candidate::{candidate_score, AppCandidate};
use display_name::choose_display_name;
use evidence::should_merge;
use identity::{card_preference_identity, resolved_canonical_id};
use merge::{merge_resolved, ResolvedApp, ResolverReport};

pub(crate) use report::{dev_report_enabled, write_dev_report};
// The surface `catalog` itself consumes. Everything else stays inside this module tree.
pub(super) use family::normalized_product_family;
pub(super) use identity::preference_identity;
pub(super) use target::normalize_path;

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
        // Artifact classification owns the reserved category. Reclassifying here used to move an
        // installer or a documentation shortcut into an ordinary category whenever it resolved
        // alone — only records that happened to merge kept it, because `merge` restores it.
        app.category = if app.artifact_kind == ArtifactKind::Application {
            classify(app)
        } else {
            AppCategory::InstallersDocs
        };
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
        AppCategory::InstallersDocs => 13,
        AppCategory::Other => 14,
    }
}

/// Behaviour tests for the module as a whole: they drive `deduplicate`/`resolve_apps` and assert
/// which cards come out, which is this module's responsibility. Unit tests for an individual
/// helper live beside that helper in its own submodule.
#[cfg(test)]
mod tests {
    use super::arguments::meaningful_launch_arguments;
    use super::display_name::name_script;
    use super::family::normalized_publisher;
    use super::identity::{canonical_id, card_preference_identity};
    use super::signals::publishers_conflict;
    use super::target::is_generic_interpreter_host;
    use super::*;
    use crate::catalog::{ArtifactKind, LaunchKind, SourceKind};

    fn app(name: &str, path: &str) -> AppInfo {
        AppInfo {
            id: String::new(),
            name: name.into(),
            path: path.into(),
            icon_base64: None,
            artifact_kind: Default::default(),
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

    // A signing-certificate subject and a marketing name are not comparable strings. Treating
    // "CN=AMD" as a publisher made it conflict with "Advanced Micro Devices, Inc." — the same
    // vendor — and kept one product in two cards.
    #[test]
    fn a_certificate_subject_is_not_treated_as_a_publisher() {
        assert_eq!(normalized_publisher(Some("CN=AMD")), "");
        assert_eq!(normalized_publisher(Some("cn=Contoso, O=Contoso Ltd")), "");
        // A real name that merely begins with those letters is untouched.
        assert_eq!(normalized_publisher(Some("CNC Software")), "cncsoftware");
    }

    // Publishers are arbitrary UTF-8 from the registry and from PE metadata. The certificate-
    // subject check byte-sliced the first three bytes, which lands inside a code point when the
    // name starts with a multi-byte character — a real scan on a machine with a Cyrillic vendor
    // name panicked with "byte index 3 is not a char boundary".
    #[test]
    fn a_multibyte_publisher_name_does_not_panic() {
        assert_eq!(
            normalized_publisher(Some("АО Лаборатория")),
            "аолаборатория"
        );
        assert_eq!(normalized_publisher(Some("Ы")), "ы");
        assert_eq!(normalized_publisher(Some("日本")), "日本");
        assert_eq!(normalized_publisher(Some("é")), "é");
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

    #[test]
    fn wsl_start_apps_do_not_merge_by_shared_host_target() {
        let mut ubuntu = app("Ubuntu", "Microsoft.AutoGenerated.{UBUNTU}");
        ubuntu.launch_kind = LaunchKind::AppUserModelId;
        ubuntu.source_kind = SourceKind::StartApps;
        ubuntu.resolved_path = Some(r"C:\Program Files\WSL\wsl.exe".into());
        let mut debian = app("Debian", "Microsoft.AutoGenerated.{DEBIAN}");
        debian.launch_kind = LaunchKind::AppUserModelId;
        debian.source_kind = SourceKind::StartApps;
        debian.resolved_path = Some(r"C:\Program Files\WSL\wsl.exe".into());

        assert_ne!(preference_identity(&ubuntu), preference_identity(&debian));
        assert_eq!(resolve(vec![ubuntu, debian]).len(), 2);
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

    fn squirrel_pair(root: &str, executable: &str, version: &str) -> (AppInfo, AppInfo) {
        let mut application = app(executable, &format!(r"C:\Menu\{executable}.lnk"));
        application.launch_kind = LaunchKind::Shortcut;
        application.source_kind = SourceKind::StartMenu;
        application.resolved_path = Some(format!(r"{root}\app-{version}\{executable}.exe"));
        application.version = Some(version.into());
        application.publisher = Some(format!("{executable} Inc."));

        // The updater stub Squirrel registers beside it: same product name, different target,
        // different version, and signed by GitHub rather than the vendor.
        let mut stub = app(
            executable,
            &format!(r"C:\Menu\{executable} Inc\{executable}.lnk"),
        );
        stub.launch_kind = LaunchKind::Shortcut;
        stub.source_kind = SourceKind::StartMenu;
        stub.resolved_path = Some(format!(r"{root}\Update.exe"));
        stub.launch_arguments = Some(format!("--processStart {executable}.exe"));
        stub.version = Some("1.1.1.0".into());
        stub.publisher = Some("GitHub".into());
        (application, stub)
    }

    // Real corpus: Discord occupied two cards, one launching `app-1.0.9250\Discord.exe` and one
    // launching `Update.exe --processStart Discord.exe`. Nothing tied them together — different
    // targets, versions 1.0.9250 vs 1.1.1.0, publishers "Discord Inc." vs "GitHub" — so the
    // name-level evidence was vetoed twice over and the duplicate reached the quick-launch palette.
    #[test]
    fn a_squirrel_updater_stub_merges_into_its_application_card() {
        let (application, stub) = squirrel_pair(
            r"C:\Users\Maks\AppData\Local\Discord",
            "Discord",
            "1.0.9250",
        );

        let resolved = resolve(vec![application, stub]);

        assert_eq!(resolved.len(), 1);
        // The card must keep the application's own target, not the stub's updater.
        assert_eq!(
            resolved[0].resolved_path.as_deref(),
            Some(r"C:\Users\Maks\AppData\Local\Discord\app-1.0.9250\Discord.exe"),
        );
        assert!(resolved[0].launch_arguments.is_none());
    }

    // The key is the package root plus the executable that ends up running, so a stub pointing at a
    // different program in the same folder is not evidence of anything.
    #[test]
    fn a_squirrel_stub_for_another_program_does_not_merge() {
        let root = r"C:\Users\Maks\AppData\Local\Vendor";
        let (application, mut stub) = squirrel_pair(root, "Alpha", "2.0.0");
        // Same product name and the same package root, but the stub starts a different executable:
        // the identity match must not fire, leaving only the name-level evidence the version veto
        // rejects.
        stub.launch_arguments = Some("--processStart Beta.exe".into());

        assert_eq!(resolve(vec![application, stub]).len(), 2);
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

    #[test]
    fn same_version_installer_does_not_contaminate_installed_application() {
        for (name, version, installed_path, target, setup_path, publisher) in [
            (
                "FileZilla",
                "3.67.0",
                r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\FileZilla FTP Client\FileZilla.lnk",
                r"C:\Program Files\FileZilla FTP Client\filezilla.exe",
                r"D:\Downloads\FileZilla_3.67.0_win64-setup.exe",
                "Tim Kosse",
            ),
            (
                "Windhawk",
                "1.6",
                r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Windhawk.lnk",
                r"D:\Apps\Windhawk\windhawk.exe",
                r"D:\Downloads\windhawk_setup.exe",
                "Ramen Software",
            ),
            (
                "Hiddify",
                "4.1.1+40101",
                r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Hiddify.lnk",
                r"C:\Program Files\Hiddify\Hiddify.exe",
                r"D:\Downloads\Hiddify-Windows-Setup-x64.exe",
                "Hiddify",
            ),
        ] {
            let mut installed = app(name, installed_path);
            installed.launch_kind = LaunchKind::Shortcut;
            installed.source_kind = SourceKind::StartMenu;
            installed.resolved_path = Some(target.into());
            installed.version = Some(version.into());
            installed.publisher = Some(publisher.into());
            installed.product_name = Some(name.into());

            let mut setup = app(name, setup_path);
            setup.source_kind = SourceKind::Portable;
            setup.version = Some(version.into());
            setup.publisher = Some(publisher.into());
            setup.product_name = Some(name.into());
            setup.install_location = Some(r"D:\Downloads".into());
            setup.artifact_kind = ArtifactKind::Installer;

            let separated = resolve(vec![installed, setup]);

            assert_eq!(separated.len(), 2, "{name}");
            assert!(separated
                .iter()
                .any(|app| app.artifact_kind == ArtifactKind::Application));
            assert!(separated
                .iter()
                .any(|app| app.artifact_kind == ArtifactKind::Installer));
        }
    }

    #[test]
    fn cross_kind_records_for_the_exact_same_setup_target_still_merge() {
        let target = r"C:\Program Files (x86)\MySQL\MySQL Installer for Windows\MySQLInstaller.exe";
        let mut shortcut = app("MySQL Installer", r"C:\Menu\MySQL Installer.lnk");
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.resolved_path = Some(target.into());
        shortcut.artifact_kind = ArtifactKind::Installer;
        let executable = app("MySQL Installer", target);

        let merged = resolve(vec![shortcut, executable]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].artifact_kind, ArtifactKind::Installer);
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

    // The same rule for an SDK sample. `classify_visibility` treated `SdkSample` as sticky but the
    // merge copy of that list did not, so a sample merged with an AUMID sibling was promoted back
    // into the main catalog. Both now consult `visibility::is_sticky_auxiliary`.
    #[test]
    fn merged_sdk_sample_stays_auxiliary() {
        use crate::catalog::visibility::apply_visibility;
        use crate::catalog::VisibilityClass;

        use crate::catalog::VisibilityReason;

        // Only the description carries the sample marker. The path deliberately avoids
        // `\sdk\samples\`, which would also trip `is_bundled_toolchain_path` and add a
        // `ProductComponent` reason — that one was already sticky, so it would mask the defect.
        let mut sample = app("SharedMemory Tool", r"D:\Apps\Server\Release\shmtool.exe");
        sample.source_kind = SourceKind::Portable;
        sample.description = Some("shared memory sample".into());
        apply_visibility(&mut sample);
        assert!(sample
            .visibility_reasons
            .contains(&VisibilityReason::SdkSample));
        assert!(!sample
            .visibility_reasons
            .contains(&VisibilityReason::ProductComponent));

        // The sibling must be a genuine Primary, or the merge is never asked to decide anything.
        let mut aumid = app("SharedMemory Tool", "Contoso.ShmTool_1!App");
        aumid.launch_kind = LaunchKind::AppUserModelId;
        aumid.source_kind = SourceKind::StartApps;
        aumid.resolved_path = Some(r"D:\Apps\Server\Launcher.exe".into());
        apply_visibility(&mut aumid);

        assert_eq!(sample.visibility_class, VisibilityClass::Auxiliary);
        assert_eq!(aumid.visibility_class, VisibilityClass::Primary);

        let merged = resolve(vec![aumid, sample]);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].visibility_class,
            VisibilityClass::Auxiliary,
            "reasons: {:?}",
            merged[0].visibility_reasons
        );
    }

    /// Maintenance entries are demoted rather than rejected now, so for the first time they reach
    /// deduplication. A "reset preferences" shortcut resolves to the same executable as the player
    /// and outranks it on `candidate_score` (a `.lnk` beats an `.exe`), so a merge would hand the
    /// card a launch target that wipes the user's settings. Its arguments are what must keep the
    /// two apart.
    #[test]
    fn a_reset_shortcut_never_takes_over_the_application_card() {
        use crate::catalog::visibility::apply_visibility;
        use crate::catalog::VisibilityClass;

        let mut reset = app(
            "VLC media player - reset preferences and cache files",
            r"C:\Menu\VLC media player - reset preferences and cache files.lnk",
        );
        reset.launch_kind = LaunchKind::Shortcut;
        reset.source_kind = SourceKind::StartMenu;
        reset.resolved_path = Some(r"C:\Program Files\VideoLAN\VLC\vlc.exe".into());
        reset.launch_arguments = Some("--reset-config --reset-plugins-cache".into());
        apply_visibility(&mut reset);
        assert_eq!(reset.visibility_class, VisibilityClass::Auxiliary);

        let mut player = app("VLC media player", r"C:\Program Files\VideoLAN\VLC\vlc.exe");
        player.source_kind = SourceKind::Registry;
        player.can_uninstall = true;
        player.publisher = Some("VideoLAN".into());
        player.description = Some("VLC media player".into());
        apply_visibility(&mut player);
        assert_eq!(player.visibility_class, VisibilityClass::Primary);

        let resolved = resolve(vec![reset, player]);

        let card = resolved
            .iter()
            .find(|entry| entry.visibility_class == VisibilityClass::Primary)
            .expect("the player keeps a primary card");
        assert!(
            card.launch_arguments.is_none(),
            "the player card must not inherit the reset arguments: {:?}",
            card.launch_arguments
        );
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
    fn a_broad_registry_install_root_does_not_absorb_unrelated_executables() {
        let mut registry = app("Broad Suite", r"D:\broad-suite.exe");
        registry.source_kind = SourceKind::Registry;
        registry.install_location = Some(r"D:\".into());
        let mut unrelated = app("Unrelated Tool", r"D:\Tools\unrelated.exe");
        unrelated.source_kind = SourceKind::Portable;

        assert_eq!(resolve(vec![registry, unrelated]).len(), 2);
    }

    #[test]
    fn a_registry_install_root_still_claims_same_product_executables_beneath_it() {
        let mut registry = app("Prog", r"C:\Apps\Prog\prog.exe");
        registry.source_kind = SourceKind::Registry;
        registry.install_location = Some(r"C:\Apps\Prog".into());
        let mut nested = app("Prog", r"C:\Apps\Prog\bin\prog-helper.exe");
        nested.source_kind = SourceKind::Portable;

        assert_eq!(resolve(vec![registry, nested]).len(), 1);
    }

    #[test]
    fn registered_product_does_not_absorb_nested_different_product_component() {
        let mut registry = app(
            "Microsoft Visual Studio Code (User)",
            r"D:\Apps\Microsoft VS Code\Code.exe",
        );
        registry.source_kind = SourceKind::Registry;
        registry.install_location = Some(r"D:\Apps\Microsoft VS Code".into());
        registry.publisher = Some("Microsoft Corporation".into());

        let mut shortcut = app("Visual Studio Code", r"C:\Menu\Visual Studio Code.lnk");
        shortcut.source_kind = SourceKind::StartMenu;
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.resolved_path = Some(r"D:\Apps\Microsoft VS Code\Code.exe".into());

        let mut component = app(
            "Microsoft Execution Containers",
            r"D:\Apps\Microsoft VS Code\resources\mxc-sdk\wxc-windows-sandbox-guest.exe",
        );
        component.source_kind = SourceKind::Portable;
        crate::catalog::visibility::apply_visibility(&mut component);
        assert!(component
            .visibility_reasons
            .contains(&crate::catalog::VisibilityReason::ProductComponent));

        let merged = resolve(vec![registry, shortcut, component]);

        assert_eq!(merged.len(), 2);
        let code = merged
            .iter()
            .find(|app| app.name.contains("Visual Studio Code"))
            .unwrap();
        assert_eq!(
            code.visibility_class,
            crate::catalog::VisibilityClass::Primary
        );
        assert!(!code
            .visibility_reasons
            .contains(&crate::catalog::VisibilityReason::ProductComponent));
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
    fn real_url_shortcut_wins_over_url_backed_start_app() {
        let mut shortcut = app(
            "Node.js website",
            r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Node.js\Node.js website.url",
        );
        shortcut.artifact_kind = ArtifactKind::Documentation;
        shortcut.category = AppCategory::InstallersDocs;
        shortcut.launch_kind = LaunchKind::Shortcut;
        shortcut.source_kind = SourceKind::StartMenu;

        let mut start_app = app("Node.js website", "https://nodejs.org/");
        start_app.artifact_kind = ArtifactKind::Documentation;
        start_app.category = AppCategory::InstallersDocs;
        start_app.launch_kind = LaunchKind::AppUserModelId;
        start_app.source_kind = SourceKind::StartApps;
        start_app.resolved_path = Some("https://nodejs.org/".into());

        let merged = resolve(vec![start_app, shortcut]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source_kind, SourceKind::StartMenu);
        assert!(merged[0].path.ends_with("Node.js website.url"));
        assert_eq!(merged[0].artifact_kind, ArtifactKind::Documentation);
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

    // A display name is not an identity. Two unrelated packaged applications can ship the same
    // one, and the old rule scored *any* pair with an AUMID on either side and a matching display
    // family at 80 — the threshold that merges outright, before the publisher-conflict check runs.
    // Everything that distinguishes these two entries (package, publisher, target, install root)
    // was ignored, and the loser's launch and uninstall identity was lost with it.
    #[test]
    fn same_named_packages_from_different_publishers_stay_separate() {
        let mut first = app("Snap Camera", "Contoso.SnapCamera_9abc!App");
        first.launch_kind = LaunchKind::AppUserModelId;
        first.source_kind = SourceKind::StartApps;
        first.publisher = Some("Contoso".into());
        first.install_location =
            Some(r"C:\Program Files\WindowsApps\Contoso.SnapCamera_9abc".into());
        first.resolved_path =
            Some(r"C:\Program Files\WindowsApps\Contoso.SnapCamera_9abc\snap.exe".into());
        let mut second = app("Snap Camera", "Fabrikam.SnapCamera_1def!App");
        second.launch_kind = LaunchKind::AppUserModelId;
        second.source_kind = SourceKind::StartApps;
        second.publisher = Some("Fabrikam".into());
        second.install_location =
            Some(r"C:\Program Files\WindowsApps\Fabrikam.SnapCamera_1def".into());
        second.resolved_path =
            Some(r"C:\Program Files\WindowsApps\Fabrikam.SnapCamera_1def\camera.exe".into());

        let merged = resolve(vec![first.clone(), second.clone()]);

        assert_eq!(merged.len(), 2, "different packages are different cards");
        // Order invariance: the rule must not depend on which side is inspected first.
        assert_eq!(resolve(vec![second, first]).len(), 2);
    }

    // The narrower rule still has to recognise a genuine package: two entry points of one
    // installed package share the package-family part of their AUMID.
    #[test]
    fn two_entry_points_of_one_package_still_merge() {
        let mut main = app(
            "Windows Terminal",
            "Microsoft.WindowsTerminal_8wekyb3d8bbwe!App",
        );
        main.launch_kind = LaunchKind::AppUserModelId;
        main.source_kind = SourceKind::StartApps;
        let mut preview = app(
            "Windows Terminal",
            "Microsoft.WindowsTerminal_8wekyb3d8bbwe!OpenHere",
        );
        preview.launch_kind = LaunchKind::AppUserModelId;
        preview.source_kind = SourceKind::StartApps;

        assert_eq!(resolve(vec![main, preview]).len(), 1);
    }

    // Name similarity must not chain through a shared middle entry: A merging with B and B with C
    // does not make A and C the same application.
    #[test]
    fn a_shared_name_does_not_chain_unrelated_packages_together() {
        let mut left = app("Notes", "Contoso.Notes_9abc!App");
        left.launch_kind = LaunchKind::AppUserModelId;
        left.source_kind = SourceKind::StartApps;
        left.publisher = Some("Contoso".into());
        let mut right = app("Notes", "Fabrikam.Notes_1def!App");
        right.launch_kind = LaunchKind::AppUserModelId;
        right.source_kind = SourceKind::StartApps;
        right.publisher = Some("Fabrikam".into());
        let mut third = app("Notes", "Northwind.Notes_5ghi!App");
        third.launch_kind = LaunchKind::AppUserModelId;
        third.source_kind = SourceKind::StartApps;
        third.publisher = Some("Northwind".into());

        assert_eq!(resolve(vec![left, right, third]).len(), 3);
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
