use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::core::{Interface, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

#[derive(Debug, Default)]
pub(crate) struct ShortcutDetails {
    pub target: Option<PathBuf>,
    pub icon_location: Option<PathBuf>,
    #[allow(dead_code)]
    pub arguments: Option<String>,
}

pub(crate) fn resolve(path: &Path) -> ShortcutDetails {
    crate::platform::windows::com::ensure_initialized();
    // SAFETY: `resolve_inner`'s only precondition is a COM apartment on the calling thread, which
    // the line above establishes and which lives as long as the thread does. A shortcut that
    // cannot be read is an ordinary `Err`, not undefined behaviour, so the default is safe.
    unsafe { resolve_inner(path) }.unwrap_or_default()
}

unsafe fn resolve_inner(path: &Path) -> windows::core::Result<ShortcutDetails> {
    // SAFETY: the caller guarantees an initialized apartment. `CoCreateInstance` is given the
    // static `ShellLink` CLSID, no aggregation, and an in-process context; the returned interface
    // is reference-counted by `IShellLinkW`, which releases it on drop, so no handle is leaked on
    // any early return below.
    let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)? };
    let persist: IPersistFile = link.cast()?;
    let path_wide = wide(path.as_os_str());
    // SAFETY: `path_wide` is a NUL-terminated UTF-16 buffer owned by this frame and still alive
    // for the whole call. `Load` only reads it. A missing or malformed `.lnk` returns an error
    // HRESULT, which `?` propagates before any buffer below is read.
    unsafe { persist.Load(PCWSTR(path_wide.as_ptr()), STGM_READ)? };

    BUFFERS.with(|buffers| {
        let buffers = &mut *buffers.borrow_mut();
        for buffer in [
            &mut buffers.target,
            &mut buffers.icon,
            &mut buffers.arguments,
        ] {
            buffer.fill(0);
        }
        // SAFETY: each call writes at most the length of the slice it is handed — the `windows`
        // bindings derive the count from the slice itself, so the buffer cannot be overrun, and
        // all three were just zero-filled so a short write leaves a NUL terminator behind rather
        // than a previous shortcut's bytes. The `RefCell` borrow is exclusive for the whole
        // block, so no other code can observe or alias the buffers mid-write. The null
        // `WIN32_FIND_DATAW` argument is documented as "do not return find data".
        let _ = unsafe { link.GetPath(&mut buffers.target, std::ptr::null_mut(), 0) };
        let mut icon_index = 0;
        // SAFETY: as above; `icon_index` is a live local the callee writes through exclusively.
        let _ = unsafe { link.GetIconLocation(&mut buffers.icon, &mut icon_index) };
        // SAFETY: as above.
        let _ = unsafe { link.GetArguments(&mut buffers.arguments) };
        Ok(ShortcutDetails {
            target: path_from_buffer(&buffers.target),
            icon_location: path_from_buffer(&buffers.icon)
                .map(|path| PathBuf::from(path.to_string_lossy().replace('/', r"\"))),
            arguments: string_from_buffer(&buffers.arguments),
        })
    })
}

const BUFFER_LENGTH: usize = 32768;

struct ShortcutBuffers {
    target: Vec<u16>,
    icon: Vec<u16>,
    arguments: Vec<u16>,
}

thread_local! {
    static BUFFERS: std::cell::RefCell<ShortcutBuffers> = std::cell::RefCell::new(ShortcutBuffers {
        target: vec![0_u16; BUFFER_LENGTH],
        icon: vec![0_u16; BUFFER_LENGTH],
        arguments: vec![0_u16; BUFFER_LENGTH],
    });
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
