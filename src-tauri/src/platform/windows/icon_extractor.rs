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
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    FOLDERID_AppsFolder, IShellItemImageFactory, SHCreateItemFromIDList, SHCreateItemInKnownFolder,
    SHGetFileInfoW, SHParseDisplayName, KF_FLAG_DEFAULT, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON,
    SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

/// Largest icon we are willing to decode. Steam library art is far below this; the bound exists
/// so a crafted or corrupt file cannot make the decoder allocate unbounded memory.
const MAX_ICON_DIMENSION: u32 = 4096;
const MAX_ICON_ALLOCATION: u64 = 64 * 1024 * 1024;

/// Decode an on-disk image (JPG/PNG/ICO) and re-encode it as a PNG data URL.
/// Used for Steam library-cache icons, which are not embedded in an executable.
///
/// The source directory is writable by any process running as the user, so the file is
/// untrusted: decoding runs under explicit limits. Without them a "decompression bomb" — a
/// few kilobytes declaring enormous dimensions — would allocate gigabytes and take the
/// application down during an ordinary scan.
pub(crate) fn image_file_to_png_data_url(path: &Path) -> Option<String> {
    let mut reader = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_ICON_DIMENSION);
    limits.max_image_height = Some(MAX_ICON_DIMENSION);
    limits.max_alloc = Some(MAX_ICON_ALLOCATION);
    reader.limits(limits);
    let image = reader.decode().ok()?;
    let mut png = Cursor::new(Vec::new());
    image.write_to(&mut png, ImageFormat::Png).ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(png.into_inner())
    ))
}

pub(crate) fn extract_icon(path: &Path) -> Option<String> {
    // `SHGetFileInfo` documents COM as a prerequisite; shell icon handlers are COM objects.
    super::com::ensure_initialized();
    let wide = wide(path.as_os_str());
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
            SHGFI_ICON | SHGFI_LARGEICON,
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
    let _ = unsafe { DestroyIcon(file_info.hIcon) };
    encoded
}

pub(crate) fn extract_app_id_icon(app_id: &str) -> Option<String> {
    super::com::ensure_initialized();
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

/// # Safety
///
/// The calling thread must have an initialized COM apartment.
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

/// # Safety
///
/// `icon` must be a live `HICON` the caller owns for the duration of the call.
unsafe fn encode_hicon(icon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<String> {
    // SAFETY: `ICONINFO` is a plain-old-data struct of handles and integers, so an all-zero value
    // is a valid initialized instance (null handles, which the checks below expect).
    let mut info: ICONINFO = unsafe { zeroed() };
    // SAFETY: `icon` is live by the caller's contract; `info` is a live, exclusively borrowed,
    // initialized struct the callee fills. On success it hands us two GDI bitmaps, which
    // `cleanup_icon_info` deletes on every path below; on failure `?` returns before either
    // handle is read, and the callee leaves them null.
    unsafe { GetIconInfo(icon, &mut info).ok()? };

    // Only the colour bitmap is a plain top-down image. `hbmMask` is a 1bpp, double-height
    // AND+XOR mask; feeding it to the 32bpp path produced a black-and-white image of twice the
    // height. There is no icon we can honestly render from it, so skip it — the card shows its
    // placeholder instead of garbage.
    let encoded = if info.hbmColor.0.is_null() {
        None
    } else {
        // SAFETY: `hbmColor` is non-null (checked) and owned by us until `cleanup_icon_info`
        // below, so it is live for the whole call.
        unsafe { encode_hbitmap(info.hbmColor) }
    };
    cleanup_icon_info(&info);
    encoded
}

/// # Safety
///
/// `bitmap_handle` must be a live GDI bitmap the caller owns for the duration of the call.
unsafe fn encode_hbitmap(bitmap_handle: windows::Win32::Graphics::Gdi::HBITMAP) -> Option<String> {
    // SAFETY: `BITMAP` is plain-old-data, so an all-zero value is a valid initialized instance;
    // every field is overwritten by `GetObjectW` before being read.
    let mut bitmap: BITMAP = unsafe { zeroed() };
    // SAFETY: the handle is live by the caller's contract. The out-pointer refers to the live
    // local above and the declared size matches its type exactly, so the callee cannot write past
    // it. A zero return means nothing was written, which is checked before any field is used.
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
    // Dimensions come from a bitmap the shell produced, so they are not ours to trust. The
    // product was computed in `u32` and wrapped: a 32768×32768 bitmap yielded a zero-length
    // buffer that `GetDIBits` then filled with `height` rows — a heap overflow in release
    // builds, where overflow wraps instead of panicking.
    if width > MAX_ICON_DIMENSION || height > MAX_ICON_DIMENSION {
        return None;
    }
    let byte_count = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))?;
    let mut pixels = vec![0_u8; byte_count];
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
    // SAFETY: `CreateCompatibleDC(None)` creates a memory DC compatible with the screen and takes
    // no caller memory. The handle is ours and is deleted below on every path out of the call.
    let dc = unsafe { CreateCompatibleDC(None) };
    // SAFETY: `dc` and `bitmap_handle` are both live here. `bitmap_info` describes exactly the
    // buffer being written: 32 bits per pixel, `width` columns, `height` rows (negative height
    // for top-down), and `pixels` was allocated as `width * height * 4` bytes with checked
    // multiplication and a dimension cap, so the callee cannot write past its end. Both
    // out-parameters are live, exclusively borrowed locals.
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
    // SAFETY: `dc` was created above, is still ours, and is not used after this point — so this
    // is the single matching delete, and it happens before the `copied == 0` early return.
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

/// # Safety
///
/// `info` must have been filled by a successful `GetIconInfo` whose bitmaps have not been deleted
/// yet; it transfers both bitmaps to the caller and this releases them.
unsafe fn cleanup_icon_info(info: &ICONINFO) {
    if !info.hbmColor.0.is_null() {
        // SAFETY: non-null by the check, owned by us through `GetIconInfo`, and deleted once —
        // `encode_hicon` calls this on exactly one path and does not use the bitmaps afterwards.
        let _ = unsafe { DeleteObject(HGDIOBJ(info.hbmColor.0)) };
    }
    if !info.hbmMask.0.is_null() {
        // SAFETY: as above for the mask bitmap.
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

    // The Steam library cache is writable by any process running as the user, so an oversized
    // image must be refused rather than decoded into memory.
    #[test]
    fn refuses_images_beyond_the_decode_limits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("huge.png");
        let oversized = MAX_ICON_DIMENSION + 1;
        let image = RgbaImage::from_pixel(oversized, 1, image::Rgba([0, 0, 0, 255]));
        DynamicImage::ImageRgba8(image)
            .save_with_format(&path, ImageFormat::Png)
            .unwrap();

        assert_eq!(image_file_to_png_data_url(&path), None);
    }

    #[test]
    fn decodes_an_image_within_the_limits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("small.png");
        let image = RgbaImage::from_pixel(32, 32, image::Rgba([0, 128, 255, 255]));
        DynamicImage::ImageRgba8(image)
            .save_with_format(&path, ImageFormat::Png)
            .unwrap();

        assert!(image_file_to_png_data_url(&path)
            .is_some_and(|url| url.starts_with("data:image/png;base64,")));
    }

    #[test]
    fn decodes_ico_file_to_png_data_url() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.ico");
        let pixel = RgbaImage::from_pixel(16, 16, image::Rgba([255, 0, 0, 255]));
        DynamicImage::ImageRgba8(pixel)
            .save_with_format(&path, ImageFormat::Ico)
            .unwrap();

        let url = image_file_to_png_data_url(&path).unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
    }
}
