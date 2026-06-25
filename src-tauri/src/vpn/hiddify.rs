use super::{Runner, VpnProvider};
use std::path::{Path, PathBuf};

// Validated later (Task 0). Defaults are the most likely commands.
pub(crate) const HIDDIFY_SETUP: [&str; 2] = ["tunnel", "install"];
pub(crate) const HIDDIFY_ENABLE: [&str; 2] = ["tunnel", "activate"];
pub(crate) const HIDDIFY_DISABLE: [&str; 2] = ["tunnel", "deactivate"];
const GUI_IMAGE: &str = "Hiddify.exe";

fn candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(pf) = std::env::var_os("ProgramFiles") {
        dirs.push(PathBuf::from(pf).join("Hiddify"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join(r"Programs\hiddify"));
    }
    dirs
}

pub(crate) fn binary_in(dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .map(|dir| dir.join("HiddifyCli.exe"))
        .find(|path| path.is_file())
}

pub struct Hiddify;

impl Hiddify {
    /// Core logic with an explicit binary path, so tests don't depend on a real install.
    pub(crate) fn set_with(
        &self,
        runner: &dyn Runner,
        enabled: bool,
        binary: &Path,
    ) -> Result<(), String> {
        if enabled {
            if runner.process_running(GUI_IMAGE) {
                let _ = runner.run(Path::new("taskkill"), &["/IM", GUI_IMAGE, "/F"]);
            }
            runner
                .run(binary, &HIDDIFY_ENABLE)
                .and_then(ok_or_fail("Could not start the VPN tunnel"))
        } else {
            runner
                .run(binary, &HIDDIFY_DISABLE)
                .and_then(ok_or_fail("Could not stop the VPN tunnel"))
        }
    }
}

fn ok_or_fail(message: &'static str) -> impl Fn(bool) -> Result<(), String> {
    move |ok| if ok { Ok(()) } else { Err(message.into()) }
}

impl VpnProvider for Hiddify {
    fn id(&self) -> &'static str { "hiddify" }
    fn name(&self) -> &'static str { "Hiddify" }
    fn binary(&self) -> Option<PathBuf> { binary_in(&candidate_dirs()) }
    fn installed(&self, _runner: &dyn Runner) -> bool { self.binary().is_some() }
    fn connected(&self, runner: &dyn Runner) -> bool { runner.process_running(GUI_IMAGE) }
    fn setup(&self, runner: &dyn Runner) -> Result<(), String> {
        let binary = self.binary().ok_or("HiddifyCli.exe was not found")?;
        runner.run_elevated(&binary, &HIDDIFY_SETUP)
    }
    fn set(&self, runner: &dyn Runner, enabled: bool) -> Result<(), String> {
        let binary = self.binary().ok_or("HiddifyCli.exe was not found")?;
        self.set_with(runner, enabled, &binary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vpn::Runner;
    use std::cell::RefCell;
    use std::path::Path;

    #[derive(Default)]
    struct FakeRunner {
        calls: RefCell<Vec<Vec<String>>>,
        running: Vec<String>,
    }
    impl Runner for FakeRunner {
        fn run(&self, program: &Path, args: &[&str]) -> Result<bool, String> {
            let mut call = vec![program.to_string_lossy().into_owned()];
            call.extend(args.iter().map(|a| a.to_string()));
            self.calls.borrow_mut().push(call);
            Ok(true)
        }
        fn run_elevated(&self, program: &Path, args: &[&str]) -> Result<(), String> {
            self.run(program, args).map(|_| ())
        }
        fn process_running(&self, image: &str) -> bool {
            self.running.iter().any(|name| name.eq_ignore_ascii_case(image))
        }
    }

    #[test]
    fn enable_deactivates_gui_then_activates_tunnel() {
        let runner = FakeRunner { running: vec!["Hiddify.exe".into()], ..Default::default() };
        Hiddify.set_with(&runner, true, Path::new(r"C:\HiddifyCli.exe")).unwrap();
        let calls = runner.calls.borrow();
        assert!(calls.iter().any(|c| c.iter().any(|a| a.contains("taskkill") || a.contains("Hiddify.exe"))));
        assert!(calls.last().unwrap().contains(&HIDDIFY_ENABLE[0].to_string()));
    }

    #[test]
    fn disable_runs_the_off_command() {
        let runner = FakeRunner::default();
        Hiddify.set_with(&runner, false, Path::new(r"C:\HiddifyCli.exe")).unwrap();
        let calls = runner.calls.borrow();
        assert!(calls.last().unwrap().contains(&HIDDIFY_DISABLE[0].to_string()));
    }

    #[test]
    fn locates_binary_in_program_files_layout() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("HiddifyCli.exe");
        std::fs::write(&exe, []).unwrap();
        assert_eq!(binary_in(&[dir.path().to_path_buf()]), Some(exe));
    }
}
