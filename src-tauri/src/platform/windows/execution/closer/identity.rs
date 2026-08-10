use std::path::PathBuf;

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

fn is_specific_enough(root: &str) -> bool {
    let segments = root.split('\\').filter(|part| !part.is_empty()).count();
    if segments < 3 {
        return false;
    }
    let stripped = strip_versions(root);
    if is_windows_directory(&stripped) {
        return false;
    }
    !SHARED_CONTAINERS
        .iter()
        .any(|container| stripped.ends_with(container))
}

fn is_windows_directory(path: &str) -> bool {
    path.split('\\')
        .filter(|part| !part.is_empty())
        .nth(1)
        .is_some_and(|part| part == "windows")
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
    fn does_not_match_a_different_executable_outside_the_installation_root() {
        let editor = target(
            r"C:\Program Files\Editor\editor.exe",
            r"C:\Program Files\Editor",
        );

        assert!(!is_instance_of(
            r"C:\Program Files\Other\updater.exe",
            &editor
        ));
    }

    #[test]
    fn matches_different_processes_inside_a_dedicated_installation_root() {
        let battle_net = target(
            r"D:\Games\Battle.net\Battle.net Launcher.exe",
            r"D:\Games\Battle.net",
        );

        assert!(is_instance_of(
            r"D:\Games\Battle.net\Battle.net.exe",
            &battle_net
        ));
        assert!(is_instance_of(
            r"D:\Games\Battle.net\Agent.exe",
            &battle_net
        ));
    }

    #[test]
    fn does_not_expand_a_shared_installation_root() {
        let launcher = target(r"C:\Program Files\Vendor\launcher.exe", r"C:\Program Files");

        assert!(!is_instance_of(
            r"C:\Program Files\Other\other.exe",
            &launcher
        ));
    }

    #[test]
    fn does_not_expand_a_two_level_installation_root() {
        let launcher = target(r"D:\Games\launcher.exe", r"D:\Games");

        assert!(!is_instance_of(r"D:\Games\Other\other.exe", &launcher));
    }

    #[test]
    fn does_not_expand_a_windows_subdirectory() {
        let command_prompt = target(r"C:\Windows\System32\cmd.exe", r"C:\Windows\System32");

        assert!(!is_instance_of(
            r"C:\Windows\System32\lsass.exe",
            &command_prompt
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
