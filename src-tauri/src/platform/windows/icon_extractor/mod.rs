mod app_id;
mod gdi;
mod shell;

pub(crate) use app_id::extract_app_id_icon;
pub(crate) use shell::{extract_icon, is_provably_not_this_files_icon};

#[cfg(test)]
pub(crate) use shell::unknown_type_icon;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::ImageFormat;
use std::ffi::OsStr;
use std::io::Cursor;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

pub(super) const MAX_ICON_DIMENSION: u32 = 4096;
const MAX_ICON_ALLOCATION: u64 = 64 * 1024 * 1024;

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

pub(super) fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbaImage};

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
