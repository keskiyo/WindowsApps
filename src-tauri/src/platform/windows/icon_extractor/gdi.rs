use super::MAX_ICON_DIMENSION;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::{DynamicImage, ImageFormat, RgbaImage};
use std::io::Cursor;
use std::mem::{size_of, zeroed};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

pub(super) unsafe fn encode_hicon(
    icon: windows::Win32::UI::WindowsAndMessaging::HICON,
) -> Option<String> {
    // SAFETY: `ICONINFO` is a plain-old-data struct of handles and integers, so an all-zero value
    // is a valid initialized instance (null handles, which the checks below expect).
    let mut info: ICONINFO = unsafe { zeroed() };
    // SAFETY: `icon` is live by the caller's contract; `info` is a live, exclusively borrowed,
    // initialized struct the callee fills. On success it hands us two GDI bitmaps, which
    // `cleanup_icon_info` deletes on every path below; on failure `?` returns before either
    // handle is read, and the callee leaves them null.
    unsafe { GetIconInfo(icon, &mut info).ok()? };

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

pub(super) unsafe fn encode_hbitmap(
    bitmap_handle: windows::Win32::Graphics::Gdi::HBITMAP,
) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_bgra_to_rgba() {
        let mut pixels = vec![10, 20, 30, 255, 1, 2, 3, 4];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![30, 20, 10, 255, 3, 2, 1, 4]);
    }
}
