use super::{AppInfo, LaunchKind, SourceKind, VisibilityClass, VisibilityReason};
use std::path::Path;

pub(crate) fn attach_close_risk(apps: &mut [AppInfo]) {
    for app in apps {
        app.close_risk = match close_target_of(app) {
            Some(target) => crate::platform::windows::close_risk(Path::new(&target))
                .id()
                .map(str::to_owned),
            None => Some("close.not_closable".to_owned()),
        };
    }
}

pub(crate) fn close_scope_of(app: &AppInfo) -> Option<String> {
    let target = close_target_of(app);
    if target.as_deref().is_some_and(is_steam_client_target) {
        return None;
    }
    target
        .as_deref()
        .and_then(|target| {
            Path::new(target)
                .parent()
                .map(|parent| parent.to_string_lossy().into_owned())
        })
        .or_else(|| {
            app.install_location
                .as_deref()
                .map(str::trim)
                .filter(|location| !location.is_empty())
                .map(str::to_owned)
        })
}

fn is_steam_client_target(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("steam.exe"))
}

pub(crate) fn close_target_of(app: &AppInfo) -> Option<String> {
    let path = app.resolved_path.clone().or_else(|| {
        matches!(
            app.launch_kind,
            LaunchKind::Executable | LaunchKind::Shortcut
        )
        .then(|| app.path.clone())
    })?;
    Path::new(&path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        .then_some(path)
}

fn is_plain_windows_powershell(app: &AppInfo, target: &Path) -> bool {
    if app.source_kind != SourceKind::StartMenu
        || app.launch_kind != LaunchKind::Shortcut
        || app
            .launch_arguments
            .as_deref()
            .is_some_and(|arguments| !arguments.trim().is_empty())
    {
        return false;
    }
    target
        .file_name()
        .and_then(|file| file.to_str())
        .is_some_and(|file| file.eq_ignore_ascii_case("powershell.exe"))
}

pub(crate) fn demote_console_applications(apps: &mut [AppInfo]) {
    for app in apps.iter_mut() {
        if app.visibility_class != VisibilityClass::Primary
            || app.launch_kind == LaunchKind::AppUserModelId
        {
            continue;
        }
        let target = app.resolved_path.as_deref().unwrap_or(&app.path);
        let path = Path::new(target);
        if is_plain_windows_powershell(app, path) {
            continue;
        }
        if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
            && crate::platform::windows::is_console_subsystem(path)
        {
            app.visibility_class = VisibilityClass::Auxiliary;
            if !app
                .visibility_reasons
                .contains(&VisibilityReason::ConsoleApplication)
            {
                app.visibility_reasons
                    .push(VisibilityReason::ConsoleApplication);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{attach_close_risk, close_scope_of, demote_console_applications};
    use crate::app_state::cached_app as app;
    use crate::catalog::{LaunchKind, SourceKind, VisibilityClass, VisibilityReason};

    fn minimal_pe(subsystem: u16) -> Vec<u8> {
        let mut buffer = vec![0_u8; 64];
        buffer[0] = b'M';
        buffer[1] = b'Z';
        buffer[60..64].copy_from_slice(&64_u32.to_le_bytes());
        buffer.extend_from_slice(b"PE\0\0");
        buffer.extend_from_slice(&[0_u8; 20]);
        buffer.extend_from_slice(&[0_u8; 68]);
        buffer.extend_from_slice(&subsystem.to_le_bytes());
        buffer
    }

    #[test]
    fn each_record_carries_the_risk_of_closing_it() {
        let mut explorer = app("Проводник", r"C:\Menu\Проводник.lnk");
        explorer.launch_kind = LaunchKind::Shortcut;
        explorer.resolved_path = Some(r"C:\Windows\explorer.exe".into());
        let mut security = app("Local Security Authority", r"C:\Windows\System32\lsass.exe");
        security.launch_kind = LaunchKind::Executable;
        let mut editor = app("Editor", r"C:\Editor\editor.exe");
        editor.launch_kind = LaunchKind::Executable;
        let mut packaged = app("Camera", "Microsoft.WindowsCamera_8wek!App");
        packaged.launch_kind = LaunchKind::AppUserModelId;
        let mut apps = vec![explorer, security, editor, packaged];

        attach_close_risk(&mut apps);

        assert_eq!(apps[0].close_risk.as_deref(), Some("close.session"));
        assert_eq!(apps[1].close_risk.as_deref(), Some("close.critical"));
        assert_eq!(apps[2].close_risk, None);
        assert_eq!(apps[3].close_risk.as_deref(), Some("close.not_closable"));
    }

    #[test]
    fn a_shell_location_reports_that_there_is_nothing_to_close() {
        let mut explorer = app("Проводник", "Microsoft.Windows.Explorer");
        explorer.launch_kind = LaunchKind::AppUserModelId;
        explorer.resolved_path = Some("::{52205FD8-5DFB-447D-801A-D0B52F2E83E1}".into());
        let mut apps = vec![explorer];

        attach_close_risk(&mut apps);

        assert_eq!(apps[0].close_risk.as_deref(), Some("close.not_closable"));
    }

    #[test]
    fn close_scope_uses_the_executable_parent_over_a_shared_vendor_root() {
        let mut entry = app(
            "Product A",
            r"C:\Program Files\Vendor\Product A\launcher.exe",
        );
        entry.install_location = Some(r"C:\Program Files\Vendor".into());

        assert_eq!(
            close_scope_of(&entry).as_deref(),
            Some(r"C:\Program Files\Vendor\Product A")
        );
    }

    #[test]
    fn steam_client_does_not_expand_close_scope_to_installed_games() {
        let steam = app("Steam", r"C:\Program Files (x86)\Steam\steam.exe");

        assert_eq!(close_scope_of(&steam), None);
    }

    #[test]
    fn console_executables_are_demoted_to_auxiliary() {
        let dir = tempfile::tempdir().unwrap();
        let cli = dir.path().join("7z.exe");
        std::fs::write(&cli, minimal_pe(3)).unwrap();
        let gui = dir.path().join("game.exe");
        std::fs::write(&gui, minimal_pe(2)).unwrap();

        let mut console = app("7-Zip Console", &cli.to_string_lossy());
        console.source_kind = SourceKind::Portable;
        let mut window = app("Game", &gui.to_string_lossy());
        window.source_kind = SourceKind::Portable;

        let mut apps = vec![console, window];
        demote_console_applications(&mut apps);

        assert_eq!(apps[0].visibility_class, VisibilityClass::Auxiliary);
        assert!(apps[0]
            .visibility_reasons
            .contains(&VisibilityReason::ConsoleApplication));
        assert_eq!(apps[1].visibility_class, VisibilityClass::Primary);
    }

    #[test]
    fn command_prompt_is_auxiliary_but_plain_powershell_stays_primary() {
        let dir = tempfile::tempdir().unwrap();
        let cmd = dir.path().join("cmd.exe");
        let powershell = dir.path().join("powershell.exe");
        std::fs::write(&cmd, minimal_pe(3)).unwrap();
        std::fs::write(&powershell, minimal_pe(3)).unwrap();

        let mut command_prompt = app("Command Prompt", r"C:\Menu\Command Prompt.lnk");
        command_prompt.source_kind = SourceKind::StartMenu;
        command_prompt.launch_kind = LaunchKind::Shortcut;
        command_prompt.resolved_path = Some(cmd.to_string_lossy().into_owned());
        let mut windows_powershell = app("Windows PowerShell", r"C:\Menu\Windows PowerShell.lnk");
        windows_powershell.source_kind = SourceKind::StartMenu;
        windows_powershell.launch_kind = LaunchKind::Shortcut;
        windows_powershell.resolved_path = Some(powershell.to_string_lossy().into_owned());
        let mut developer_prompt = app("Developer Command Prompt", r"C:\Menu\Developer Prompt.lnk");
        developer_prompt.source_kind = SourceKind::StartMenu;
        developer_prompt.launch_kind = LaunchKind::Shortcut;
        developer_prompt.resolved_path = Some(cmd.to_string_lossy().into_owned());
        developer_prompt.launch_arguments = Some("/k setup.bat".into());

        let mut apps = vec![command_prompt, windows_powershell, developer_prompt];
        demote_console_applications(&mut apps);

        assert_eq!(apps[0].visibility_class, VisibilityClass::Auxiliary);
        assert!(apps[0]
            .visibility_reasons
            .contains(&VisibilityReason::ConsoleApplication));
        assert_eq!(apps[1].visibility_class, VisibilityClass::Primary);
        assert_eq!(apps[2].visibility_class, VisibilityClass::Auxiliary);
        assert!(apps[2]
            .visibility_reasons
            .contains(&VisibilityReason::ConsoleApplication));
    }
}
