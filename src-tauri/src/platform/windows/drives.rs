use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};

const DRIVE_FIXED_TYPE: u32 = 3;

pub(crate) fn fixed_drive_roots() -> Vec<PathBuf> {
    // SAFETY: `GetLogicalDrives` takes no arguments and reads no caller memory. It returns 0 on
    // failure, which the bit test below treats as "no drives" rather than as an error.
    let mask = unsafe { GetLogicalDrives() };
    let drives = (0..26).filter_map(|index| {
        if mask & (1 << index) == 0 {
            return None;
        }
        let letter = char::from(b'A' + index as u8);
        let root = format!(r"{letter}:\");
        let wide = root.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        // SAFETY: `PCWSTR` must point at a NUL-terminated UTF-16 string that outlives the call.
        // `wide` is built here with an explicit trailing NUL, is not moved or reallocated between
        // its construction and this call, and stays alive for the rest of the iteration body.
        let kind = unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) };
        Some((letter, kind))
    });
    roots_for_types(drives)
}

fn roots_for_types(drives: impl IntoIterator<Item = (char, u32)>) -> Vec<PathBuf> {
    drives
        .into_iter()
        .filter(|(_, kind)| *kind == DRIVE_FIXED_TYPE)
        .map(|(letter, _)| PathBuf::from(format!(r"{letter}:\")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_drive_discovery_returns_only_rooted_paths() {
        assert!(fixed_drive_roots()
            .iter()
            .all(|path| path.parent().is_none()));
    }

    #[test]
    fn keeps_fixed_drives_and_rejects_removable_and_network_drives() {
        assert_eq!(
            roots_for_types([('C', 3), ('D', 2), ('E', 3), ('Z', 4)]),
            vec![PathBuf::from(r"C:\"), PathBuf::from(r"E:\")],
        );
    }
}
