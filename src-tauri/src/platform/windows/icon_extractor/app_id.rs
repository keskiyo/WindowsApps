use super::gdi::encode_hbitmap;
use super::wide;
use std::ffi::OsStr;
use windows::core::PCWSTR;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{DeleteObject, HGDIOBJ};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    FOLDERID_AppsFolder, IShellItemImageFactory, SHCreateItemFromIDList, SHCreateItemInKnownFolder,
    SHParseDisplayName, KF_FLAG_DEFAULT, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
};

pub(crate) fn extract_app_id_icon(app_id: &str) -> Option<String> {
    crate::platform::windows::com::ensure_initialized();
    extract_app_id_icon_inner(app_id).ok().flatten()
}

fn extract_app_id_icon_inner(app_id: &str) -> windows::core::Result<Option<String>> {
    let wide = wide(OsStr::new(app_id));
    // SAFETY: `extract_app_id_icon` initializes COM before calling, which every shell interface
    // here requires. `wide` is NUL-terminated and alive for the whole block. `FOLDERID_AppsFolder`
    // is a static GUID. The factory is a reference-counted interface released on drop. `GetImage`
    // returns an `HBITMAP` that becomes ours: `?` on it returns before there is a bitmap to leak,
    // and once it exists `DeleteObject` runs on both the `Some` and `None` encoding results, so
    // the GDI object is freed exactly once.
    unsafe {
        let factory: IShellItemImageFactory =
            SHCreateItemInKnownFolder(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, PCWSTR(wide.as_ptr()))
                .or_else(|_| apps_folder_factory(app_id))?;
        let bitmap = factory.GetImage(
            SIZE { cx: 64, cy: 64 },
            SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK,
        )?;
        let value = encode_hbitmap(bitmap);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        Ok(value)
    }
}

unsafe fn apps_folder_factory(app_id: &str) -> windows::core::Result<IShellItemImageFactory> {
    let display_name = apps_folder_shell_name(app_id);
    let display_name_wide = wide(OsStr::new(&display_name));
    let mut pidl = std::ptr::null_mut();
    // SAFETY: the caller guarantees an apartment. `display_name_wide` is NUL-terminated and alive
    // for the call; `pidl` is a live local the callee writes through. On failure `?` returns
    // before `pidl` is read, and the callee leaves it untouched — there is nothing to free.
    unsafe {
        SHParseDisplayName(PCWSTR(display_name_wide.as_ptr()), None, &mut pidl, 0, None)?;
    }
    // SAFETY: `pidl` was just written by a successful `SHParseDisplayName`, so it is a valid
    // ID list; `SHCreateItemFromIDList` only reads it.
    let item = unsafe { SHCreateItemFromIDList::<IShellItemImageFactory>(pidl) };
    // SAFETY: `SHParseDisplayName` allocates the ID list with the COM task allocator and transfers
    // ownership, so this is the matching free. It runs whether or not the item was created, and
    // `pidl` is not used afterwards.
    unsafe { CoTaskMemFree(Some(pidl.cast())) };
    item
}

fn apps_folder_shell_name(app_id: &str) -> String {
    format!(r"shell:AppsFolder\{app_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_apps_folder_shell_name() {
        assert_eq!(
            apps_folder_shell_name("Microsoft.WindowsCamera_8wekyb3d8bbwe!App"),
            r"shell:AppsFolder\Microsoft.WindowsCamera_8wekyb3d8bbwe!App",
        );
    }
}
