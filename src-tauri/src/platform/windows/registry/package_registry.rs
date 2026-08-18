use std::collections::HashMap;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

const APPLICATION_DATA: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\AppModel\StateRepository\Cache\Application\Data";
const MAX_APPLICATIONS: usize = 8192;

pub(crate) fn packaged_executables() -> HashMap<String, String> {
    let mut executables = HashMap::new();
    let Ok(root) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(APPLICATION_DATA) else {
        return executables;
    };
    for name in root.enum_keys().flatten().take(MAX_APPLICATIONS) {
        let Ok(entry) = root.open_subkey(&name) else {
            continue;
        };
        let (Ok(app_id), Ok(executable)) = (
            entry.get_value::<String, _>("ApplicationUserModelId"),
            entry.get_value::<String, _>("Executable"),
        ) else {
            continue;
        };
        let app_id = app_id.trim();
        let executable = executable.trim();
        if app_id.is_empty() || executable.is_empty() {
            continue;
        }
        executables.insert(app_id.to_lowercase(), executable.to_string());
    }
    executables
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_packaged_executables_without_panicking() {
        let executables = packaged_executables();

        assert!(executables.len() <= MAX_APPLICATIONS);
        for (app_id, executable) in &executables {
            assert!(!app_id.is_empty());
            assert!(!executable.is_empty());
            assert_eq!(app_id.to_lowercase(), *app_id);
            assert_eq!(executable.trim(), executable);
        }
    }
}
