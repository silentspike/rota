"""Exercise the complete test-log runner with isolated executable fixtures."""

import pathlib
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
RUNNER = ROOT / "tools/tests-lokal.sh"


class TestLocalTestReport(unittest.TestCase):
    def run_report(self, failed=0, *, all_names=False, silent=False):
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
            suite = binaries / "fixture-12345678"
            suite.write_text(
                "#!/bin/sh\ncat <<'LIBTEST_OUTPUT'\n"
                + output
                + f"\nLIBTEST_OUTPUT\nexit {int(failed > 0)}\n"
            )
            suite.chmod(0o755)
            log = root / "build.log"
            log.write_text("Executable tests/fixture.rs (target/debug/deps/fixture-12345678)\n")
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


if __name__ == "__main__":
    unittest.main()
