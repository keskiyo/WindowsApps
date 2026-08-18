use super::gdi::encode_hicon;
use super::wide;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::mem::size_of;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::UI::Shell::{
    SHGetFileInfoW, SHFILEINFOW, SHGFI_FLAGS, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_USEFILEATTRIBUTES,
};

pub(crate) fn extract_icon(path: &Path) -> Option<String> {
    let icon = shell_icon(path.as_os_str(), SHGFI_ICON | SHGFI_LARGEICON)?;
    if is_generic_answer_about_an_unreadable_file(
        &icon,
        unknown_type_icon().as_deref(),
        class_icon_for(path).as_deref(),
        cannot_be_read(path),
    ) {
        return None;
    }
    Some(icon)
}

pub(crate) fn is_provably_not_this_files_icon(path: &Path, icon: &str) -> bool {
    is_not_this_files_icon(
        icon,
        unknown_type_icon().as_deref(),
        class_icon_for(path).as_deref(),
    )
}

pub(crate) fn unknown_type_icon() -> Option<String> {
    class_icon("")
}

fn is_generic_answer_about_an_unreadable_file(
    icon: &str,
    unknown: Option<&str>,
    class: Option<&str>,
    unreadable: bool,
) -> bool {
    unreadable && (unknown == Some(icon) || class == Some(icon))
}

fn is_not_this_files_icon(icon: &str, unknown: Option<&str>, class: Option<&str>) -> bool {
    let (Some(unknown), Some(class)) = (unknown, class) else {
        return false;
    };
    icon == unknown && class != unknown
}

fn cannot_be_read(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && File::open(path).is_err())
}

fn class_icon_for(path: &Path) -> Option<String> {
    class_icon(
        &path
            .extension()
            .map(|extension| extension.to_string_lossy().to_lowercase())
            .unwrap_or_default(),
    )
}

fn class_icon(extension: &str) -> Option<String> {
    static ICONS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    let icons = ICONS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(icon) = icons.lock().ok()?.get(extension) {
        return Some(icon.clone());
    }
    let separator = if extension.is_empty() { "" } else { "." };
    let icon = shell_icon(
        OsStr::new(&format!("windowsapps-class-probe{separator}{extension}")),
        SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES,
    )?;
    icons
        .lock()
        .ok()?
        .insert(extension.to_owned(), icon.clone());
    Some(icon)
}

fn shell_icon(name: &OsStr, flags: SHGFI_FLAGS) -> Option<String> {
    crate::platform::windows::com::ensure_initialized();
    let wide = wide(name);
    let mut file_info = SHFILEINFOW::default();
    // SAFETY: `wide` is a NUL-terminated UTF-16 buffer alive for the call; `file_info` is a live,
    // exclusively borrowed, fully initialized `SHFILEINFOW` whose declared size matches the value
    // passed, so the callee cannot write past it. COM is initialized above, which `SHGetFileInfo`
    // requires because shell icon handlers are COM objects.
    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut file_info),
            size_of::<SHFILEINFOW>() as u32,
            flags,
        )
    };
    if result == 0 || file_info.hIcon.0.is_null() {
        return None;
    }
    // SAFETY: `SHGFI_ICON` transfers ownership of `hIcon` to us, and it is non-null (checked
    // above). `encode_hicon` only reads it. `DestroyIcon` then runs on every path out of this
    // function — including the `None` encoding result — so the icon is released exactly once.
    let encoded = unsafe { encode_hicon(file_info.hIcon) };
    // SAFETY: as above; the handle is still ours and has not been destroyed yet.
    let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::DestroyIcon(file_info.hIcon) };
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNKNOWN: &str = "data:image/png;base64,unknown-file-type";
    const CLASS: &str = "data:image/png;base64,the-generic-application-icon";
    const OWN: &str = "data:image/png;base64,the-editors-own-icon";

    #[test]
    fn refuses_the_unknown_type_icon_where_the_extension_has_its_own() {
        assert!(is_not_this_files_icon(UNKNOWN, Some(UNKNOWN), Some(CLASS)));
    }

    #[test]
    fn keeps_the_unknown_type_icon_where_the_extension_has_no_association() {
        assert!(!is_not_this_files_icon(
            UNKNOWN,
            Some(UNKNOWN),
            Some(UNKNOWN)
        ));
    }

    #[test]
    fn keeps_an_executables_own_icon() {
        assert!(!is_not_this_files_icon(OWN, Some(UNKNOWN), Some(CLASS)));
    }

    #[test]
    fn rejects_nothing_when_a_reference_is_unavailable() {
        assert!(!is_not_this_files_icon(UNKNOWN, None, Some(CLASS)));
        assert!(!is_not_this_files_icon(UNKNOWN, Some(UNKNOWN), None));
    }

    #[test]
    fn refuses_any_generic_answer_about_a_file_that_will_not_open() {
        for icon in [UNKNOWN, CLASS] {
            assert!(is_generic_answer_about_an_unreadable_file(
                icon,
                Some(UNKNOWN),
                Some(CLASS),
                true,
            ));
        }
    }

    #[test]
    fn keeps_a_generic_answer_about_a_readable_file() {
        assert!(!is_generic_answer_about_an_unreadable_file(
            CLASS,
            Some(UNKNOWN),
            Some(CLASS),
            false,
        ));
    }

    #[test]
    fn keeps_a_specific_icon_even_from_a_file_that_will_not_open() {
        assert!(!is_generic_answer_about_an_unreadable_file(
            OWN,
            Some(UNKNOWN),
            Some(CLASS),
            true,
        ));
    }

    #[test]
    fn a_readable_file_is_not_reported_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("Editor.exe");
        std::fs::write(&executable, b"binary").unwrap();

        assert!(!cannot_be_read(&executable));
    }

    #[test]
    fn only_a_file_that_exists_and_will_not_open_counts_as_unreadable() {
        let dir = tempfile::tempdir().unwrap();

        assert!(!cannot_be_read(&dir.path().join("gone.exe")));
        assert!(!cannot_be_read(dir.path()));
    }
}
