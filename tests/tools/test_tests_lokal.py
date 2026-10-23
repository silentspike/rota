"""Exercise the complete test-log runner with isolated executable fixtures."""

import json
import os
import pathlib
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
RUNNER = ROOT / "tools/tests-lokal.sh"


class TestLocalTestReport(unittest.TestCase):
    def run_report(self, failed=0, *, all_names=False, silent=False,
                   log_target="target", place_binary=True):
        with tempfile.TemporaryDirectory(prefix="rota-test-report-") as directory:
            root = pathlib.Path(directory)
            (root / "sim").mkdir()
            binaries = root / "target/debug/deps"
            binaries.mkdir(parents=True)
            names = [f"test failure_{i} ... FAILED" for i in range(failed)]
            status = "FAILED" if failed else "ok"
            output = "\n".join([
                "running tests",
                "test successful_case ... ok",
                *names,
                "",
                f"test result: {status}. 1 passed; {failed} failed; 0 ignored; "
                "0 measured; 0 filtered out; finished in 0.00s",
            ])
            if place_binary:
                suite = binaries / "fixture-12345678"
                suite.write_text(
                    "#!/bin/sh\ncat <<'LIBTEST_OUTPUT'\n"
                    + output
                    + f"\nLIBTEST_OUTPUT\nexit {int(failed > 0)}\n"
                )
                suite.chmod(0o755)
            log = root / "build.log"
            # log_target "/target/current" is how cargo prints the path on the
            # cargo remote v2 service; the copied binary still lives in target/.
            log.write_text(f"Executable tests/fixture.rs ({log_target}/debug/deps/fixture-12345678)\n")
            if silent:
                extra = binaries / "silent-12345678"
                extra.write_text("#!/bin/sh\necho This is not a test result\n")
                extra.chmod(0o755)
                with log.open("a") as stream:
                    stream.write("Executable examples/silent.rs (target/debug/deps/silent-12345678)\n")
            args = ["bash", str(RUNNER)]
            if all_names:
                args.append("--alle-namen")
            return subprocess.run(
                [*args, str(log), str(root)], capture_output=True, text=True, check=False
            )

    def test_summary_is_not_a_failed_test_name(self):
        result = self.run_report(failed=1)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("test failure_0 ... FAILED", result.stdout)
        self.assertNotIn("test result:", result.stdout)
        self.assertIn("Tests bestanden:      1", result.stdout)

    def test_default_prints_three_real_names(self):
        result = self.run_report(failed=4)
        self.assertNotEqual(result.returncode, 0)
        names = [line.strip() for line in result.stdout.splitlines() if "... FAILED" in line]
        self.assertEqual(names, [f"test failure_{i} ... FAILED" for i in range(3)])

    def test_all_names_prints_every_failure(self):
        result = self.run_report(failed=4, all_names=True)
        self.assertIn("4 rote Namen:", result.stdout)
        self.assertIn("test failure_3 ... FAILED", result.stdout)
        self.assertNotIn("test result:", result.stdout)

    def test_success_has_no_failed_names(self):
        result = self.run_report()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertNotIn("FAILED", result.stdout)
        self.assertIn("Tests bestanden:      1", result.stdout)
        self.assertIn("Suiten mit Fehlern:   0", result.stdout)

    def test_silent_suite_does_not_hide_behind_successful_suite(self):
        result = self.run_report(silent=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Ohne Ergebniszeile:   1", result.stdout)

    def test_cargo_remote_v2_path_runs_the_copied_binary(self):
        result = self.run_report(log_target="/target/current")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("Aus dem Log gelesen: 1 Testbinaries", result.stdout)
        self.assertIn("Suiten ausgefuehrt:   1 von 1", result.stdout)
        self.assertIn("Tests bestanden:      1", result.stdout)

    def test_cargo_remote_v2_path_without_copy_back_is_missing(self):
        # Without `-c` nothing comes back: the v2 path must not count as found.
        result = self.run_report(log_target="/target/current", place_binary=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("FEHLT: target/debug/deps/fixture-12345678", result.stdout)
        self.assertIn("Binaries fehlten:     1", result.stdout)


JOB = "0123456789abcdef01234567"
SUITE = "#!/bin/sh\necho 'running 1 test'\necho 'test fall ... ok'\necho\n" \
        "echo 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out'\n"
MAIN = "#!/bin/sh\necho 'main() ran' >> \"$0.started\"\n"


class TestCargoRemoteV2NoRun(unittest.TestCase):
    """`--no-run` on cargo remote v2 prints no Executable lines; the job result names them."""

    def run_job(self, *, exit_code=0, args=("test", "--no-run"), write_result=True):
        with tempfile.TemporaryDirectory(prefix="rota-test-v2-") as directory:
            root = pathlib.Path(directory) / "tree"
            home = pathlib.Path(directory) / "home"
            (root / "sim").mkdir(parents=True)
            files = {
                "target/debug/deps/suite_a-0123456789abcdef": SUITE,
                "target/debug/examples/beispiel-fedcba9876543210": SUITE,
                "target/debug/rota-sim": MAIN,
                "target/debug/examples/gewoehnlich": MAIN,
            }
            for relative, script in files.items():
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(script)
                path.chmod(0o755)
            if write_result:
                state = home / ".local/state/cargo-remote/jobs" / JOB
                state.mkdir(parents=True)
                (state / "result.json").write_text(json.dumps({
                    "id": JOB, "exit_code": exit_code, "args": list(args),
                    "files": [{"path": p} for p in ["source/Cargo.lock", *files]],
                }))
            log = root / "build.log"
            log.write_text(f"cargo-remote: job {JOB} | linux-gnu-bookworm | Rust 1.98.1 | normal | workspace x\n"
                           "    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.06s\n"
                           "cargo-remote: succeeded | exit 0 | 0.80s\n")
            result = subprocess.run(["bash", str(RUNNER), str(log), str(root)], capture_output=True,
                                    text=True, check=False, env={**os.environ, "HOME": str(home)})
            started = sorted(p.name for p in root.rglob("*.started"))
            return result, started

    def test_job_result_runs_only_hashed_test_binaries(self):
        result, started = self.run_job()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(f"Aus dem Job-Ergebnis {JOB} gelesen: 2 Testbinaries", result.stdout)
        self.assertIn("Suiten ausgefuehrt:   2 von 2", result.stdout)
        self.assertIn("Tests bestanden:      2", result.stdout)
        self.assertEqual(started, [], "the program or a plain example was started with its main()")

    def test_failed_build_is_not_run(self):
        result, started = self.run_job(exit_code=101)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Exit 101", result.stderr)
        self.assertNotIn("Suiten ausgefuehrt", result.stdout)
        self.assertEqual(started, [])

    def test_non_test_job_is_rejected(self):
        result, _ = self.run_job(args=("build", "--release"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("kein Testbau", result.stderr)

    def test_missing_job_result_fails_loudly(self):
        result, _ = self.run_job(write_result=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("das Job-Ergebnis fehlt", result.stdout)


if __name__ == "__main__":
    unittest.main()
