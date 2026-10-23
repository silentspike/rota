"""Run tools/rust-pin.py as the CI does, against isolated file pairs and the real tree."""

import pathlib
import subprocess
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
TOOL = ROOT / "tools/rust-pin.py"

KETTE = '[toolchain]\nchannel = "{kanal}"\nprofile = "minimal"\ncomponents = ["rustfmt", "clippy"]\n'
DIENST = ('# Kommentar ueber dem Abschnitt\n\n[cargo_remote]\nschema = 1\nproject_id = "x"\n'
          'profile = "linux-gnu-bookworm"\ntoolchain = "{kanal}"\ntargets = []\nresources = "normal"\n')


class RustPin(unittest.TestCase):
    def run_tool(self, kette=None, dienst=None, *extra, root=None):
        with tempfile.TemporaryDirectory(prefix="rota-rust-pin-") as directory:
            wurzel = pathlib.Path(directory)
            if kette is not None:
                (wurzel / "rust-toolchain.toml").write_text(kette)
            if dienst is not None:
                (wurzel / ".cargo-remote.toml").write_text(dienst)
            return subprocess.run(
                [sys.executable, "-B", str(TOOL), "--root", str(root or wurzel), *extra],
                capture_output=True, text=True, check=False,
            )

    def test_matching_pins_print_only_the_version(self):
        result = self.run_tool(KETTE.format(kanal="1.98.1"), DIENST.format(kanal="1.98.1"))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "1.98.1\n")
        self.assertEqual(result.stderr, "")

    def test_mismatch_names_both_versions(self):
        result = self.run_tool(KETTE.format(kanal="1.98.1"), DIENST.format(kanal="1.99.0"))
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")
        self.assertIn("1.98.1", result.stderr)
        self.assertIn("1.99.0", result.stderr)

    def test_floating_channel_is_not_a_pin(self):
        result = self.run_tool(KETTE.format(kanal="stable"), DIENST.format(kanal="1.98.1"))
        self.assertEqual(result.returncode, 1)
        self.assertIn("keine exakte Version", result.stderr)

    def test_two_part_version_is_not_a_pin(self):
        result = self.run_tool(KETTE.format(kanal="1.98"), DIENST.format(kanal="1.98"))
        self.assertEqual(result.returncode, 1)
        self.assertIn("keine exakte Version", result.stderr)

    def test_missing_toolchain_file_fails(self):
        result = self.run_tool(None, DIENST.format(kanal="1.98.1"))
        self.assertEqual(result.returncode, 1)
        self.assertIn("rust-toolchain.toml fehlt", result.stderr)

    def test_legacy_only_configuration_fails(self):
        legacy = '[[remote]]\nhost = "root@192.0.2.1"\ntemp_dir = "/tmp/builds"\n'
        result = self.run_tool(KETTE.format(kanal="1.98.1"), legacy)
        self.assertEqual(result.returncode, 1)
        self.assertIn("0-mal", result.stderr)

    def test_header_inside_a_comment_is_rejected(self):
        # Gemessen am echten Client: Mit der Kopfzeile in einem Kommentar
        # schreibt er den Abschnitt nach dem naechsten Lauf doppelt.
        dienst = "# Der Abschnitt [cargo_remote] wird verwaltet\n" + DIENST.format(kanal="1.98.1")
        result = self.run_tool(KETTE.format(kanal="1.98.1"), dienst)
        self.assertEqual(result.returncode, 1)
        self.assertIn("2-mal", result.stderr)

    def test_unsupported_schema_fails(self):
        dienst = DIENST.format(kanal="1.98.1").replace("schema = 1", "schema = 2")
        result = self.run_tool(KETTE.format(kanal="1.98.1"), dienst)
        self.assertEqual(result.returncode, 1)
        self.assertIn("schema", result.stderr)

    def test_running_compiler_must_equal_the_pin(self):
        paar = (KETTE.format(kanal="1.98.1"), DIENST.format(kanal="1.98.1"))
        gleich = self.run_tool(*paar, "--rustc", "rustc 1.98.1 (0123456ab 2026-09-01)")
        self.assertEqual(gleich.returncode, 0, gleich.stderr)
        anders = self.run_tool(*paar, "--rustc", "rustc 1.99.1 (58cf8f9c3 2026-10-13)")
        self.assertEqual(anders.returncode, 1)
        self.assertIn("Es läuft rustc 1.99.1", anders.stderr)

    def test_version_prefix_is_not_a_match(self):
        # "1.98.10" beginnt mit "1.98.1" — ein Praefixvergleich hielte das fuer gleich.
        paar = (KETTE.format(kanal="1.98.1"), DIENST.format(kanal="1.98.1"))
        result = self.run_tool(*paar, "--rustc", "rustc 1.98.10 (0123456ab 2027-01-01)")
        self.assertEqual(result.returncode, 1)
        self.assertIn("1.98.10", result.stderr)

    def test_unreadable_rustc_output_fails(self):
        paar = (KETTE.format(kanal="1.98.1"), DIENST.format(kanal="1.98.1"))
        result = self.run_tool(*paar, "--rustc", "")
        self.assertEqual(result.returncode, 1)
        self.assertIn("keine lesbare Version", result.stderr)

    def test_checked_in_pins_agree(self):
        # Die Positivkontrolle am echten Baum: Ohne sie koennten alle Faelle oben
        # gruen sein, waehrend die eingecheckten Dateien auseinanderlaufen.
        result = subprocess.run([sys.executable, "-B", str(TOOL)], capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertRegex(result.stdout, r"^\d+\.\d+\.\d+\n$")


if __name__ == "__main__":
    unittest.main()
