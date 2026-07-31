use std::ffi::{c_void, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ExecutableMetadata {
    pub description: Option<String>,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub product_name: Option<String>,
    pub original_filename: Option<String>,
}

/// Whether an executable targets the console subsystem — a command-line tool such as `7z.exe` or
/// `git.exe` — rather than the GUI subsystem. Read straight from the PE optional header: no
/// metadata guessing, no launching, so it generalizes to any executable (including ones nobody
/// can test by hand). Any read/parse problem answers `false`.
/// Ceiling for a PE version resource. A real VS_VERSIONINFO block with every translation is a few
/// kilobytes. The size comes from the file itself, which is untrusted input for this purpose, and
/// it drives the allocation directly — so it is bounded before the buffer exists rather than after.
/// A file above the cap simply reports no metadata.
const MAX_VERSION_RESOURCE_BYTES: u32 = 4 * 1024 * 1024;

pub(crate) fn read(path: &Path) -> ExecutableMetadata {
    let path_wide = wide(path.as_os_str());
    // SAFETY: `path_wide` is a NUL-terminated UTF-16 buffer owned by this frame and alive for
    // both calls below. The reserved handle argument is `None`. A file with no version resource
    // answers 0, which is checked before anything is allocated.
    let size = unsafe { GetFileVersionInfoSizeW(PCWSTR(path_wide.as_ptr()), None) };
    if size == 0 || size > MAX_VERSION_RESOURCE_BYTES {
        return ExecutableMetadata::default();
    }
    let mut data = vec![0_u8; size as usize];
    // SAFETY: `data` was allocated with exactly `size` bytes — the length the same API just
    // reported and the length passed here — so the callee cannot write past its end. The pointer
    // comes from a live, uniquely borrowed `Vec` that is not moved for the rest of this function.
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
    // Try every translation the file declares, not just the first. An executable with only a
    // non-English block used to lose ProductName/CompanyName/FileDescription entirely, because
    // the lookup was hard-coded to the first pair and fell back to US English — a query into a
    // translation the file does not contain. These fields feed deduplication and visibility, so
    // the loss changed classification. US English stays as a last resort.
    let translations = candidate_translations(translations(&data));
    let value = |key: &str| string_value(&data, &translations, key);
    ExecutableMetadata {
        description: value("FileDescription"),
        version: pick_version(value("ProductVersion"), value("FileVersion")),
        publisher: value("CompanyName"),
        product_name: value("ProductName"),
        original_filename: value("OriginalFilename"),
    }
}

/// ProductVersion is the marketing version and the primary source; FileVersion is the numeric PE
/// version, used only when ProductVersion is absent (some executables carry only FileVersion).
fn pick_version(product_version: Option<String>, file_version: Option<String>) -> Option<String> {
    product_version.or(file_version)
}

/// The translations to query, in order: everything the file declares, then US English as a last
/// resort (so a file with no translation block still gets the historical behaviour).
fn candidate_translations(mut declared: Vec<(u16, u16)>) -> Vec<(u16, u16)> {
    const US_ENGLISH: (u16, u16) = (0x0409, 0x04b0);
    if !declared.contains(&US_ENGLISH) {
        declared.push(US_ENGLISH);
    }
    declared
}

fn translations(data: &[u8]) -> Vec<(u16, u16)> {
    let query = wide(OsStr::new(r"\VarFileInfo\Translation"));
    let mut buffer: *mut c_void = std::ptr::null_mut();
    let mut length = 0;
    // SAFETY: `data` holds a version-info block `GetFileVersionInfoW` filled in, which is what
    // `VerQueryValueW` requires; `query` is a NUL-terminated UTF-16 buffer alive for the call.
    // `buffer` and `length` are live locals the callee writes through. It borrows `data` — it
    // returns an interior pointer rather than an allocation — so nothing here is freed, and
    // `data` outlives every use of `buffer` below.
    let found = unsafe {
        VerQueryValueW(
            data.as_ptr().cast(),
            PCWSTR(query.as_ptr()),
            &mut buffer,
            &mut length,
        )
    };
    // The block is an array of (language, code page) u16 pairs; `length` is in bytes.
    if !found.as_bool() || buffer.is_null() || length < 4 {
        return Vec::new();
    }
    // Same alignment concern as `string_value_for`: the block lives in a `Vec<u8>`. Read each
    // `u16` from its bytes rather than reinterpreting a `u8` pointer as `*const u16`.
    let pair_count = length as usize / 4;
    // SAFETY: `buffer` is non-null (checked above) and points into `data`, which is still alive
    // and not mutated while the slice exists. The length is rounded *down* to whole 4-byte pairs,
    // so it never exceeds the `length` bytes the callee reported. A `u8` slice has no alignment
    // requirement, which is precisely why the pairs are decoded from bytes below instead of
    // reinterpreting this as `*const u16`.
    let bytes = unsafe { std::slice::from_raw_parts(buffer.cast::<u8>(), pair_count * 4) };
    bytes
        .chunks_exact(4)
        .map(|pair| {
            (
                u16::from_le_bytes([pair[0], pair[1]]),
                u16::from_le_bytes([pair[2], pair[3]]),
            )
        })
        .collect()
}

fn string_value(data: &[u8], translations: &[(u16, u16)], key: &str) -> Option<String> {
    translations
        .iter()
        .find_map(|&(language, code_page)| string_value_for(data, language, code_page, key))
}

fn string_value_for(data: &[u8], language: u16, code_page: u16, key: &str) -> Option<String> {
    let query = format!(r"\StringFileInfo\{language:04x}{code_page:04x}\{key}");
    let query_wide = wide(OsStr::new(&query));
    let mut buffer: *mut c_void = std::ptr::null_mut();
    let mut length = 0;
    // SAFETY: as in `translations` — a version-info block filled by `GetFileVersionInfoW`, a
    // NUL-terminated query alive for the call, and live out-parameters. The returned pointer
    // borrows `data`, which outlives every read below.
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
    // `buffer` points inside a `Vec<u8>`, whose alignment is 1, so reinterpreting it directly as
    // `*const u16` violates the `from_raw_parts` alignment contract. Copy the raw bytes (a `u8`
    // read has no alignment requirement) into a properly aligned `u16` buffer instead. `length`
    // is a character count for string values.
    let mut wide = vec![0_u16; length as usize];
    // SAFETY: source and destination are both valid for `wide.len() * 2` bytes — the destination
    // by construction, the source because `length` is the character count the callee reported for
    // a UTF-16 value inside `data`, which is still alive. The two allocations are distinct, so
    // they cannot overlap. Both pointers are cast to `*const u8`/`*mut u8`, which have no
    // alignment requirement; that is the whole point of copying rather than reinterpreting.
    unsafe {
        std::ptr::copy_nonoverlapping(
            buffer.cast::<u8>(),
            wide.as_mut_ptr().cast::<u8>(),
            wide.len() * 2,
        );
    }
    let end = wide
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(wide.len());
    let value = String::from_utf16_lossy(&wide[..end]).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_files_own_translations_are_tried_before_the_english_fallback() {
        // A Russian-only executable: its block is queried first, US English only as a backstop.
        let russian = (0x0419, 0x04b0);
        assert_eq!(
            candidate_translations(vec![russian]),
            vec![russian, (0x0409, 0x04b0)]
        );
    }

    #[test]
    fn every_declared_translation_is_kept_in_order() {
        let declared = vec![(0x0419, 0x04b0), (0x0407, 0x04b0)];
        assert_eq!(
            candidate_translations(declared.clone()),
            [declared, vec![(0x0409, 0x04b0)]].concat()
        );
    }

    #[test]
    fn english_is_not_duplicated_when_the_file_already_declares_it() {
        assert_eq!(
            candidate_translations(vec![(0x0409, 0x04b0)]),
            vec![(0x0409, 0x04b0)]
        );
    }

    #[test]
    fn a_file_without_a_translation_block_still_queries_english() {
        assert_eq!(candidate_translations(vec![]), vec![(0x0409, 0x04b0)]);
    }

    #[test]
    fn product_version_wins_when_present() {
        assert_eq!(
            pick_version(Some("4.5".to_string()), Some("4.5.2180.0".to_string())),
            Some("4.5".to_string())
        );
    }

    #[test]
    fn file_version_is_used_only_when_product_version_is_absent() {
        assert_eq!(
            pick_version(None, Some("4.5.2180.0".to_string())),
            Some("4.5.2180.0".to_string())
        );
    }

    #[test]
    fn no_version_when_neither_is_present() {
        assert_eq!(pick_version(None, None), None);
    }
}
