mod cache;
mod read;
mod target;

pub(crate) use cache::{read_cached_details, retain_cached_details};
pub(crate) use read::read_details;
pub(crate) use target::{can_open_folder, details_fingerprint, folder_target, AppDetailsTarget};
