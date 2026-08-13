use std::ffi::OsStr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

#[derive(Debug, PartialEq, Eq)]
enum TrayAction {
    Open,
    Quit,
}

fn tray_action(id: &str) -> Option<TrayAction> {
    match id {
        "open" => Some(TrayAction::Open),
        "quit" => Some(TrayAction::Quit),
        _ => None,
    }
}

#[derive(Default)]
pub(crate) struct LifecycleState {
    quitting: AtomicBool,
}

impl LifecycleState {
    pub(crate) fn mark_quitting(&self) {
        self.quitting.store(true, Ordering::SeqCst);
    }

    pub(crate) fn should_hide_on_close(&self) -> bool {
        !self.quitting.load(Ordering::SeqCst)
    }
}

pub(crate) fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub(crate) fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

pub(crate) fn starts_hidden_from_autostart<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    args.into_iter()
        .any(|argument| argument.as_ref() == OsStr::new("--autostart"))
}

pub(crate) fn should_hide_on_autostart(autostart: bool, tray_ready: bool) -> bool {
    autostart && tray_ready
}

pub(crate) fn should_show_on_second_instance<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    !starts_hidden_from_autostart(args)
}

pub(crate) fn setup_tray(app: &AppHandle, state: Arc<LifecycleState>) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Windows Apps", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let icon = app.default_window_icon().cloned();
    let mut builder = TrayIconBuilder::with_id("windows-apps")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Windows Apps")
        .on_menu_event(move |app, event| match tray_action(event.id().as_ref()) {
            Some(TrayAction::Open) => show_main_window(app),
            Some(TrayAction::Quit) => {
                state.mark_quitting();
                app.exit(0);
            }
            None => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = icon {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_close_hides_the_window() {
        let state = LifecycleState::default();
        assert!(state.should_hide_on_close());
    }

    #[test]
    fn explicit_quit_disables_close_interception() {
        let state = LifecycleState::default();
        state.mark_quitting();
        assert!(!state.should_hide_on_close());
    }

    #[test]
    fn tray_menu_ids_map_to_explicit_actions() {
        assert_eq!(tray_action("open"), Some(TrayAction::Open));
        assert_eq!(tray_action("quit"), Some(TrayAction::Quit));
        assert_eq!(tray_action("unknown"), None);
    }

    #[test]
    fn autostart_mode_requires_an_exact_argument() {
        assert!(starts_hidden_from_autostart(["app.exe", "--autostart"]));
        assert!(!starts_hidden_from_autostart(["app.exe"]));
        assert!(!starts_hidden_from_autostart([
            "app.exe",
            "--autostart-extra",
        ]));
    }

    #[test]
    fn autostart_hides_only_when_the_tray_is_ready() {
        assert!(should_hide_on_autostart(true, true));
        assert!(!should_hide_on_autostart(true, false));
        assert!(!should_hide_on_autostart(false, true));
    }

    #[test]
    fn second_autostart_invocation_keeps_the_existing_window_hidden() {
        assert!(!should_show_on_second_instance(["app.exe", "--autostart"]));
        assert!(should_show_on_second_instance(["app.exe"]));
    }
}
