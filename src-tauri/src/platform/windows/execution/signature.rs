use serde::{Deserialize, Serialize};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, HWND, TRUST_E_NOSIGNATURE};
use windows::Win32::Security::WinTrust::{
    WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0,
    WINTRUST_FILE_INFO, WTD_CACHE_ONLY_URL_RETRIEVAL, WTD_CHOICE_FILE, WTD_REVOKE_NONE,
    WTD_STATEACTION_IGNORE, WTD_UI_NONE,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AppSignatureStatus {
    Verified,
    Unsigned,
    #[default]
    Unavailable,
}

fn map_verify_status(status: i32) -> AppSignatureStatus {
    if status == 0 {
        AppSignatureStatus::Verified
    } else if status == TRUST_E_NOSIGNATURE.0 {
        AppSignatureStatus::Unsigned
    } else {
        AppSignatureStatus::Unavailable
    }
}

/// Checks Authenticode trust with the locally available policy and revocation data. The command
/// never shows UI or downloads data, so an unreachable provider cannot block a catalog request.
pub(crate) fn verify_signature(path: &Path) -> AppSignatureStatus {
    if !path.is_file() {
        return AppSignatureStatus::Unavailable;
    }

    let path_wide = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let mut file_info = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: PCWSTR(path_wide.as_ptr()),
        hFile: HANDLE::default(),
        pgKnownSubject: std::ptr::null_mut(),
    };
    let mut trust_data = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 {
            pFile: &mut file_info,
        },
        dwStateAction: WTD_STATEACTION_IGNORE,
        dwProvFlags: WTD_CACHE_ONLY_URL_RETRIEVAL,
        ..WINTRUST_DATA::default()
    };
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    // SAFETY: every pointer refers to a live stack value for the entire synchronous call:
    // `path_wide` is NUL-terminated, `file_info` retains its pointer, and `trust_data` retains
    // its pointer to `file_info`. `WTD_STATEACTION_IGNORE` creates no state handle requiring a
    // later close, while `WTD_UI_NONE` prevents the provider from interacting with the desktop.
    let status = unsafe {
        WinVerifyTrust(
            HWND::default(),
            &mut action,
            (&mut trust_data as *mut WINTRUST_DATA).cast(),
        )
    };
    map_verify_status(status)
}

#[cfg(test)]
mod tests {
    use super::{map_verify_status, AppSignatureStatus};
    use windows::Win32::Foundation::TRUST_E_NOSIGNATURE;

    #[test]
    fn distinguishes_unsigned_files_from_other_verification_failures() {
        assert_eq!(map_verify_status(0), AppSignatureStatus::Verified);
        assert_eq!(
            map_verify_status(TRUST_E_NOSIGNATURE.0),
            AppSignatureStatus::Unsigned
        );
        assert_eq!(map_verify_status(-1), AppSignatureStatus::Unavailable);
        assert_eq!(map_verify_status(1), AppSignatureStatus::Unavailable);
    }
}
