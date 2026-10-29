"""Run tools/auslieferung-pruefdaten.py against planted builds and the real data (issue #1177)."""

import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
TOOL = ROOT / "tools/auslieferung-pruefdaten.py"


def first_test_body():
    """The body of the first archive test record, as the bundle would carry it."""
    document = json.loads((ROOT / "data/archive/test-entries.json").read_text(encoding="utf-8"))
    return document["entries"][0]["body"]


def first_catalogue_body():
    """A text that ships: the first entry of the archive catalogue."""
    document = json.loads((ROOT / "data/archive/archive.json").read_text(encoding="utf-8"))
    return document["entries"][0]["body"]


class Auslieferung(unittest.TestCase):
    def run_tool(self, files):
        with tempfile.TemporaryDirectory(prefix="rota-auslieferung-") as directory:
            dist = pathlib.Path(directory)
            for name, content in files.items():
                (dist / name).parent.mkdir(parents=True, exist_ok=True)
                (dist / name).write_text(content, encoding="utf-8")
            return subprocess.run(
                [sys.executable, "-B", str(TOOL), "--root", str(ROOT), "--dist", str(dist)],
                capture_output=True, text=True, check=False,
            )

    def test_planted_test_record_is_found_by_name(self):
        result = self.run_tool({"assets/index.js": f'const a="{first_test_body()}";'})
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("FUND assets/index.js", result.stdout)

    def test_planted_test_id_is_found(self):
        result = self.run_tool({"assets/index.js": 'const a={id:"test-after-kill"};'})
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("test-after-kill", result.stdout)

    def test_clean_build_passes_and_counts_its_markers(self):
        result = self.run_tool({"assets/index.js": "const a=1;"})
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertRegex(result.stdout, r"^0 Funde, [1-9][0-9]* Merkmale")

    def test_catalogue_text_is_not_a_finding(self):
        # A sentence that ships in the catalogue must not count as test data,
        # even if a test record happened to share it.
        result = self.run_tool({"assets/index.js": f'const a="{first_catalogue_body()}";'})
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_missing_build_is_not_a_pass(self):
        with tempfile.TemporaryDirectory(prefix="rota-auslieferung-") as directory:
            leer = pathlib.Path(directory) / "dist"
            result = subprocess.run(
                [sys.executable, "-B", str(TOOL), "--root", str(ROOT), "--dist", str(leer)],
                capture_output=True, text=True, check=False,
            )
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)

    def test_markers_cover_every_archive_test_record(self):
        # Every archive test record contributes at least one marker; otherwise
        # one of them could ship unnoticed.
        sys.path.insert(0, str(ROOT / "tools"))
        try:
            import importlib.util

            spec = importlib.util.spec_from_file_location("auslieferung", TOOL)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
        finally:
            sys.path.pop(0)
        marks = module.markers(ROOT)
        document = json.loads((ROOT / "data/archive/test-entries.json").read_text(encoding="utf-8"))
        for entry in document["entries"]:
            self.assertIn(entry["id"], marks)
            self.assertTrue(any(m in entry["body"] for m in marks), entry["id"])


if __name__ == "__main__":
    unittest.main()
