use crate::platform::windows::com::{ensure_initialized, CoTaskString};
use windows::core::{Interface, GUID};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::UI::Shell::{
    BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, IShellItem2,
    SHGetKnownFolderItem, KF_FLAG_DEFAULT, SIGDN, SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
};

const LINK: GUID = GUID::from_u128(0xb9b4b3fc_2b51_4a42_b5d8_324146afcf25);
const APP_USER_MODEL: GUID = GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3);

const TARGET_PARSING_PATH: PROPERTYKEY = PROPERTYKEY {
    fmtid: LINK,
    pid: 2,
};
const PACKAGE_INSTALL_PATH: PROPERTYKEY = PROPERTYKEY {
    fmtid: APP_USER_MODEL,
    pid: 15,
};
const PACKAGE_FULL_NAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: APP_USER_MODEL,
    pid: 21,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PackageIdentity {
    pub full_name: String,
    pub install_location: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct StartAppEntry {
    pub name: String,
    pub app_id: String,
    pub target: Option<String>,
    pub package: Option<PackageIdentity>,
}

const MAX_ENTRIES: usize = 4096;
const BATCH: usize = 32;

pub(crate) fn start_apps(is_cancelled: &dyn Fn() -> bool) -> Option<Vec<StartAppEntry>> {
    ensure_initialized();
    enumerate(is_cancelled).ok()
}

fn enumerate(is_cancelled: &dyn Fn() -> bool) -> windows::core::Result<Vec<StartAppEntry>> {
    // SAFETY: `start_apps` joins an apartment before calling, which every interface below requires.
    // `FOLDERID_AppsFolder` is a static constant, so the pointer outlives the call; `KF_FLAG_DEFAULT`
    // with no access token asks for the calling user's own view of the folder and creates nothing.
    // Both interfaces are reference counted and released when they drop, and `?` returns before
    // either binding exists on failure, so no reference outlives this function.
    let folder: IShellItem =
        unsafe { SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None) }?;
    let items: IEnumShellItems = unsafe { folder.BindToHandler(None, &BHID_EnumItems) }?;

    let mut entries = Vec::new();
    let mut batch: [Option<IShellItem>; BATCH] = std::array::from_fn(|_| None);
    while entries.len() < MAX_ENTRIES && !is_cancelled() {
        let mut fetched = 0;
        // SAFETY: `batch` is a live local for the whole call and `Option<IShellItem>` has the
        // nullable-pointer layout the enumerator writes, so it fills at most `BATCH` slots that are
        // in bounds. `fetched` is a live local the callee writes through. Every slot is drained by
        // `take()` below before the next iteration, so the enumerator never overwrites a reference
        // that still holds a count, and each reference it addrefs for us is released exactly once.
        unsafe { items.Next(&mut batch, Some(&mut fetched)) }?;
        if fetched == 0 {
            break;
        }
        entries.extend(
            batch
                .iter_mut()
                .filter_map(|slot| read_entry(&slot.take()?)),
        );
    }
    Ok(entries)
}

fn read_entry(item: &IShellItem) -> Option<StartAppEntry> {
    let app_id = display_name(item, SIGDN_PARENTRELATIVEPARSING)?;
    let name = display_name(item, SIGDN_NORMALDISPLAY)?;
    let properties = item.cast::<IShellItem2>().ok();
    Some(StartAppEntry {
        target: properties
            .as_ref()
            .and_then(|item| string_property(item, &TARGET_PARSING_PATH)),
        package: properties.as_ref().and_then(package_identity),
        name,
        app_id,
    })
}

fn package_identity(item: &IShellItem2) -> Option<PackageIdentity> {
    Some(PackageIdentity {
        full_name: string_property(item, &PACKAGE_FULL_NAME)?,
        install_location: string_property(item, &PACKAGE_INSTALL_PATH),
    })
}

fn display_name(item: &IShellItem, form: SIGDN) -> Option<String> {
    // SAFETY: `item` is a live interface the caller owns for the whole call, so the vtable entry is
    // valid. On success the shell allocates a null-terminated UTF-16 buffer with the COM task
    // allocator and transfers it to us; `CoTaskString::own` takes that ownership on the same line,
    // so it is freed on every path out. On failure nothing is allocated.
    let raw = unsafe { item.GetDisplayName(form) }.ok()?;
    CoTaskString::own(raw).to_trimmed()
}

fn string_property(item: &IShellItem2, key: &PROPERTYKEY) -> Option<String> {
    // SAFETY: `item` and `key` are both live for the whole call — the key is a constant and the
    // interface is borrowed from the caller. Entries that carry no such property fail, which
    // `.ok()?` turns into `None` before anything is allocated. On success the shell hands us a
    // null-terminated UTF-16 buffer from the COM task allocator and `CoTaskString::own` takes that
    // ownership on the same line, so it is freed exactly once.
    let raw = unsafe { item.GetString(key) }.ok()?;
    CoTaskString::own(raw).to_trimmed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerates_the_apps_folder_without_an_interpreter() {
        let never = || false;

        let Some(entries) = start_apps(&never) else {
            return;
        };

        assert!(entries.len() <= MAX_ENTRIES + BATCH);
        for entry in &entries {
            assert!(!entry.name.is_empty());
            assert!(!entry.app_id.is_empty());
            assert_eq!(entry.name.trim(), entry.name);
            assert!(entry.target.as_deref() != Some(""));
        }
    }

    #[test]
    fn a_packaged_entry_carries_the_full_name_its_uninstall_needs() {
        let never = || false;

        let Some(entries) = start_apps(&never) else {
            return;
        };

        for package in entries.iter().filter_map(|entry| entry.package.as_ref()) {
            assert!(package.full_name.contains('_'));
            assert!(package.install_location.as_deref() != Some(""));
        }
    }

    #[test]
    fn a_cancelled_scan_stops_before_the_first_batch() {
        let cancelled = || true;

        let entries = start_apps(&cancelled);

        assert_eq!(entries, Some(Vec::new()));
    }
}
