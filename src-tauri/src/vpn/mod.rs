use serde::Serialize;
use std::path::Path;

pub mod hiddify;

/// Result of a CLI invocation, abstracted so providers are unit-testable.
pub trait Runner {
    /// Run `program args...` unelevated; Ok(true) when the process exits 0.
    fn run(&self, program: &Path, args: &[&str]) -> Result<bool, String>;
    /// Run `program args...` elevated (UAC). Ok(()) once the user accepts.
    fn run_elevated(&self, program: &Path, args: &[&str]) -> Result<(), String>;
    /// True if a process with this image name is running (used for status / GUI checks).
    fn process_running(&self, image_name: &str) -> bool;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VpnInfo {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub connected: bool,
}

pub trait VpnProvider {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn binary(&self) -> Option<std::path::PathBuf>;
    fn installed(&self, runner: &dyn Runner) -> bool;
    fn connected(&self, runner: &dyn Runner) -> bool;
    fn setup(&self, runner: &dyn Runner) -> Result<(), String>;
    fn set(&self, runner: &dyn Runner, enabled: bool) -> Result<(), String>;
    fn info(&self, runner: &dyn Runner) -> VpnInfo {
        VpnInfo {
            id: self.id().into(),
            name: self.name().into(),
            installed: self.binary().is_some() && self.installed(runner),
            connected: self.connected(runner),
        }
    }
}

/// All known providers (Hiddify today; extend here later).
pub fn providers() -> Vec<Box<dyn VpnProvider>> {
    vec![Box::new(hiddify::Hiddify)]
}

pub fn provider(id: &str) -> Option<Box<dyn VpnProvider>> {
    providers().into_iter().find(|provider| provider.id() == id)
}

use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, program: &Path, args: &[&str]) -> Result<bool, String> {
        Command::new(program)
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map(|status| status.success())
            .map_err(|error| format!("Could not run {}: {error}", program.display()))
    }
    fn run_elevated(&self, program: &Path, args: &[&str]) -> Result<(), String> {
        crate::platform::windows::launcher::shell_execute_elevated(program, args)
    }
    fn process_running(&self, image: &str) -> bool {
        Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {image}"), "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|out| {
                String::from_utf8_lossy(&out.stdout)
                    .to_lowercase()
                    .contains(&image.to_lowercase())
            })
            .unwrap_or(false)
    }
}
