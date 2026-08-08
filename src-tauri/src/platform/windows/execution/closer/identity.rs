use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CloseTarget {
    pub executable: PathBuf,
    pub install_root: Option<PathBuf>,
}

impl CloseTarget {
    pub(crate) fn new(executable: PathBuf, install_root: Option<PathBuf>) -> Self {
        Self {
            executable,
            install_root,
        }
    }

    pub(super) fn file_name(&self) -> Option<String> {
        file_name_of(&self.executable.to_string_lossy())
    }
}

static SHARED_CONTAINERS: &[&str] = &[
    r"\program files",
    r"\program files (x86)",
    r"\program files\windowsapps",
    r"\programdata",
    r"\windows",
    r"\users",
    r"\appdata",
    r"\appdata\local",
    r"\appdata\locallow",
    r"\appdata\roaming",
    r"\appdata\local\programs",
    r"\desktop",
    r"\downloads",
    r"\documents",
    r"\temp",
];

pub(super) fn is_instance_of(image: &str, target: &CloseTarget) -> bool {
    let image = normalize(image);
    if image == normalize(&target.executable.to_string_lossy()) {
        return true;
    }
    if file_name_of(&image) != target.file_name() {
        return false;
    }
    target
        .install_root
        .as_deref()
        .map(|root| normalize(&root.to_string_lossy()))
        .filter(|root| is_specific_enough(root))
        .is_some_and(|root| is_within(&image, &root))
}

fn normalize(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut previous_separator = false;
    for character in path.trim().trim_matches('"').chars() {
        let character = if character == '/' { '\\' } else { character };
        if character == '\\' {
            if previous_separator {
                continue;
            }
            previous_separator = true;
        } else {
            previous_separator = false;
        }
        result.push(character.to_ascii_lowercase());
    }
    result.trim_end_matches('\\').to_string()
}

fn file_name_of(path: &str) -> Option<String> {
    Path::new(&normalize(path))
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

fn is_specific_enough(root: &str) -> bool {
    let segments = root.split('\\').filter(|part| !part.is_empty()).count();
    if segments < 2 {
        return false;
    }
    let stripped = strip_versions(root);
    !SHARED_CONTAINERS
        .iter()
        .any(|container| stripped.ends_with(container))
}

fn is_within(image: &str, root: &str) -> bool {
    let image = strip_versions(image);
    let root = strip_versions(root);
    image
        .strip_prefix(&root)
        .is_some_and(|rest| rest.starts_with('\\'))
}

fn strip_versions(path: &str) -> String {
    path.split('\\')
        .map(strip_versions_from_segment)
        .collect::<Vec<_>>()
        .join("\\")
}

fn strip_versions_from_segment(segment: &str) -> String {
    let mut result = String::with_capacity(segment.len());
    let mut token = String::new();
    for character in segment.chars() {
        if character.is_ascii_digit() || character == '.' {
            token.push(character);
            continue;
        }
        result.push_str(&keep_unless_version(&token));
        token.clear();
        result.push(character);
    }
    result.push_str(&keep_unless_version(&token));
    result
}

fn keep_unless_version(token: &str) -> String {
    let is_version =
        token.contains(|character: char| character.is_ascii_digit()) && token.contains('.');
    if is_version {
        String::new()
    } else {
        token.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(executable: &str, install_root: &str) -> CloseTarget {
        CloseTarget::new(PathBuf::from(executable), Some(PathBuf::from(install_root)))
    }

    #[test]
    fn matches_the_same_executable_whatever_the_path_style() {
        let editor = target(
            r"C:\Program Files\Editor\editor.exe",
            r"C:\Program Files\Editor",
        );

        assert!(is_instance_of(
            r"c:\program files\editor\EDITOR.EXE",
            &editor
        ));
        assert!(is_instance_of(
            "C:/Program Files/Editor/editor.exe",
            &editor
        ));
        assert!(is_instance_of(
            r"C:\Program Files\\Editor\editor.exe",
            &editor
        ));
    }

    #[test]
    fn does_not_match_a_different_executable_in_the_same_folder() {
        let editor = target(
            r"C:\Program Files\Editor\editor.exe",
            r"C:\Program Files\Editor",
        );

        assert!(!is_instance_of(
            r"C:\Program Files\Editor\updater.exe",
            &editor
        ));
    }

    #[test]
    fn matches_a_packaged_application_across_an_update() {
        let chat = target(
            r"C:\Program Files\WindowsApps\OpenAI.Codex_26.730.8199.0_x64__2p2nqsd0c76g0\app\ChatGPT.exe",
            r"C:\Program Files\WindowsApps\OpenAI.Codex_26.730.8199.0_x64__2p2nqsd0c76g0",
        );

        assert!(is_instance_of(
            r"C:\Program Files\WindowsApps\OpenAI.Codex_26.803.5235.0_x64__2p2nqsd0c76g0\app\ChatGPT.exe",
            &chat
        ));
    }

    #[test]
    fn does_not_match_another_package_with_the_same_executable_name() {
        let chat = target(
            r"C:\Program Files\WindowsApps\OpenAI.Codex_26.730.8199.0_x64__2p2nqsd0c76g0\app\ChatGPT.exe",
            r"C:\Program Files\WindowsApps\OpenAI.Codex_26.730.8199.0_x64__2p2nqsd0c76g0",
        );

        assert!(!is_instance_of(
            r"C:\Program Files\WindowsApps\Contoso.Clone_1.0.0.0_x64__abcdefghijklm\app\ChatGPT.exe",
            &chat
        ));
    }

    #[test]
    fn matches_a_launcher_stub_against_the_versioned_executable_it_starts() {
        let claude = target(
            r"C:\Users\Example\AppData\Local\AnthropicClaude\claude.exe",
            r"C:\Users\Example\AppData\Local\AnthropicClaude",
        );

        assert!(is_instance_of(
            r"C:\Users\Example\AppData\Local\AnthropicClaude\app-1.26832.0\claude.exe",
            &claude
        ));
    }

    #[test]
    fn never_reaches_a_same_named_executable_outside_the_install_tree() {
        let claude = target(
            r"C:\Users\Example\AppData\Local\AnthropicClaude\claude.exe",
            r"C:\Users\Example\AppData\Local\AnthropicClaude",
        );

        for outsider in [
            r"C:\Users\Example\AppData\Roaming\Claude\claude-code\2.1.222\claude.exe",
            r"C:\Users\Example\.local\bin\claude.exe",
            r"D:\Portable\claude.exe",
        ] {
            assert!(!is_instance_of(outsider, &claude), "{outsider}");
        }
    }

    #[test]
    fn a_shared_container_never_widens_the_match() {
        for container in [
            r"C:\Program Files",
            r"C:\Program Files (x86)",
            r"C:\Program Files\WindowsApps",
            r"C:\Users\Example\AppData\Local",
            r"C:\Users\Example\AppData\Roaming",
            r"C:\Users\Example\Downloads",
            r"C:\Windows",
            r"C:\",
        ] {
            let loose = target(&format!(r"{container}\tool.exe"), container);

            assert!(
                !is_instance_of(&format!(r"{container}\Vendor\tool.exe"), &loose),
                "{container}"
            );
        }
    }

    #[test]
    fn without_an_install_root_only_the_exact_executable_matches() {
        let bare = CloseTarget::new(PathBuf::from(r"C:\Apps\Tool\tool.exe"), None);

        assert!(is_instance_of(r"C:\Apps\Tool\tool.exe", &bare));
        assert!(!is_instance_of(r"C:\Apps\Tool\v2\tool.exe", &bare));
    }

    #[test]
    fn covers_the_scenario_that_closed_only_one_of_three_applications() {
        let chat = target(
            r"C:\Program Files\WindowsApps\OpenAI.Codex_26.730.8199.0_x64__2p2nqsd0c76g0\app\ChatGPT.exe",
            r"C:\Program Files\WindowsApps\OpenAI.Codex_26.730.8199.0_x64__2p2nqsd0c76g0",
        );
        assert!(is_instance_of(
            r"C:\Program Files\WindowsApps\OpenAI.Codex_26.803.5235.0_x64__2p2nqsd0c76g0\app\ChatGPT.exe",
            &chat
        ));

        let claude = target(
            r"C:\Users\Example\AppData\Local\AnthropicClaude\claude.exe",
            r"C:\Users\Example\AppData\Local\AnthropicClaude",
        );
        assert!(is_instance_of(
            r"C:\Users\Example\AppData\Local\AnthropicClaude\app-1.26832.0\claude.exe",
            &claude
        ));
        assert!(!is_instance_of(
            r"C:\Users\Example\AppData\Roaming\Claude\claude-code\2.1.222\claude.exe",
            &claude
        ));

        let amnezia = target(
            r"C:\Program Files\AmneziaVPN\AmneziaVPN.exe",
            r"C:\\Program Files\AmneziaVPN",
        );
        assert!(is_instance_of(
            r"C:\Program Files\AmneziaVPN\AmneziaVPN.exe",
            &amnezia
        ));
    }

    #[test]
    fn a_version_is_a_run_of_digits_and_dots_not_any_digit() {
        assert_eq!(strip_versions_from_segment("app-1.26832.0"), "app-");
        assert_eq!(
            strip_versions_from_segment("OpenAI.Codex_26.803.5235.0_x64__2p2nqsd0c76g0"),
            "OpenAI.Codex__x64__2p2nqsd0c76g0"
        );
        assert_eq!(strip_versions_from_segment("AmneziaVPN"), "AmneziaVPN");
        assert_eq!(strip_versions_from_segment("7-Zip"), "7-Zip");
        assert_eq!(strip_versions_from_segment("x64"), "x64");
    }
}
