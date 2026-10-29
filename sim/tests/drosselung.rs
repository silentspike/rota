//! Executed CPU scheduling, live file control and deterministic combat results.
//! No OS resource groups are created by these tests.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use rota_sim::apl::{parse, AplName};
use rota_sim::balance::{run, run_controlled, BalanceConfig};
use rota_sim::drosselung::{ControlFileWatcher, ControlSettings, RuntimeControl};
use rota_sim::gear::WornValues;

const POLL: Duration = Duration::from_millis(1);

fn settings(cpu_threads: usize, cpu_percent: u8, paused: bool) -> ControlSettings {
    ControlSettings {
        cpu_threads,
        cpu_percent,
        paused,
    }
}

fn wait_until(mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        thread::sleep(POLL);
    }
    condition()
}

/// Wait until the watcher has counted exactly `count` file changes and the
/// last one is in force. `RuntimeControl` shows a policy before the watcher
/// thread counts it, so its effect alone does not prove the count.
fn accepted(
    watcher: &ControlFileWatcher,
    control: &RuntimeControl,
    count: u64,
    expected: ControlSettings,
) -> bool {
    wait_until(|| {
        watcher.status().applied_updates == count && control.snapshot().settings == expected
    })
}

#[derive(Debug)]
struct TempFile {
    directory: PathBuf,
    path: PathBuf,
}

impl TempFile {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "rota-control-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        Self {
            path: directory.join("control.json"),
            directory,
        }
    }

    fn write(&self, settings: ControlSettings) {
        let next = self.directory.join("next.json");
        fs::write(
            &next,
            format!(
                "{{\"cpu_threads\":{},\"cpu_percent\":{},\"paused\":{}}}\n",
                settings.cpu_threads, settings.cpu_percent, settings.paused
            ),
        )
        .unwrap();
        fs::rename(next, &self.path).unwrap();
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn invalid_update_cannot_partially_change_running_policy() {
    assert!(RuntimeControl::new(0, settings(0, 100, false)).is_err());
    let control = RuntimeControl::new(2, settings(2, 100, false)).unwrap();
    for invalid in [
        settings(3, 100, true),
        settings(1, 0, true),
        settings(1, 101, true),
    ] {
        assert!(control.update(invalid).is_err());
        assert_eq!(control.snapshot().settings, settings(2, 100, false));
        assert_eq!(control.snapshot().updates, 0);
    }
    control.update(settings(2, 100, false)).unwrap();
    assert_eq!(
        control.snapshot().updates,
        0,
        "repeated policy is not a change"
    );
}

#[test]
fn thread_limit_waits_for_inflight_jobs_and_does_not_pin_worker_identities() {
    let control = Arc::new(RuntimeControl::new(2, settings(2, 100, false)).unwrap());
    let first = control.acquire();
    let second = control.acquire();
    control.update(settings(1, 100, false)).unwrap();
    let (send, recv) = mpsc::channel();
    let worker_control = Arc::clone(&control);
    let worker = thread::spawn(move || {
        let _permit = worker_control.acquire();
        send.send(()).unwrap();
    });
    let waited = wait_until(|| control.snapshot().throttle_events > 0);
    drop(first);
    let no_early_start = recv.try_recv().is_err();
    let active_after_reduction = control.snapshot().active_workers;
    drop(second);
    let completed = recv.recv_timeout(Duration::from_secs(5)).is_ok();
    worker.join().unwrap();
    assert!(waited && no_early_start && completed);
    assert_eq!(active_after_reduction, 1);
    assert_eq!(control.snapshot().completed_jobs, 3);
    assert_eq!(control.snapshot().peak_active_workers, 2);
}

#[test]
fn pause_and_zero_threads_park_compute_but_status_and_updates_still_work() {
    for blocked in [settings(1, 100, true), settings(0, 100, false)] {
        let control = Arc::new(RuntimeControl::new(1, blocked).unwrap());
        let worker_control = Arc::clone(&control);
        let worker = thread::spawn(move || {
            let _permit = worker_control.acquire();
        });
        let waited = wait_until(|| control.snapshot().throttle_events > 0);
        let before_resume = control.snapshot();
        control.update(settings(1, 100, false)).unwrap();
        worker.join().unwrap();
        assert!(waited);
        assert_eq!(before_resume.started_jobs, 0);
        assert_eq!(before_resume.active_workers, 0);
        assert_eq!(control.snapshot().completed_jobs, 1);
    }
}

#[test]
fn compute_slot_rest_cannot_be_bypassed_by_a_different_worker() {
    let control = Arc::new(RuntimeControl::new(2, settings(1, 50, false)).unwrap());
    let permit = control.acquire();
    thread::sleep(Duration::from_millis(20));
    let (send, receive) = mpsc::channel();
    let worker_control = Arc::clone(&control);
    let worker = thread::spawn(move || {
        let _permit = worker_control.acquire();
        let released: Instant = receive.recv().unwrap();
        released.elapsed()
    });
    // Establish that the contender is waiting before release; host scheduling
    // must not turn this into a test of how quickly a new thread starts.
    let contender_waited = wait_until(|| control.snapshot().throttle_events > 0);
    let released = Instant::now();
    drop(permit);
    let work = control.snapshot().max_job_time;
    send.send(released).unwrap();
    let waited = worker.join().unwrap();
    // Work/rest is 1:1 at 50%. Allow 2 ms for test bookkeeping after release;
    // scheduler delays only increase the measured wait, never shorten it.
    assert!(waited + Duration::from_millis(2) >= work);
    assert!(contender_waited);
    assert!(control.snapshot().throttle_events > 0);
    assert_eq!(control.snapshot().completed_jobs, 2);
}

#[test]
fn changing_cpu_percent_wakes_a_worker_in_its_rest_period() {
    let control = Arc::new(RuntimeControl::new(1, settings(1, 1, false)).unwrap());
    let permit = control.acquire();
    thread::sleep(Duration::from_millis(20));
    drop(permit);
    let worker_control = Arc::clone(&control);
    let worker = thread::spawn(move || {
        let _permit = worker_control.acquire();
    });
    let waited = wait_until(|| control.snapshot().throttle_events > 0);
    let resumed = Instant::now();
    control.update(settings(1, 100, false)).unwrap();
    worker.join().unwrap();
    assert!(waited);
    assert!(
        resumed.elapsed() < Duration::from_secs(1),
        "old 1% delay survived the update"
    );
}

#[test]
fn panicking_job_releases_its_permit() {
    let control = RuntimeControl::new(1, settings(1, 100, false)).unwrap();
    let result = std::panic::catch_unwind(|| {
        let _permit = control.acquire();
        panic!("deliberate job failure");
    });
    assert!(result.is_err());
    assert_eq!(control.snapshot().active_workers, 0);
    drop(control.acquire());
    assert_eq!(control.snapshot().completed_jobs, 2);
}

#[test]
fn initial_control_file_is_strict_complete_and_read_only() {
    let file = TempFile::new();
    let control = Arc::new(RuntimeControl::new(2, settings(2, 100, false)).unwrap());
    assert!(ControlFileWatcher::start(&file.path, Arc::clone(&control), POLL).is_err());
    let invalid = [
        "",
        "{}",
        "[]",
        "{\"cpu_threads\":1}",
        "{\"cpu_threads\":1,\"cpu_percent\":50,\"paused\":false,}",
        "{\"cpu_threads\":1,\"cpu_percent\":50,\"paused\":false,\"paused\":true}",
        "{\"cpu_threads\":1,\"cpu_percent\":50,\"paused\":false,\"gpu\":false}",
        "{\"cpu_threads\":3,\"cpu_percent\":50,\"paused\":false}",
        "{\"cpu_threads\":01,\"cpu_percent\":50,\"paused\":false}",
        "{\"cpu_threads\":1.0,\"cpu_percent\":50,\"paused\":false}",
        "{\"cpu_threads\":1,\"cpu_percent\":0,\"paused\":false}",
        "{\"cpu_threads\":1,\"cpu_percent\":101,\"paused\":false}",
        "{\"cpu_threads\":1,\"cpu_percent\":50,\"paused\":0}",
        "{\"cpu_threads\":1,\"cpu_percent\":50,\"paused\":false} trailing",
    ];
    for text in invalid {
        fs::write(&file.path, text).unwrap();
        assert!(
            ControlFileWatcher::start(&file.path, Arc::clone(&control), POLL).is_err(),
            "{text}"
        );
        assert_eq!(fs::read_to_string(&file.path).unwrap(), text);
        assert_eq!(control.snapshot().settings, settings(2, 100, false));
    }
    fs::write(&file.path, " ".repeat(4097)).unwrap();
    assert!(ControlFileWatcher::start(&file.path, Arc::clone(&control), POLL).is_err());
    file.write(settings(1, 50, true));
    let bytes = fs::read(&file.path).unwrap();
    assert!(ControlFileWatcher::start(&file.path, Arc::clone(&control), Duration::ZERO).is_err());
    let mut watcher = ControlFileWatcher::start(&file.path, Arc::clone(&control), POLL).unwrap();
    assert_eq!(control.snapshot().settings, settings(1, 50, true));
    watcher.stop().unwrap();
    watcher.stop().unwrap();
    assert_eq!(fs::read(&file.path).unwrap(), bytes);
}

#[test]
fn watcher_drop_joins_and_no_longer_changes_policy() {
    let file = TempFile::new();
    file.write(settings(1, 100, true));
    let control = Arc::new(RuntimeControl::new(1, settings(1, 100, false)).unwrap());
    let watcher =
        ControlFileWatcher::start(&file.path, Arc::clone(&control), Duration::from_secs(1))
            .unwrap();
    let stop = Instant::now();
    drop(watcher);
    assert!(
        stop.elapsed() < Duration::from_millis(500),
        "stop did not interrupt the poll timer"
    );
    file.write(settings(1, 100, false));
    thread::sleep(Duration::from_millis(10));
    assert!(control.snapshot().settings.paused);
}

fn fingerprint(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
    })
}

#[test]
fn actual_balance_run_survives_live_limits_pause_and_malformed_file_bit_for_bit() {
    let apl = parse(
        AplName("throttle-proof"),
        "smoulder if dot.smoulder.remains < gcd\npyre if heat >= 15\nkindle",
    )
    .unwrap();
    let config = BalanceConfig {
        iterations: 512,
        threads: 4,
        base_seed: 42,
        stop_on_kill: false,
        worn: WornValues {
            purity: 375,
            ..WornValues::none()
        },
        ..BalanceConfig::default()
    };
    let reference = run(&config, &apl);
    let one_thread = run(
        &BalanceConfig {
            threads: 1,
            ..config
        },
        &apl,
    );
    assert_eq!(reference.samples, one_thread.samples);
    assert!(
        reference
            .samples
            .windows(2)
            .any(|pair| pair[0].damage != pair[1].damage),
        "must exercise unreduced random fights"
    );

    let file = TempFile::new();
    file.write(settings(4, 10, true));
    let control = Arc::new(RuntimeControl::new(4, settings(4, 100, false)).unwrap());
    let mut watcher = ControlFileWatcher::start(&file.path, Arc::clone(&control), POLL).unwrap();
    // The initial file is applied inside `start`, before any worker exists, and
    // is not a file change. No job can finish before the first written change.
    let initial_uncounted = watcher.status().applied_updates == 0;
    let worker_control = Arc::clone(&control);
    let worker = thread::spawn(move || run_controlled(&config, &apl, &worker_control));

    let initially_parked = wait_until(|| control.snapshot().throttle_events >= 4);
    let parked_before_any_job = control.snapshot().started_jobs == 0;
    fs::write(&file.path, "{\"cpu_threads\":2,").unwrap();
    let invalid_seen = wait_until(|| watcher.status().rejected_updates > 0);
    let kept_policy = control.snapshot().settings == settings(4, 10, true);
    let error_visible = watcher.status().last_error.is_some();
    file.write(settings(2, 1, false));
    let resumed = accepted(&watcher, &control, 1, settings(2, 1, false));
    let running = wait_until(|| control.snapshot().completed_jobs >= 4);
    file.write(settings(2, 1, true));
    let paused = accepted(&watcher, &control, 2, settings(2, 1, true))
        && wait_until(|| control.snapshot().active_workers == 0);
    let completed_while_paused = control.snapshot().completed_jobs;
    thread::sleep(Duration::from_millis(10));
    let stayed_paused = control.snapshot().completed_jobs == completed_while_paused;
    let had_remaining_work =
        completed_while_paused > 0 && completed_while_paused < u64::from(config.iterations);

    file.write(settings(1, 100, false));
    let final_update = accepted(&watcher, &control, 3, settings(1, 100, false));
    watcher.stop().unwrap();
    // Counts are final only after `stop` has joined the watcher thread.
    let watcher_evidence = watcher.status();
    // Always release parked workers before any assertions or joining, including
    // on a failed observer condition. A regression fails instead of hanging CI.
    control.update(settings(1, 100, false)).unwrap();
    let actual = worker.join().unwrap();
    let evidence = control.snapshot();

    assert!(initial_uncounted && initially_parked && parked_before_any_job);
    assert!(invalid_seen && kept_policy && error_visible);
    assert!(resumed && running && paused && stayed_paused && had_remaining_work && final_update);
    assert_eq!(
        watcher_evidence.applied_updates, 3,
        "exactly the three valid file changes after the initial read"
    );
    assert!(watcher_evidence.last_error.is_none());
    assert_eq!(evidence.completed_jobs, u64::from(config.iterations));
    assert_eq!(evidence.started_jobs, evidence.completed_jobs);
    assert_eq!(evidence.active_workers, 0);
    assert!(evidence.throttle_events > 0);
    assert_eq!(
        evidence.updates, 4,
        "initial file plus three file changes; the closing release repeats the last policy"
    );
    assert_eq!(actual.samples, reference.samples);
    assert_eq!(actual.casts, reference.casts);
    assert_eq!(actual.to_json(), reference.to_json());
    eprintln!("#932 CPU: full={:016x} controlled={:016x}, jobs={}, throttles={}, updates={}, max_job={:?}",
        fingerprint(&reference.to_json()), fingerprint(&actual.to_json()), evidence.completed_jobs,
        evidence.throttle_events, evidence.updates, evidence.max_job_time);
}
