use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::core::{Interface, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED, STGM_READ,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

#[derive(Debug, Default)]
pub struct ShortcutDetails {
    pub target: Option<PathBuf>,
    pub icon_location: Option<PathBuf>,
    /// Command-line arguments stored in the shortcut. A non-empty value usually marks a
    /// "command" shortcut (e.g. `pg_ctl ... reload`) rather than a plain application.
    /// Read for the upcoming command-like filtering diagnostics; not consumed yet.
    #[allow(dead_code)]
    pub arguments: Option<String>,
}

pub fn resolve(path: &Path) -> ShortcutDetails {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() };
    let details = unsafe { resolve_inner(path) }.unwrap_or_default();
    if initialized {
        unsafe { CoUninitialize() };
    }
    details
}

unsafe fn resolve_inner(path: &Path) -> windows::core::Result<ShortcutDetails> {
    let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)? };
    let persist: IPersistFile = link.cast()?;
    let path_wide = wide(path.as_os_str());
    unsafe { persist.Load(PCWSTR(path_wide.as_ptr()), STGM_READ)? };

    let mut target = vec![0_u16; 32768];
    let _ = unsafe { link.GetPath(&mut target, std::ptr::null_mut(), 0) };
    let mut icon = vec![0_u16; 32768];
    let mut icon_index = 0;
    let _ = unsafe { link.GetIconLocation(&mut icon, &mut icon_index) };
    let mut arguments = vec![0_u16; 32768];
    let _ = unsafe { link.GetArguments(&mut arguments) };
    Ok(ShortcutDetails {
        target: path_from_buffer(&target),
        // Installers write icon locations with forward slashes and 8.3 names
        // (C:/PROGRA~1/...). Normalize separators so cache keys stay consistent.
        icon_location: path_from_buffer(&icon)
            .map(|path| PathBuf::from(path.to_string_lossy().replace('/', r"\"))),
        arguments: string_from_buffer(&arguments),
    })
}

fn path_from_buffer(buffer: &[u16]) -> Option<PathBuf> {
    string_from_buffer(buffer).map(PathBuf::from)
}

fn string_from_buffer(buffer: &[u16]) -> Option<String> {
    let end = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    let value = String::from_utf16_lossy(&buffer[..end]).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}
