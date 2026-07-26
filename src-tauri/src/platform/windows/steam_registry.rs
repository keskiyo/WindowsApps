use std::path::PathBuf;
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

pub(crate) fn install_root() -> Option<PathBuf> {
    let steam = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Valve\Steam")
        .ok()?;
    let path = steam.get_value::<String, _>("SteamPath").ok()?;
    Some(PathBuf::from(path.replace('/', r"\")))
}
