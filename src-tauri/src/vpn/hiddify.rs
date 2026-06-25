use super::{Runner, VpnProvider};
use std::path::{Path, PathBuf};

// Variant 1 (validated in Task 0): Hiddify's standalone CLI tunnel control is broken on
// this build (gRPC port bug on non-English locale; `run` duplicates the `balance` outbound).
// The reliable path is the GUI itself, which connects via its own working core + the
// already-installed privileged service. So: ON launches the GUI (with "Connect on start"
// enabled, it auto-connects — no manual click); OFF closes it, dropping the tunnel.
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
        .map(|dir| dir.join(GUI_IMAGE))
        .find(|path| path.is_file())
}

pub struct Hiddify;

impl Hiddify {
    /// Core logic with an explicit binary path, so tests don't depend on a real install.
    /// ON launches the GUI (auto-connect); OFF force-closes it.
    pub(crate) fn set_with(
        &self,
        runner: &dyn Runner,
        enabled: bool,
        binary: &Path,
    ) -> Result<(), String> {
        if enabled {
            runner.launch(binary)
        } else {
            runner
                .run(Path::new("taskkill"), &["/IM", GUI_IMAGE, "/F"])
                .map(|_| ())
        }
    }
}

impl VpnProvider for Hiddify {
    fn id(&self) -> &'static str {
        "hiddify"
    }
    fn name(&self) -> &'static str {
        "Hiddify"
    }
    fn binary(&self) -> Option<PathBuf> {
        binary_in(&candidate_dirs())
    }
    fn installed(&self, _runner: &dyn Runner) -> bool {
        self.binary().is_some()
    }
    fn connected(&self, runner: &dyn Runner) -> bool {
        runner.process_running(GUI_IMAGE)
    }
    fn setup(&self, _runner: &dyn Runner) -> Result<(), String> {
        // Variant 1 needs no privileged setup — the GUI handles connection itself.
        Ok(())
    }
    fn set(&self, runner: &dyn Runner, enabled: bool) -> Result<(), String> {
        let binary = self.binary().ok_or("Hiddify.exe was not found")?;
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
        launched: RefCell<Vec<String>>,
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
        fn launch(&self, program: &Path) -> Result<(), String> {
            self.launched
                .borrow_mut()
                .push(program.to_string_lossy().into_owned());
            Ok(())
        }
        fn process_running(&self, image: &str) -> bool {
            self.running
                .iter()
                .any(|name| name.eq_ignore_ascii_case(image))
        }
    }

    #[test]
    fn enable_launches_the_gui() {
        let runner = FakeRunner::default();
        Hiddify
            .set_with(&runner, true, Path::new(r"C:\Program Files\Hiddify\Hiddify.exe"))
            .unwrap();
        assert_eq!(
            runner.launched.borrow().as_slice(),
            [r"C:\Program Files\Hiddify\Hiddify.exe".to_string()]
        );
        assert!(runner.calls.borrow().is_empty());
    }

    #[test]
    fn disable_closes_the_gui_process() {
        let runner = FakeRunner::default();
        Hiddify
            .set_with(&runner, false, Path::new(r"C:\Program Files\Hiddify\Hiddify.exe"))
            .unwrap();
        let calls = runner.calls.borrow();
        let last = calls.last().unwrap();
        assert!(last.iter().any(|a| a.contains("taskkill")));
        assert!(last.iter().any(|a| a == GUI_IMAGE));
        assert!(runner.launched.borrow().is_empty());
    }

    #[test]
    fn locates_gui_binary_in_program_files_layout() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Hiddify.exe");
        std::fs::write(&exe, []).unwrap();
        assert_eq!(binary_in(&[dir.path().to_path_buf()]), Some(exe));
    }
}
