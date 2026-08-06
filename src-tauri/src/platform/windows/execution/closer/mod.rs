//! Closing the programs a catalog entry starts.
//!
//! The catalog never retains a PID — `launcher` hands its process handle back only long enough to
//! wait for input idle — so a close has to find the programs again by the image they run. Every
//! process of that image is asked to close the way the title-bar button asks (`WM_CLOSE`, which
//! lets an application prompt to save), and whatever ignores the request is terminated, so nothing
//! of the program is left behind.
//!
//! A whole batch is closed in one pass: one process snapshot, one window enumeration and a single
//! grace period for every target together. Closing apps one at a time would make a scenario wait
//! the grace period once per application.

mod frames;
mod processes;

use processes::{image_path_of, running_images, same_executable, terminate};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// How long a program may take to honour `WM_CLOSE` before it is terminated. Long enough for an
/// editor to raise its "save changes?" prompt and for the user to answer it.
const GRACE_PERIOD_MS: u64 = 5000;

/// What a close attempt actually did, per requested target. Counts rather than a bare success:
/// "it was not running" and "it was closed" are different answers and the interface reports both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CloseOutcome {
    pub(crate) closed: usize,
    pub(crate) not_running: usize,
}

/// The executable names to look for, lowercased — the cheap filter that keeps the process
/// enumeration from opening a handle for every process on the machine.
fn file_names_of(targets: &[&str]) -> HashSet<String> {
    targets
        .iter()
        .filter_map(|target| Path::new(target).file_name())
        .map(|name| name.to_string_lossy().to_lowercase())
        .collect()
}

fn processes_of(target: &str, running: &[(u32, String)]) -> Vec<u32> {
    running
        .iter()
        .filter(|(_, image)| same_executable(image, target))
        .map(|(pid, _)| *pid)
        .collect()
}

/// Ask every window of `targets` to close, then terminate whatever ignored the request.
///
/// Blocking: the grace period is a real sleep, so callers run this off the async runtime.
pub(crate) fn close_processes(targets: &[PathBuf]) -> CloseOutcome {
    let targets: Vec<&str> = targets
        .iter()
        .filter_map(|target| target.to_str())
        .collect();
    if targets.is_empty() {
        return CloseOutcome::default();
    }
    let running = running_images(&file_names_of(&targets));
    let matched: Vec<Vec<u32>> = targets
        .iter()
        .map(|target| processes_of(target, &running))
        .collect();
    let live: HashSet<u32> = matched.iter().flatten().copied().collect();
    if live.is_empty() {
        return CloseOutcome {
            closed: 0,
            not_running: targets.len(),
        };
    }
    for window in frames::windows_of(live) {
        frames::ask_to_close(window);
    }
    std::thread::sleep(std::time::Duration::from_millis(GRACE_PERIOD_MS));
    finish(&targets, &matched)
}

/// Terminates what survived the grace period and reports, per target, whether anything is left.
fn finish(targets: &[&str], matched: &[Vec<u32>]) -> CloseOutcome {
    // Re-read the process list rather than reusing the first snapshot: a program that honoured the
    // request is already gone, and one that restarted itself has a pid nobody has seen yet.
    let running = running_images(&file_names_of(targets));
    let mut outcome = CloseOutcome::default();
    for (target, before) in targets.iter().zip(matched) {
        if before.is_empty() {
            outcome.not_running += 1;
            continue;
        }
        let survivors = processes_of(target, &running);
        for pid in &survivors {
            terminate(*pid);
        }
        // The termination result is not the answer — a process can exit on its own between the
        // snapshot and the call, and one belonging to another user cannot be opened at all. Asking
        // again is what distinguishes "gone" from "still there".
        if survivors.iter().all(|pid| is_gone(*pid, target)) {
            outcome.closed += 1;
        }
    }
    outcome
}

/// Whether nothing of `target` runs under `pid` any more. A pid Windows already recycled for an
/// unrelated process counts as gone, which is also why a survivor's image is re-checked before it
/// is ever terminated by the caller above.
fn is_gone(pid: u32, target: &str) -> bool {
    image_path_of(pid).is_none_or(|image| !same_executable(&image, target))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_nothing_for_an_empty_request() {
        assert_eq!(close_processes(&[]), CloseOutcome::default());
    }

    // Nothing runs from these paths, so the close does no window work and reports both targets as
    // idle instead of claiming it closed them.
    #[test]
    fn reports_targets_that_are_not_running() {
        let outcome = close_processes(&[
            PathBuf::from(r"C:\Nowhere\this-executable-does-not-exist.exe"),
            PathBuf::from(r"C:\Nowhere\neither-does-this-one.exe"),
        ]);

        assert_eq!(
            outcome,
            CloseOutcome {
                closed: 0,
                not_running: 2
            }
        );
    }

    #[test]
    fn looks_only_for_the_executable_names_it_was_given() {
        let names = file_names_of(&[r"C:\Editor\Editor.exe", "C:/Games/game.exe"]);

        assert_eq!(
            names,
            HashSet::from(["editor.exe".into(), "game.exe".into()])
        );
    }

    #[test]
    fn selects_only_the_processes_running_the_requested_image() {
        let running = vec![
            (10, r"C:\Editor\editor.exe".to_string()),
            (11, r"c:\editor\EDITOR.EXE".to_string()),
            (12, r"C:\Editor\updater.exe".to_string()),
        ];

        assert_eq!(
            processes_of(r"C:\Editor\editor.exe", &running),
            vec![10, 11]
        );
    }

    // A target nothing was running for must not be reported as closed, and must not cost a
    // termination attempt either.
    #[test]
    fn counts_a_target_with_no_processes_as_idle() {
        let outcome = finish(&[r"C:\Editor\editor.exe"], &[Vec::new()]);

        assert_eq!(
            outcome,
            CloseOutcome {
                closed: 0,
                not_running: 1
            }
        );
    }

    // The process ids are long gone, so nothing of the target is left: the target counts as closed
    // without any handle being opened on a live process.
    #[test]
    fn counts_a_target_whose_processes_are_gone_as_closed() {
        let outcome = finish(&[r"C:\Nowhere\gone.exe"], &[vec![u32::MAX]]);

        assert_eq!(
            outcome,
            CloseOutcome {
                closed: 1,
                not_running: 0
            }
        );
    }
}
