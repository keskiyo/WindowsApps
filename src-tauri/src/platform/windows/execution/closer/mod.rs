mod frames;
mod identity;
mod processes;

pub(crate) use identity::CloseTarget;

use identity::is_instance_of;
use processes::{image_path_of, running_images, terminate};
use std::collections::HashSet;

const GRACE_SECONDS: u64 = 5;
const TERMINATION_RECHECKS: usize = 4;
const TERMINATION_RECHECK_DELAY_MS: u64 = 250;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CloseStage {
    Asking { running: usize },
    Waiting { seconds_left: u64 },
    Terminating,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CloseOutcome {
    pub(crate) closed: usize,
    pub(crate) not_running: usize,
    pub(crate) failed: usize,
}

fn processes_of(target: &CloseTarget, running: &[(u32, String)]) -> Vec<u32> {
    running
        .iter()
        .filter(|(_, image)| is_instance_of(image, target))
        .map(|(pid, _)| *pid)
        .collect()
}

pub(crate) fn close_processes(
    targets: &[CloseTarget],
    progress: impl Fn(CloseStage),
) -> CloseOutcome {
    if targets.is_empty() {
        return CloseOutcome::default();
    }
    let running = running_images();
    let matched: Vec<Vec<u32>> = targets
        .iter()
        .map(|target| processes_of(target, &running))
        .collect();
    let live: HashSet<u32> = matched.iter().flatten().copied().collect();
    if live.is_empty() {
        return CloseOutcome {
            closed: 0,
            not_running: targets.len(),
            failed: 0,
        };
    }
    progress(CloseStage::Asking {
        running: matched.iter().filter(|pids| !pids.is_empty()).count(),
    });
    for window in frames::windows_of(live) {
        frames::ask_to_close(window);
    }
    for seconds_left in (1..=GRACE_SECONDS).rev() {
        progress(CloseStage::Waiting { seconds_left });
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    progress(CloseStage::Terminating);
    finish(targets, &matched)
}

fn finish(targets: &[CloseTarget], matched: &[Vec<u32>]) -> CloseOutcome {
    let running = running_images();
    finish_with(targets, matched, &running, terminate, is_gone)
}

fn finish_with(
    targets: &[CloseTarget],
    matched: &[Vec<u32>],
    running: &[(u32, String)],
    terminate_process: impl FnMut(u32) -> bool,
    process_is_gone: impl FnMut(u32, &CloseTarget) -> bool,
) -> CloseOutcome {
    finish_with_pause(
        targets,
        matched,
        running,
        terminate_process,
        process_is_gone,
        || {
            std::thread::sleep(std::time::Duration::from_millis(
                TERMINATION_RECHECK_DELAY_MS,
            ))
        },
    )
}

fn finish_with_pause(
    targets: &[CloseTarget],
    matched: &[Vec<u32>],
    running: &[(u32, String)],
    mut terminate_process: impl FnMut(u32) -> bool,
    mut process_is_gone: impl FnMut(u32, &CloseTarget) -> bool,
    mut pause: impl FnMut(),
) -> CloseOutcome {
    let survivors: Vec<Vec<u32>> = targets
        .iter()
        .map(|target| processes_of(target, running))
        .collect();
    for pids in &survivors {
        for pid in pids {
            terminate_process(*pid);
        }
    }
    let mut gone: Vec<Vec<bool>> = survivors
        .iter()
        .map(|pids| vec![pids.is_empty(); pids.len()])
        .collect();
    for attempt in 0..=TERMINATION_RECHECKS {
        for ((target, pids), states) in targets.iter().zip(&survivors).zip(&mut gone) {
            for (pid, state) in pids.iter().zip(states) {
                if !*state {
                    *state = process_is_gone(*pid, target);
                }
            }
        }
        if gone.iter().flatten().all(|state| *state) {
            break;
        }
        if attempt < TERMINATION_RECHECKS {
            pause();
        }
    }
    let mut outcome = CloseOutcome::default();
    for ((before, survivors), gone) in matched.iter().zip(&survivors).zip(&gone) {
        if before.is_empty() {
            outcome.not_running += 1;
        } else if survivors.iter().zip(gone).all(|(_, state)| *state) {
            outcome.closed += 1;
        } else {
            outcome.failed += 1;
        }
    }
    outcome
}

fn is_gone(pid: u32, target: &CloseTarget) -> bool {
    image_path_of(pid).is_none_or(|image| !is_instance_of(&image, target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    fn target(executable: &str) -> CloseTarget {
        let path = PathBuf::from(executable);
        let root = path.parent().map(Path::to_path_buf);
        CloseTarget::new(path, root)
    }

    #[test]
    fn reports_nothing_for_an_empty_request() {
        assert_eq!(close_processes(&[], |_| {}), CloseOutcome::default());
    }

    // Nothing is running, so nothing is asked, waited for or terminated: a scenario that closes
    // idle applications must not show the user a five second countdown over no work.
    #[test]
    fn an_idle_target_reports_no_stages() {
        let stages = Cell::new(0);

        close_processes(
            &[target(r"C:\Nowhere\this-executable-does-not-exist.exe")],
            |_| stages.set(stages.get() + 1),
        );

        assert_eq!(stages.get(), 0);
    }

    #[test]
    fn reports_targets_that_are_not_running() {
        let outcome = close_processes(
            &[
                target(r"C:\Nowhere\this-executable-does-not-exist.exe"),
                target(r"C:\Nowhere\neither-does-this-one.exe"),
            ],
            |_| {},
        );

        assert_eq!(
            outcome,
            CloseOutcome {
                closed: 0,
                not_running: 2,
                failed: 0,
            }
        );
    }

    #[test]
    fn selects_all_processes_in_the_requested_installation_root() {
        let running = vec![
            (10, r"C:\Apps\Editor\editor.exe".to_string()),
            (11, r"c:\apps\editor\EDITOR.EXE".to_string()),
            (12, r"C:\Apps\Editor\updater.exe".to_string()),
        ];

        assert_eq!(
            processes_of(&target(r"C:\Apps\Editor\editor.exe"), &running),
            vec![10, 11, 12]
        );
    }

    #[test]
    fn selects_launcher_replacements_in_the_same_installation_root() {
        let running = vec![
            (10, r"D:\Games\Battle.net\Battle.net.exe".to_string()),
            (11, r"D:\Games\Battle.net\Agent.exe".to_string()),
            (12, r"D:\Games\Other\Battle.net.exe".to_string()),
        ];

        assert_eq!(
            processes_of(
                &target(r"D:\Games\Battle.net\Battle.net Launcher.exe"),
                &running
            ),
            vec![10, 11]
        );
    }

    #[test]
    fn selects_the_processes_of_an_application_that_updated_itself() {
        let chat = CloseTarget::new(
            PathBuf::from(r"C:\Program Files\WindowsApps\Vendor.App_1.0.0.0_x64__abc\app\Chat.exe"),
            Some(PathBuf::from(
                r"C:\Program Files\WindowsApps\Vendor.App_1.0.0.0_x64__abc",
            )),
        );
        let running = vec![
            (
                20,
                r"C:\Program Files\WindowsApps\Vendor.App_2.5.9.0_x64__abc\app\Chat.exe"
                    .to_string(),
            ),
            (
                21,
                r"C:\Program Files\WindowsApps\Other.App_2.5.9.0_x64__zzz\app\Chat.exe".to_string(),
            ),
        ];

        assert_eq!(processes_of(&chat, &running), vec![20]);
    }

    #[test]
    fn counts_a_target_with_no_processes_as_idle() {
        let outcome = finish(&[target(r"C:\Editor\editor.exe")], &[Vec::new()]);

        assert_eq!(
            outcome,
            CloseOutcome {
                closed: 0,
                not_running: 1,
                failed: 0,
            }
        );
    }

    #[test]
    fn counts_a_target_whose_processes_are_gone_as_closed() {
        let outcome = finish(&[target(r"C:\Nowhere\gone.exe")], &[vec![u32::MAX]]);

        assert_eq!(
            outcome,
            CloseOutcome {
                closed: 1,
                not_running: 0,
                failed: 0,
            }
        );
    }

    #[test]
    fn reports_a_process_that_survives_termination_as_failed() {
        let outcome = finish_with(
            &[target(r"C:\\Editor\\editor.exe")],
            &[vec![10]],
            &[(10, r"C:\\Editor\\editor.exe".to_string())],
            |_| false,
            |_, _| false,
        );

        assert_eq!(outcome.failed, 1);
        assert_eq!(outcome.closed, 0);
    }

    #[test]
    fn rechecks_all_survivors_in_one_bounded_window() {
        let targets = [
            target(r"C:\Editor\editor.exe"),
            target(r"C:\Browser\browser.exe"),
        ];
        let matched = [vec![10], vec![20]];
        let running = [
            (10, r"C:\Editor\editor.exe".to_string()),
            (20, r"C:\Browser\browser.exe".to_string()),
        ];

        let started_at = Instant::now();
        let outcome = finish_with(&targets, &matched, &running, |_| false, |_, _| false);

        assert_eq!(outcome.failed, 2);
        assert!(started_at.elapsed() < Duration::from_millis(1500));
    }

    #[test]
    fn confirms_a_process_that_exits_after_the_first_probe() {
        let probes = Cell::new(0);

        let outcome = finish_with_pause(
            &[target(r"C:\Editor\editor.exe")],
            &[vec![10]],
            &[(10, r"C:\Editor\editor.exe".to_string())],
            |_| false,
            |_, _| {
                probes.set(probes.get() + 1);
                probes.get() > 1
            },
            || {},
        );

        assert_eq!(outcome.closed, 1);
    }
}
