use std::path::Path;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CloseRisk {
    Safe,
    Session,
    Critical,
}

impl CloseRisk {
    pub(crate) fn id(self) -> Option<&'static str> {
        match self {
            Self::Safe => None,
            Self::Session => Some("close.session"),
            Self::Critical => Some("close.critical"),
        }
    }
}

static CRITICAL: &[&str] = &[
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "winlogon.exe",
    "services.exe",
    "lsass.exe",
    "lsaiso.exe",
    "svchost.exe",
];

static SESSION: &[&str] = &[
    "explorer.exe",
    "dwm.exe",
    "sihost.exe",
    "ctfmon.exe",
    "taskhostw.exe",
    "startmenuexperiencehost.exe",
    "shellexperiencehost.exe",
    "searchhost.exe",
    "searchapp.exe",
    "textinputhost.exe",
    "applicationframehost.exe",
    "runtimebroker.exe",
    "dllhost.exe",
    "rundll32.exe",
    "conhost.exe",
];

pub(crate) fn close_risk(path: &Path) -> CloseRisk {
    let Some(name) = file_name(path) else {
        return CloseRisk::Safe;
    };
    if CRITICAL.contains(&name.as_str()) {
        return CloseRisk::Critical;
    }
    if SESSION.contains(&name.as_str()) || is_this_launcher(path) {
        return CloseRisk::Session;
    }
    CloseRisk::Safe
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
}

fn is_this_launcher(path: &Path) -> bool {
    static OWN: OnceLock<Option<String>> = OnceLock::new();
    let own = OWN.get_or_init(|| std::env::current_exe().ok().as_deref().and_then(file_name));
    own.as_deref() == file_name(path).as_deref()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn terminating_these_ends_windows() {
        for name in CRITICAL {
            let path = PathBuf::from(r"C:\Windows\System32").join(name);
            assert_eq!(close_risk(&path), CloseRisk::Critical, "{name}");
        }
    }

    #[test]
    fn terminating_these_ends_the_desktop_session() {
        for name in SESSION {
            let path = PathBuf::from(r"C:\Windows").join(name);
            assert_eq!(close_risk(&path), CloseRisk::Session, "{name}");
        }
    }

    #[test]
    fn an_ordinary_application_is_safe_to_close() {
        for path in [
            r"C:\Program Files\Editor\editor.exe",
            r"C:\Windows\System32\notepad.exe",
            r"C:\Windows\System32\mspaint.exe",
            r"D:\Games\game.exe",
        ] {
            assert_eq!(close_risk(Path::new(path)), CloseRisk::Safe, "{path}");
        }
    }

    #[test]
    fn the_verdict_does_not_depend_on_how_the_path_is_written() {
        for path in [
            r"C:\Windows\explorer.exe",
            r"c:\windows\EXPLORER.EXE",
            r"C:/Windows/Explorer.Exe",
        ] {
            assert_eq!(close_risk(Path::new(path)), CloseRisk::Session, "{path}");
        }
    }

    #[test]
    fn closing_the_launcher_from_its_own_scenario_is_a_session_risk() {
        let own = std::env::current_exe().expect("the test binary has a path");

        assert_eq!(close_risk(&own), CloseRisk::Session);
    }

    #[test]
    fn a_path_with_no_file_name_is_not_a_target() {
        assert_eq!(close_risk(Path::new(r"C:\")), CloseRisk::Safe);
    }

    #[test]
    fn every_risk_above_safe_carries_a_distinct_stable_identifier() {
        assert_eq!(CloseRisk::Safe.id(), None);
        assert_eq!(CloseRisk::Session.id(), Some("close.session"));
        assert_eq!(CloseRisk::Critical.id(), Some("close.critical"));
    }

    #[test]
    fn no_name_is_listed_in_both_tiers() {
        for name in SESSION {
            assert!(!CRITICAL.contains(name), "{name}");
        }
    }
}
