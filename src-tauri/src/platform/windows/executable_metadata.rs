use std::ffi::{c_void, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ExecutableMetadata {
    pub description: Option<String>,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub product_name: Option<String>,
}

pub fn read(path: &Path) -> ExecutableMetadata {
    let path_wide = wide(path.as_os_str());
    let size = unsafe { GetFileVersionInfoSizeW(PCWSTR(path_wide.as_ptr()), None) };
    if size == 0 {
        return ExecutableMetadata::default();
    }
    let mut data = vec![0_u8; size as usize];
    if unsafe {
        GetFileVersionInfoW(
            PCWSTR(path_wide.as_ptr()),
            None,
            size,
            data.as_mut_ptr().cast(),
        )
    }
    .is_err()
    {
        return ExecutableMetadata::default();
    }
    let (language, code_page) = translation(&data).unwrap_or((0x0409, 0x04b0));
    ExecutableMetadata {
        description: string_value(&data, language, code_page, "FileDescription"),
        version: string_value(&data, language, code_page, "ProductVersion"),
        publisher: string_value(&data, language, code_page, "CompanyName"),
        product_name: string_value(&data, language, code_page, "ProductName"),
    }
}

pub fn fill_missing(
    description: &mut Option<String>,
    version: &mut Option<String>,
    publisher: &mut Option<String>,
    metadata: &ExecutableMetadata,
) {
    if description.is_none() {
        *description = metadata.description.clone();
    }
    if version.is_none() {
        *version = metadata.version.clone();
    }
    if publisher.is_none() {
        *publisher = metadata.publisher.clone();
    }
}

fn translation(data: &[u8]) -> Option<(u16, u16)> {
    let query = wide(OsStr::new(r"\VarFileInfo\Translation"));
    let mut buffer: *mut c_void = std::ptr::null_mut();
    let mut length = 0;
    let found = unsafe {
        VerQueryValueW(
            data.as_ptr().cast(),
            PCWSTR(query.as_ptr()),
            &mut buffer,
            &mut length,
        )
    };
    if !found.as_bool() || buffer.is_null() || length < 4 {
        return None;
    }
    let values = unsafe { std::slice::from_raw_parts(buffer.cast::<u16>(), 2) };
    Some((values[0], values[1]))
}

fn string_value(data: &[u8], language: u16, code_page: u16, key: &str) -> Option<String> {
    let query = format!(r"\StringFileInfo\{language:04x}{code_page:04x}\{key}");
    let query_wide = wide(OsStr::new(&query));
    let mut buffer: *mut c_void = std::ptr::null_mut();
    let mut length = 0;
    let found = unsafe {
        VerQueryValueW(
            data.as_ptr().cast(),
            PCWSTR(query_wide.as_ptr()),
            &mut buffer,
            &mut length,
        )
    };
    if !found.as_bool() || buffer.is_null() || length == 0 {
        return None;
    }
    let value = unsafe { std::slice::from_raw_parts(buffer.cast::<u16>(), length as usize) };
    let end = value.iter().position(|character| *character == 0).unwrap_or(value.len());
    let value = String::from_utf16_lossy(&value[..end]).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::{fill_missing, ExecutableMetadata};

    #[test]
    fn fills_only_missing_metadata_fields() {
        let mut description = Some("Registry description".to_string());
        let mut version = None;
        let mut publisher = None;
        fill_missing(
            &mut description,
            &mut version,
            &mut publisher,
            &ExecutableMetadata {
                description: Some("File description".into()),
                version: Some("2.14.0".into()),
                publisher: Some("Happ".into()),
                product_name: Some("Happ".into()),
            },
        );
        assert_eq!(description.as_deref(), Some("Registry description"));
        assert_eq!(version.as_deref(), Some("2.14.0"));
        assert_eq!(publisher.as_deref(), Some("Happ"));
    }
}
