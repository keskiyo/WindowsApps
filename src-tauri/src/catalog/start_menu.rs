use super::{
    artifact, filters, machine, make_app, AppCategory, AppInfo, ArtifactKind, LaunchKind,
    ScanControl, SourceKind, StageBudget, StageStop, DEFAULT_STAGE_TIMEOUT, START_MENU_MAX_DEPTH,
    START_MENU_MAX_ENTRIES,
};
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub(crate) struct StartMenuScan {
    pub apps: Vec<AppInfo>,
    pub stop: Option<StageStop>,
    pub complete: bool,
}

struct StartMenuWalk {
    apps: Vec<AppInfo>,
    complete: bool,
}

pub(super) fn scan_start_menu(control: &ScanControl) -> StartMenuScan {
    let mut roots = vec![PathBuf::from(
        r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs",
    )];
    if let Some(appdata) = env::var_os("APPDATA") {
        roots.push(PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    let budget = control.stage_with(
        DEFAULT_STAGE_TIMEOUT,
        START_MENU_MAX_ENTRIES,
        START_MENU_MAX_DEPTH,
    );
    if budget.should_stop() {
        return StartMenuScan {
            apps: Vec::new(),
            stop: budget.stop(),
            complete: false,
        };
    }
    let walked = walk_start_menu_shortcuts(roots, &budget, &machine::MachineFacts::current());

    StartMenuScan {
        apps: walked.apps,
        stop: budget.stop(),
        complete: walked.complete,
    }
}

fn walk_start_menu_shortcuts(
    roots: Vec<PathBuf>,
    budget: &StageBudget,
    facts: &machine::MachineFacts,
) -> StartMenuWalk {
    let mut complete = true;
    let apps = roots
        .into_iter()
        .flat_map(|root| {
            WalkDir::new(root)
                .follow_links(false)
                .max_depth(budget.max_depth())
                .into_iter()
        })
        .take_while(|_| budget.charge_entry())
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry),
            Err(_) => {
                complete = false;
                None
            }
        })
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry.path().extension().is_some_and(|extension| {
                extension.eq_ignore_ascii_case("lnk") || extension.eq_ignore_ascii_case("url")
            })
        })
        .filter_map(|entry| {
            let path = entry.into_path();
            let name = path.file_stem()?.to_string_lossy().trim().to_string();
            if name.is_empty() {
                return None;
            }
            let is_url = path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("url"));
            let details = if is_url {
                Default::default()
            } else {
                crate::platform::windows::shortcut::resolve(&path)
            };
            let target = details
                .target
                .as_ref()
                .map(|value| value.to_string_lossy().into_owned());
            if !is_url
                && filters::is_maintenance_entry(&name, &path.to_string_lossy(), target.as_deref())
            {
                return None;
            }
            let mut app = make_app(name, path);
            app.source_kind = SourceKind::StartMenu;
            if is_url {
                app.launch_kind = LaunchKind::Shortcut;
            }
            app.resolved_path = target;
            app.shortcut_icon_path = details
                .icon_location
                .map(|value| value.to_string_lossy().into_owned());
            app.launch_arguments = details.arguments;
            if let Some(target) = app.resolved_path.as_deref() {
                let metadata =
                    crate::platform::windows::executable_metadata::read(Path::new(target));
                let internal_name = metadata.internal_name.clone();
                app.product_name = metadata.product_name;
                app.original_filename = metadata.original_filename;
                app.description = metadata.description;
                app.version = metadata.version;
                app.publisher = metadata.publisher;
                app.artifact_kind = artifact::classify(&app, internal_name.as_deref(), facts);
            }
            if app.artifact_kind == ArtifactKind::Application {
                app.artifact_kind = artifact::classify(&app, None, facts);
            }
            if app.artifact_kind != ArtifactKind::Application {
                app.category = AppCategory::InstallersDocs;
            }
            (!is_url || app.artifact_kind == ArtifactKind::Documentation).then_some(app)
        })
        .collect();
    StartMenuWalk { apps, complete }
}

#[cfg(test)]
mod tests {
    use super::super::{
        machine, ArtifactKind, ScanControl, StageBudget, StageStop, DEFAULT_STAGE_TIMEOUT,
        START_MENU_MAX_DEPTH, START_MENU_MAX_ENTRIES,
    };
    use super::AppInfo;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    fn facts() -> machine::MachineFacts {
        machine::MachineFacts::empty()
    }

    fn walk_start_menu_shortcuts(roots: Vec<PathBuf>, budget: &StageBudget) -> Vec<AppInfo> {
        super::walk_start_menu_shortcuts(roots, budget, &facts()).apps
    }

    fn nested_shortcuts(root: &Path, folders: usize) {
        let mut current = root.to_path_buf();
        for index in 0..folders {
            current = current.join(format!("Level {index}"));
            std::fs::create_dir_all(&current).unwrap();
            std::fs::write(current.join(format!("Tool {index}.lnk")), []).unwrap();
        }
    }

    #[test]
    fn an_unreadable_start_menu_root_is_not_an_empty_success() {
        let dir = tempfile::tempdir().unwrap();
        let never = || false;
        let control = ScanControl::new(&never);
        let budget = control.stage_with(DEFAULT_STAGE_TIMEOUT, usize::MAX, START_MENU_MAX_DEPTH);

        let scan =
            super::walk_start_menu_shortcuts(vec![dir.path().join("missing")], &budget, &facts());

        assert!(scan.apps.is_empty());
        assert!(!scan.complete);
        assert_eq!(budget.stop(), None);
    }

    #[test]
    fn the_start_menu_traversal_stops_at_its_depth_limit() {
        let dir = tempfile::tempdir().unwrap();
        nested_shortcuts(dir.path(), 6);
        let never = || false;
        let control = ScanControl::new(&never);
        let budget = control.stage_with(DEFAULT_STAGE_TIMEOUT, usize::MAX, 2);

        let apps = walk_start_menu_shortcuts(vec![dir.path().to_path_buf()], &budget);

        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Tool 0");
        assert_eq!(budget.stop(), None);
    }

    #[test]
    fn the_start_menu_traversal_stops_at_its_entry_limit() {
        let dir = tempfile::tempdir().unwrap();
        nested_shortcuts(dir.path(), 6);
        let never = || false;
        let control = ScanControl::new(&never);
        let budget = control.stage_with(DEFAULT_STAGE_TIMEOUT, 3, START_MENU_MAX_DEPTH);

        let apps = walk_start_menu_shortcuts(vec![dir.path().to_path_buf()], &budget);

        assert!(apps.len() < 6);
        assert_eq!(budget.stop(), Some(StageStop::EntryLimit));
    }

    #[test]
    fn the_start_menu_traversal_stops_at_its_deadline() {
        let dir = tempfile::tempdir().unwrap();
        nested_shortcuts(dir.path(), 6);
        let never = || false;
        let control = ScanControl::new(&never);
        let budget = control.stage_with(Duration::ZERO, usize::MAX, START_MENU_MAX_DEPTH);

        let apps = walk_start_menu_shortcuts(vec![dir.path().to_path_buf()], &budget);

        assert!(apps.is_empty());
        assert_eq!(budget.stop(), Some(StageStop::TimedOut));
    }

    #[test]
    fn the_start_menu_traversal_stops_when_the_scan_is_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        nested_shortcuts(dir.path(), 6);
        let cancelled = || true;
        let control = ScanControl::new(&cancelled);
        let budget = control.stage_with(DEFAULT_STAGE_TIMEOUT, usize::MAX, START_MENU_MAX_DEPTH);

        let apps = walk_start_menu_shortcuts(vec![dir.path().to_path_buf()], &budget);

        assert!(apps.is_empty());
        assert_eq!(budget.stop(), Some(StageStop::Cancelled));
    }

    #[test]
    fn an_ordinary_start_menu_tree_is_walked_completely() {
        let dir = tempfile::tempdir().unwrap();
        nested_shortcuts(dir.path(), 4);
        let never = || false;
        let control = ScanControl::new(&never);
        let budget = control.stage_with(
            DEFAULT_STAGE_TIMEOUT,
            START_MENU_MAX_ENTRIES,
            START_MENU_MAX_DEPTH,
        );

        let apps = walk_start_menu_shortcuts(vec![dir.path().to_path_buf()], &budget);

        assert_eq!(apps.len(), 4);
        assert_eq!(budget.stop(), None);
    }

    #[test]
    fn start_menu_url_documentation_is_discovered_without_scanning_arbitrary_docs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Node.js website.url"),
            b"[InternetShortcut]",
        )
        .unwrap();
        std::fs::write(dir.path().join("Unrelated.url"), b"[InternetShortcut]").unwrap();
        std::fs::write(dir.path().join("Manual.pdf"), b"fixture").unwrap();
        let never = || false;
        let control = ScanControl::new(&never);
        let budget = control.stage_with(DEFAULT_STAGE_TIMEOUT, 50, 4);

        let apps = walk_start_menu_shortcuts(vec![dir.path().to_path_buf()], &budget);

        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Node.js website");
        assert_eq!(apps[0].artifact_kind, ArtifactKind::Documentation);
    }
}
