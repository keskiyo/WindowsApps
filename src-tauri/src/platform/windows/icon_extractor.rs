use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::{DynamicImage, ImageFormat, RgbaImage};
use std::ffi::OsStr;
use std::io::Cursor;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::System::Com::{
    CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{
    FOLDERID_AppsFolder, IShellItemImageFactory, SHCreateItemFromIDList, SHCreateItemInKnownFolder,
    SHGetFileInfoW, SHParseDisplayName, KF_FLAG_DEFAULT, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON,
    SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

/// Decode an on-disk image (JPG/PNG/ICO) and re-encode it as a PNG data URL.
/// Used for Steam library-cache icons, which are not embedded in an executable.
pub fn image_file_to_png_data_url(path: &Path) -> Option<String> {
    let image = image::open(path).ok()?;
    let mut png = Cursor::new(Vec::new());
    image.write_to(&mut png, ImageFormat::Png).ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(png.into_inner())
    ))
}

pub fn extract_icon(path: &Path) -> Option<String> {
    let wide = wide(path.as_os_str());
    let mut file_info = SHFILEINFOW::default();
    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut file_info),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if result == 0 || file_info.hIcon.0.is_null() {
        return None;
    }
    let encoded = unsafe { encode_hicon(file_info.hIcon) };
    let _ = unsafe { DestroyIcon(file_info.hIcon) };
    encoded
}

pub fn extract_app_id_icon(app_id: &str) -> Option<String> {
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() };
    let encoded = extract_app_id_icon_inner(app_id).ok().flatten();
    if initialized {
        unsafe { CoUninitialize() };
    }
    encoded
}

fn extract_app_id_icon_inner(app_id: &str) -> windows::core::Result<Option<String>> {
    let wide = wide(OsStr::new(app_id));
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
    unsafe {
        SHParseDisplayName(PCWSTR(display_name_wide.as_ptr()), None, &mut pidl, 0, None)?;
    }
    let item = unsafe { SHCreateItemFromIDList::<IShellItemImageFactory>(pidl) };
    unsafe { CoTaskMemFree(Some(pidl.cast())) };
    item
}

fn apps_folder_shell_name(app_id: &str) -> String {
    format!(r"shell:AppsFolder\{app_id}")
}

unsafe fn encode_hicon(icon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<String> {
    let mut info: ICONINFO = unsafe { zeroed() };
    unsafe { GetIconInfo(icon, &mut info).ok()? };

    let bitmap_handle = if !info.hbmColor.0.is_null() {
        info.hbmColor
    } else {
        info.hbmMask
    };
    let encoded = unsafe { encode_hbitmap(bitmap_handle) };
    cleanup_icon_info(&info);
    encoded
}

unsafe fn encode_hbitmap(bitmap_handle: windows::Win32::Graphics::Gdi::HBITMAP) -> Option<String> {
    let mut bitmap: BITMAP = unsafe { zeroed() };
    let object_size = unsafe {
        GetObjectW(
            HGDIOBJ(bitmap_handle.0),
            size_of::<BITMAP>() as i32,
            Some((&mut bitmap as *mut BITMAP).cast()),
        )
    };
    if object_size == 0 || bitmap.bmWidth <= 0 || bitmap.bmHeight <= 0 {
        return None;
    }

    let width = bitmap.bmWidth as u32;
    let height = bitmap.bmHeight.unsigned_abs();
    let mut pixels = vec![0_u8; (width * height * 4) as usize];
    let mut bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let dc = unsafe { CreateCompatibleDC(None) };
    let copied = unsafe {
        GetDIBits(
            dc,
            bitmap_handle,
            0,
            height,
            Some(pixels.as_mut_ptr().cast()),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        )
    };
    let _ = unsafe { DeleteDC(dc) };
    if copied == 0 {
        return None;
    }

    bgra_to_rgba(&mut pixels);
    let image = RgbaImage::from_raw(width, height, pixels)?;
    let mut png = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut png, ImageFormat::Png)
        .ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(png.into_inner())
    ))
}

fn bgra_to_rgba(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
}

unsafe fn cleanup_icon_info(info: &ICONINFO) {
    if !info.hbmColor.0.is_null() {
        let _ = unsafe { DeleteObject(HGDIOBJ(info.hbmColor.0)) };
    }
    if !info.hbmMask.0.is_null() {
        let _ = unsafe { DeleteObject(HGDIOBJ(info.hbmMask.0)) };
    }
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_bgra_to_rgba() {
        let mut pixels = vec![10, 20, 30, 255, 1, 2, 3, 4];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![30, 20, 10, 255, 3, 2, 1, 4]);
    }

    #[test]
    fn builds_apps_folder_shell_name() {
        assert_eq!(
            apps_folder_shell_name("Microsoft.WindowsCamera_8wekyb3d8bbwe!App"),
            r"shell:AppsFolder\Microsoft.WindowsCamera_8wekyb3d8bbwe!App",
        );
    }
}
