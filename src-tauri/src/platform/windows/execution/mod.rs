pub(crate) mod exec_target;
pub(crate) mod executable_metadata;
pub(crate) mod folder;
pub(crate) mod launcher;
pub(crate) mod pe;
pub(crate) mod signature;

pub(crate) use folder::open_trusted_folder;
pub(crate) use pe::{is_console_subsystem, read_architecture, AppArchitecture};
pub(crate) use signature::{verify_signature, AppSignatureStatus};
