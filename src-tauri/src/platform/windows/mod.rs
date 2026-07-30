pub(crate) mod autostart;
pub(crate) mod change_watcher;
pub(crate) mod com;
pub(crate) mod drives;
mod execution;
pub(crate) mod icon_extractor;
mod locale;
mod registry;
mod shortcuts;
mod uninstall;

pub(crate) use execution::{exec_target, executable_metadata, launcher};
pub(crate) use locale::{os_ui_script, NameScript};
pub(crate) use registry::{install_registry, steam_registry, uninstall_registry};
pub(crate) use shortcuts::{global_shortcut, shortcut};
pub(crate) use uninstall::{uninstall_history, uninstaller};
