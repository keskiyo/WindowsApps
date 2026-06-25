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
