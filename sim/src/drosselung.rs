//! Cooperative CPU control at complete-job boundaries (#932, E15.1).
//!
//! This module controls scheduling only. It never receives a seed, combat
//! state, result, archive, or campaign fingerprint. A permit covers one whole
//! fight; dropping it makes its slot available to any waiting worker. Fixed
//! worker identities must not be parked: workers with unfinished static ranges
//! would otherwise remain asleep after the enabled-thread count decreases.
//!
//! `cpu_percent` is measured work/rest pacing per compute slot, not an OS CPU
//! quota. Job duration is wall time, so descheduling can make pacing conservative.
//! Reaction is bounded by the current job plus control-file polling, not by a
//! claimed 25-ms quantum. PSI, cgroups, GPU control, and memory admission/eviction
//! remain separate unimplemented parts of #932.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Runtime policy, deliberately separate from semantic run configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlSettings {
    /// Maximum simultaneous compute slots; zero parks all compute workers.
    pub cpu_threads: usize,
    /// Work fraction per enabled slot, from 1 to 100 percent.
    pub cpu_percent: u8,
    /// Park new jobs; already started jobs finish normally.
    pub paused: bool,
}

/// Invalid policy, unreadable file, or failed watcher lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlError(String);

impl fmt::Display for ControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ControlError {}

impl ControlSettings {
    fn validate(self, max_workers: usize) -> Result<Self, ControlError> {
        if max_workers == 0 || self.cpu_threads > max_workers {
            return Err(ControlError(format!(
                "cpu_threads must be between 0 and {max_workers}; worker capacity must be positive"
            )));
        }
        if !(1..=100).contains(&self.cpu_percent) {
            return Err(ControlError("cpu_percent must be between 1 and 100".into()));
        }
        Ok(self)
    }
}

/// Observed scheduling evidence; these counters are not result fingerprints.
#[derive(Debug, Clone, Copy)]
pub struct ControlSnapshot {
    /// Most recently accepted scheduling policy.
    pub settings: ControlSettings,
    /// Complete jobs currently holding a compute slot.
    pub active_workers: usize,
    /// Highest observed simultaneous slot occupancy since creation.
    pub peak_active_workers: usize,
    /// Permits issued since creation.
    pub started_jobs: u64,
    /// Released permits, including unwinding after a failed job.
    pub completed_jobs: u64,
    /// Acquisitions which actually waited, including pause and pacing waits.
    pub throttle_events: u64,
    /// Successful policy changes, excluding repeated identical settings.
    pub updates: u64,
    /// Longest complete job observed; the current worker reaction granularity.
    pub max_job_time: Duration,
}

#[derive(Debug)]
struct Slot {
    busy: bool,
    finished: Option<Instant>,
    work: Duration,
}

impl Slot {
    fn remaining_rest(&self, percent: u8) -> Duration {
        let Some(finished) = self.finished else {
            return Duration::ZERO;
        };
        let rest = self.work.saturating_mul(u32::from(100 - percent)) / u32::from(percent);
        rest.saturating_sub(finished.elapsed())
    }
}

#[derive(Debug)]
struct State {
    snapshot: ControlSnapshot,
    slots: Vec<Slot>,
}

/// Shared gate for existing CPU workers. The watcher never acquires this gate.
#[derive(Debug)]
pub struct RuntimeControl {
    state: Mutex<State>,
    changed: Condvar,
    max_workers: usize,
}

impl RuntimeControl {
    /// Create the gate with a fixed worker capacity and validated initial policy.
    pub fn new(max_workers: usize, settings: ControlSettings) -> Result<Self, ControlError> {
        let settings = settings.validate(max_workers)?;
        Ok(Self {
            state: Mutex::new(State {
                snapshot: ControlSnapshot {
                    settings,
                    active_workers: 0,
                    peak_active_workers: 0,
                    started_jobs: 0,
                    completed_jobs: 0,
                    throttle_events: 0,
                    updates: 0,
                    max_job_time: Duration::ZERO,
                },
                slots: (0..max_workers)
                    .map(|_| Slot {
                        busy: false,
                        finished: None,
                        work: Duration::ZERO,
                    })
                    .collect(),
            }),
            changed: Condvar::new(),
            max_workers,
        })
    }

    /// Wait until a complete job can start. Keep the permit through the job.
    pub fn acquire(&self) -> WorkPermit<'_> {
        let mut state = self.state.lock().expect("CPU control mutex poisoned");
        let mut counted_wait = false;
        loop {
            let settings = state.snapshot.settings;
            let mut wait_for = None;
            if !settings.paused && state.snapshot.active_workers < settings.cpu_threads {
                for index in 0..settings.cpu_threads {
                    let slot = &state.slots[index];
                    if slot.busy {
                        continue;
                    }
                    let remaining = slot.remaining_rest(settings.cpu_percent);
                    if remaining.is_zero() {
                        state.slots[index].busy = true;
                        state.snapshot.active_workers += 1;
                        state.snapshot.peak_active_workers = state
                            .snapshot
                            .peak_active_workers
                            .max(state.snapshot.active_workers);
                        state.snapshot.started_jobs += 1;
                        return WorkPermit {
                            control: self,
                            slot: index,
                            started: Instant::now(),
                        };
                    }
                    wait_for =
                        Some(wait_for.map_or(remaining, |prior: Duration| prior.min(remaining)));
                }
            }
            if !counted_wait {
                state.snapshot.throttle_events += 1;
                counted_wait = true;
            }
            state = if let Some(duration) = wait_for {
                self.changed
                    .wait_timeout(state, duration)
                    .expect("CPU control mutex poisoned")
                    .0
            } else {
                self.changed
                    .wait(state)
                    .expect("CPU control mutex poisoned")
            };
        }
    }

    /// Replace the entire policy atomically and wake parked/paced workers.
    pub fn update(&self, settings: ControlSettings) -> Result<(), ControlError> {
        let settings = settings.validate(self.max_workers)?;
        let mut state = self.state.lock().expect("CPU control mutex poisoned");
        if state.snapshot.settings != settings {
            state.snapshot.settings = settings;
            state.snapshot.updates += 1;
            self.changed.notify_all();
        }
        Ok(())
    }

    /// Observe progress even while all compute workers are paused.
    #[must_use]
    pub fn snapshot(&self) -> ControlSnapshot {
        self.state
            .lock()
            .expect("CPU control mutex poisoned")
            .snapshot
    }
}

/// RAII slot ownership. Releasing a slot records work and starts its rest time.
#[derive(Debug)]
#[must_use = "hold this permit for the entire compute job"]
pub struct WorkPermit<'a> {
    control: &'a RuntimeControl,
    slot: usize,
    started: Instant,
}

impl Drop for WorkPermit<'_> {
    fn drop(&mut self) {
        let work = self.started.elapsed();
        let mut state = self
            .control
            .state
            .lock()
            .expect("CPU control mutex poisoned");
        state.slots[self.slot] = Slot {
            busy: false,
            finished: Some(Instant::now()),
            work,
        };
        state.snapshot.active_workers -= 1;
        state.snapshot.completed_jobs += 1;
        state.snapshot.max_job_time = state.snapshot.max_job_time.max(work);
        self.control.changed.notify_all();
    }
}

fn whitespace(text: &str) -> &str {
    text.trim_matches([' ', '\t', '\r', '\n'])
}

fn unsigned(text: &str) -> Result<usize, ControlError> {
    if text.is_empty()
        || !text.bytes().all(|byte| byte.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err(ControlError("expected a JSON unsigned integer".into()));
    }
    text.parse()
        .map_err(|_| ControlError("control integer is too large".into()))
}

// This intentionally small JSON contract has exactly three scalar fields. A
// duplicate, unknown key, missing field or non-JSON number is an error; no
// partial file is ever applied. No dependency is added to the simulation crate.
fn parse_settings(text: &str, max_workers: usize) -> Result<ControlSettings, ControlError> {
    let object = whitespace(text)
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .ok_or_else(|| ControlError("control file must be one JSON object".into()))?;
    let mut cpu_threads = None;
    let mut cpu_percent = None;
    let mut paused = None;
    for pair in object.split(',') {
        let (key, value) = pair
            .split_once(':')
            .ok_or_else(|| ControlError("control field requires a key and value".into()))?;
        let value = whitespace(value);
        match whitespace(key) {
            "\"cpu_threads\"" if cpu_threads.is_none() => cpu_threads = Some(unsigned(value)?),
            "\"cpu_percent\"" if cpu_percent.is_none() => {
                cpu_percent =
                    Some(u8::try_from(unsigned(value)?).map_err(|_| {
                        ControlError("cpu_percent must be between 1 and 100".into())
                    })?);
            }
            "\"paused\"" if paused.is_none() => {
                paused = Some(match value {
                    "true" => true,
                    "false" => false,
                    _ => return Err(ControlError("paused must be a JSON boolean".into())),
                });
            }
            _ => {
                return Err(ControlError(format!(
                    "unknown or duplicate control key: {}",
                    whitespace(key)
                )))
            }
        }
    }
    ControlSettings {
        cpu_threads: cpu_threads.ok_or_else(|| ControlError("missing cpu_threads".into()))?,
        cpu_percent: cpu_percent.ok_or_else(|| ControlError("missing cpu_percent".into()))?,
        paused: paused.ok_or_else(|| ControlError("missing paused".into()))?,
    }
    .validate(max_workers)
}

fn read_settings(
    path: &Path,
    max_workers: usize,
) -> Result<(String, ControlSettings), ControlError> {
    let mut text = String::new();
    File::open(path)
        .and_then(|file| file.take(4097).read_to_string(&mut text))
        .map_err(|error| ControlError(format!("{}: {error}", path.display())))?;
    if text.len() > 4096 {
        return Err(ControlError("control file exceeds 4096 bytes".into()));
    }
    let settings = parse_settings(&text, max_workers)?;
    Ok((text, settings))
}

/// Control-plane activity, independent of compute-slot availability.
#[derive(Debug, Clone, Default)]
pub struct WatchStatus {
    /// Changed complete file contents accepted after the initial read.
    pub applied_updates: u64,
    /// Failed polling reads or validations, including repeated failures.
    pub rejected_updates: u64,
    /// Latest polling error; cleared by the next valid read.
    pub last_error: Option<String>,
}

#[derive(Debug, Default)]
struct WatchState {
    stop: bool,
    status: WatchStatus,
}

/// Poll a validated complete JSON file on a separate control thread.
///
/// Initial invalid input is returned to the caller. Later read/parse failures
/// retain the last accepted policy and are observable through `status`. Atomic
/// rename is the recommended writer protocol. Dropping this guard stops and
/// joins the watcher; it does not unpause or otherwise alter the compute policy.
#[derive(Debug)]
pub struct ControlFileWatcher {
    state: Arc<(Mutex<WatchState>, Condvar)>,
    handle: Option<JoinHandle<()>>,
}

impl ControlFileWatcher {
    /// Validate the initial file and start a watcher that never occupies a compute slot.
    pub fn start(
        path: impl AsRef<Path>,
        control: Arc<RuntimeControl>,
        poll_interval: Duration,
    ) -> Result<Self, ControlError> {
        Self::start_inner(path, control, poll_interval, None)
    }

    /// Also report each distinct polling error while compute workers may be paused.
    /// The handler runs outside both locks on the watcher thread and must return promptly.
    pub fn start_with_error_handler(
        path: impl AsRef<Path>,
        control: Arc<RuntimeControl>,
        poll_interval: Duration,
        handler: fn(&str),
    ) -> Result<Self, ControlError> {
        Self::start_inner(path, control, poll_interval, Some(handler))
    }

    fn start_inner(
        path: impl AsRef<Path>,
        control: Arc<RuntimeControl>,
        poll_interval: Duration,
        handler: Option<fn(&str)>,
    ) -> Result<Self, ControlError> {
        if poll_interval < Duration::from_millis(1) || poll_interval > Duration::from_secs(1) {
            return Err(ControlError(
                "control polling must be between 1 and 1000 ms".into(),
            ));
        }
        let path = path.as_ref().to_path_buf();
        let (initial_text, initial_settings) = read_settings(&path, control.max_workers)?;
        control.update(initial_settings)?;
        let state = Arc::new((Mutex::new(WatchState::default()), Condvar::new()));
        let shared = Arc::clone(&state);
        let handle = thread::Builder::new()
            .name("rota-sim-control".into())
            .spawn(move || watch_file(path, control, poll_interval, initial_text, &shared, handler))
            .map_err(|error| ControlError(format!("cannot start control watcher: {error}")))?;
        Ok(Self {
            state,
            handle: Some(handle),
        })
    }

    #[must_use]
    /// Observe accepted/rejected reads independently of compute progress.
    ///
    /// A change reaches `RuntimeControl` before it is counted here, so a live
    /// reader can see the new policy while `applied_updates` still lags by one.
    /// The counts are final once [`Self::stop`] has joined the watcher thread.
    pub fn status(&self) -> WatchStatus {
        self.state
            .0
            .lock()
            .expect("control watcher mutex poisoned")
            .status
            .clone()
    }

    /// Stop immediately, waking the polling timer, and join once.
    pub fn stop(&mut self) -> Result<(), ControlError> {
        {
            let mut state = self.state.0.lock().expect("control watcher mutex poisoned");
            state.stop = true;
            self.state.1.notify_all();
        }
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| ControlError("control watcher panicked".into()))?;
        }
        Ok(())
    }
}

impl Drop for ControlFileWatcher {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn watch_file(
    path: PathBuf,
    control: Arc<RuntimeControl>,
    poll_interval: Duration,
    mut accepted_text: String,
    shared: &(Mutex<WatchState>, Condvar),
    handler: Option<fn(&str)>,
) {
    loop {
        let state = shared.0.lock().expect("control watcher mutex poisoned");
        let (state, _) = shared
            .1
            .wait_timeout_while(state, poll_interval, |state| !state.stop)
            .expect("control watcher mutex poisoned");
        if state.stop {
            break;
        }
        drop(state);

        let next = read_settings(&path, control.max_workers);
        let update = match next {
            Ok((text, settings)) if text != accepted_text => match control.update(settings) {
                Ok(()) => {
                    accepted_text = text;
                    Ok(true)
                }
                Err(error) => Err(error),
            },
            Ok(_) => Ok(false),
            Err(error) => Err(error),
        };
        let mut state = shared.0.lock().expect("control watcher mutex poisoned");
        match update {
            Ok(changed) => {
                if changed {
                    state.status.applied_updates += 1;
                }
                state.status.last_error = None;
            }
            Err(error) => {
                let message = error.to_string();
                let changed = state.status.last_error.as_deref() != Some(message.as_str());
                state.status.rejected_updates += 1;
                state.status.last_error = Some(message.clone());
                drop(state);
                if changed {
                    if let Some(report) = handler {
                        report(&message);
                    }
                }
            }
        }
    }
}
