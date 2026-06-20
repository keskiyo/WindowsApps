use winreg::{enums::HKEY_CURRENT_USER, RegKey};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "Windows Apps";

fn command() -> Result<String, String> {
    let executable = std::env::current_exe().map_err(|error| format!("Could not locate Windows Apps: {error}"))?;
    Ok(format!(r#""{}""#, executable.display()))
}

pub fn is_enabled() -> Result<bool, String> {
    let key = match RegKey::predef(HKEY_CURRENT_USER).open_subkey(RUN_KEY) {
        Ok(key) => key,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("Could not read Windows startup settings: {error}")),
    };
    Ok(key.get_value::<String, _>(VALUE_NAME).is_ok())
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY)
        .map_err(|error| format!("Could not open Windows startup settings: {error}"))?;
    if enabled {
        key.set_value(VALUE_NAME, &command()?).map_err(|error| format!("Could not enable startup: {error}"))
    } else {
        match key.delete_value(VALUE_NAME) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("Could not disable startup: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_command_quotes_the_executable_path() {
        let value = command().unwrap();
        assert!(value.starts_with('"'));
        assert!(value.ends_with('"'));
    }
}
