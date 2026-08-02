pub(crate) mod autostart;
pub(crate) mod change_watcher;
pub(crate) mod com;
pub(crate) mod drives;
mod execution;
pub(crate) mod icon_extractor;
pub(crate) mod known_folders;
mod locale;
mod registry;
mod shortcuts;
mod uninstall;

pub(crate) use execution::{
    exec_target, executable_metadata, is_console_subsystem, launcher, open_trusted_folder,
    read_architecture, verify_signature, AppArchitecture, AppSignatureStatus,
};
pub(crate) use locale::{os_ui_script, NameScript};
pub(crate) use registry::{
    install_registry, registered_targets, steam_registry, uninstall_registry,
};
pub(crate) use shortcuts::{global_shortcut, shortcut};
pub(crate) use uninstall::{uninstall_history, uninstaller};
