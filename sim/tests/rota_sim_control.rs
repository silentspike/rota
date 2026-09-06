//! Real CLI control-file checks, including pause and reload through the process boundary.
mod gemeinsam;

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::thread;
use std::time::{Duration, Instant};

struct TempControl(PathBuf);
impl TempControl {
    fn new(text: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let base = std::env::var_os("RUNNER_TEMP")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let path = base.join(format!(
            "rota-control-{}-{}.json",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, text).unwrap();
        Self(path)
    }
    fn replace(&self, text: &str) {
        let next = self.0.with_extension("next");
        fs::write(&next, text).unwrap();
        fs::rename(next, &self.0).unwrap();
    }
}
impl Drop for TempControl {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
struct Running(Option<Child>);
impl Drop for Running {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn invalid_control_file_is_rejected_before_any_result() {
    let file = TempControl::new("{\"cpu_threads\":1,\"paused\":false}");
    let (code, out, error) = gemeinsam::rufe(&[
        "run",
        "--search",
        "none",
        "--control-file",
        file.0.to_str().unwrap(),
        "--json",
    ]);
    assert_ne!(code, 0);
    assert!(error.contains("cpu_percent"), "{error}");
    assert!(!out.contains("\"result\""), "{out}");
}

#[test]
fn unsupported_search_never_silently_ignores_control_file() {
    let (code, out, error) = gemeinsam::rufe(&[
        "run",
        "--control-file",
        "does-not-exist.json",
        "--iterations",
        "1",
    ]);
    assert_ne!(code, 0);
    assert!(error.contains("--search none"), "{error}");
    assert!(!out.contains("\"result\""));
}

#[test]
fn paused_cli_resumes_from_file_and_returns_the_reference_result() {
    let args = [
        "run",
        "--search",
        "none",
        "--threads",
        "2",
        "--iterations",
        "6",
        "--json",
    ];
    let (code, reference, error) = gemeinsam::rufe(&args);
    assert_eq!(code, 0, "{error}");
    let reference: serde_json::Value = serde_json::from_str(&reference).unwrap();
    let file = TempControl::new("{\"cpu_threads\":2,\"cpu_percent\":100,\"paused\":true}");
    let child = Command::new(gemeinsam::programm())
        .args(args)
        .args(["--control-file", file.0.to_str().unwrap()])
        .current_dir(gemeinsam::wurzel())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut child = Running(Some(child));
    let stderr = child.0.as_mut().unwrap().stderr.take().unwrap();
    let (send, receive) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut messages = String::new();
        for line in BufReader::new(stderr).lines() {
            let line = line.unwrap();
            if line.starts_with("Runtime control active:") {
                let _ = send.send("active");
            }
            if line.starts_with("Control-file update rejected:") {
                let _ = send.send("rejected");
            }
            messages.push_str(&line);
            messages.push('\n');
        }
        messages
    });
    assert_eq!(
        receive
            .recv_timeout(Duration::from_secs(10))
            .expect("CLI never activated its watcher"),
        "active"
    );
    thread::sleep(Duration::from_millis(200));
    assert!(
        child.0.as_mut().unwrap().try_wait().unwrap().is_none(),
        "paused job finished"
    );
    // An incomplete update must retain the paused policy.
    file.replace("{\"paused\":false}");
    assert_eq!(
        receive
            .recv_timeout(Duration::from_secs(10))
            .expect("paused CLI never reported the invalid resume file"),
        "rejected"
    );
    thread::sleep(Duration::from_millis(200));
    assert!(
        receive.try_recv().is_err(),
        "unchanged file error was repeatedly printed"
    );
    assert!(
        child.0.as_mut().unwrap().try_wait().unwrap().is_none(),
        "partial update resumed the job"
    );
    file.replace("{\"cpu_threads\":1,\"cpu_percent\":50,\"paused\":false}");
    let deadline = Instant::now() + Duration::from_secs(10);
    while child.0.as_mut().unwrap().try_wait().unwrap().is_none() {
        assert!(
            Instant::now() < deadline,
            "CLI did not resume after valid update"
        );
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.0.take().unwrap().wait_with_output().unwrap();
    let messages = reader.join().unwrap();
    assert!(output.status.success(), "{messages}");
    assert!(
        messages.contains("1 jobs completed"),
        "deterministic reduction changed: {messages}"
    );
    assert!(messages.contains("rejected file updates"), "{messages}");
    assert!(
        !messages.contains(" / 0 rejected file updates"),
        "incomplete update was not observed: {messages}"
    );
    let controlled: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(controlled["result"], reference["result"]);
}
