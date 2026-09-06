"""Exercise #895 through the real CLI, including deliberately invalid registries."""

import copy
import importlib.util
import itertools
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('sim_registry', ROOT / 'tools/sim-registry.py')
REGISTRY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(REGISTRY)


class SimRegistry(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = json.loads(REGISTRY.REGISTRY.read_text())
        cls.plan = REGISTRY.PLAN.read_text()

    def setUp(self):
        self.registry = copy.deepcopy(self.source)
        self.entries = {e['id']: e for e in self.registry['entries']}

    def cli(self, registry=None, plan=None, mode='--check'):
        # The historical issue requires temporary artifacts under /work/tmp.
        temp_root = Path('/work/tmp')
        temp_root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='rota-registry-', dir=temp_root) as temporary:
            directory = Path(temporary)
            data = directory / 'registry.json'
            document = directory / 'plan.md'
            data.write_text(json.dumps(registry if registry is not None else self.registry))
            document.write_text(self.plan if plan is None else plan)
            result = subprocess.run(
                [sys.executable, '-B', str(ROOT / 'tools/sim-registry.py'), mode,
                 '--registry', str(data), '--plan', str(document)],
                capture_output=True, text=True, check=False,
            )
            return result, document.read_text()

    def assert_rejected(self, message, plan=None):
        result, _ = self.cli(plan=plan)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn(message, result.stderr)

    def test_actual_registry_and_tables_pass_and_list_missing_task_ids(self):
        result, _ = self.cli()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('75 tasks, 1 forecasts', result.stdout)
        self.assertIn('88 headings, 53 task-id headings', result.stdout)
        self.assertIn('Historical baseline (#895): 91 headings / 58 task-id headings', result.stdout)
        missing = [line for line in result.stdout.splitlines()
                   if line.startswith('NORMATIVE WITHOUT TASK ID:')]
        self.assertEqual(len(missing), 25)
        self.assertTrue(any('e14-der-werte-deckel' in line for line in missing))
        self.assertTrue(any('e13-die-cuda-pipeline' in line for line in missing))
        self.assertIn('Herstellerkennung fehlt', result.stdout)

    def test_ac2_duplicate_id_is_rejected_by_cli(self):
        self.registry['entries'].append(copy.deepcopy(self.entries['E11.0']))
        self.assert_rejected('duplicate id: E11.0')

    def test_ac2_unknown_dependency_is_rejected_by_cli(self):
        self.entries['E11.0']['depends_on'] = ['E999.0']
        self.assert_rejected('unknown dependency E999.0')

    def test_ac2_dependency_cycle_is_rejected_by_cli(self):
        self.entries['E11.0']['depends_on'] = ['E11.0b']
        self.assert_rejected('dependency cycle: E11.0 -> E11.0b -> E11.0')

    def test_ac2_counted_task_without_section_is_rejected_by_cli(self):
        self.entries['E11.0']['normative_section'] = 'nonexistent-section'
        self.assert_rejected('missing normative section nonexistent-section')

    def test_ac2_ready_without_acceptance_is_rejected_by_cli(self):
        self.entries['E11.0']['status'] = 'acceptance_ready'
        self.entries['E11.0']['acceptance'] = []
        self.assert_rejected('acceptance_ready without acceptance criteria')

    def test_manual_drift_in_each_table_is_rejected_by_cli(self):
        for name in REGISTRY.TABLES:
            with self.subTest(table=name):
                marker = f'<!-- sim-registry:{name}:start -->\n\n'
                changed = self.plan.replace(marker, marker + '| invented count |\n', 1)
                self.assert_rejected('generated table drift', changed)

    def test_write_restores_only_generated_tables_and_is_idempotent(self):
        marker = '<!-- sim-registry:epics:start -->\n\n'
        changed = self.plan.replace(marker, marker + '| invented count |\n', 1)
        result, restored = self.cli(plan=changed, mode='--write')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(restored, self.plan)
        second, unchanged = self.cli(plan=restored, mode='--write')
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(unchanged, restored)

    def test_shared_historical_task_cannot_point_to_unrelated_existing_section(self):
        self.entries['E20.1']['normative_section'] = self.entries['E11.0']['normative_section']
        self.assert_rejected('E20.1: missing normative evidence inside shared section')

    def test_historical_package_without_own_heading_cannot_disappear(self):
        self.registry['entries'] = [e for e in self.registry['entries'] if e['id'] != 'E20.1']
        self.assert_rejected('missing historical issue package: E20.1')

    def test_new_numbered_and_unnumbered_headings_require_registry_entries(self):
        for heading, message in [
            ('### E99.0 — Neues Paket', 'unregistered package heading: E99.0'),
            ('### Neue verbindliche Regel', 'unclassified heading without task id'),
        ]:
            with self.subTest(heading=heading):
                plan = self.plan.replace('## Teil 5', heading + '\n\n## Teil 5', 1)
                self.assert_rejected(message, plan)

    def test_every_pair_of_seven_kernel_writers_conflicts_including_cross_epic(self):
        writers = ['E11.0', 'E11.1', 'E11.2', 'E11.4', 'E11.4d', 'E11.7b', 'E18.1']
        for left, right in itertools.combinations(writers, 2):
            with self.subTest(left=left, right=right):
                errors = REGISTRY.validate(self.registry, self.plan, {left: 'alice', right: 'bob'})
                self.assertTrue(any('ownership conflict: sim/src/fight.rs' in e for e in errors), errors)

    def test_explicit_nonwriters_do_not_take_the_kernel_lock(self):
        assignments = dict.fromkeys(['E11.0', 'E20.0', 'E16.0', 'E19.0', 'E21.0', 'E11.0b'], 'worker')
        self.assertEqual(REGISTRY.validate(self.registry, self.plan, assignments), [])
        # The scope is explicit: passing this rule does not prove that all files are disjoint.
        self.assertIn('nicht vollständig', self.registry['ownership_scope'])

    def test_unknown_assignment_scope_and_forbidden_write_are_rejected(self):
        errors = REGISTRY.validate(self.registry, self.plan, {'E999.0': 'alice', 'E15.6': 'bob'})
        self.assertTrue(any('unknown package' in e for e in errors))
        self.assertTrue(any('ownership unresolved' in e for e in errors))
        self.entries['E20.0']['writes'] = ['sim/src/fight.rs']
        errors = REGISTRY.validate(self.registry, self.plan)
        self.assertTrue(any('file both writable and forbidden' in e for e in errors))

    def test_fenced_examples_are_not_sections_and_duplicate_slugs_are_distinct(self):
        document = '# Wirkungs-IR\n```md\n# Phantom\n```\n# Wirkungs-IR\n'
        self.assertEqual([h['anchor'] for h in REGISTRY.headings(document)],
                         ['wirkungs-ir', 'wirkungs-ir-1'])


if __name__ == '__main__':
    unittest.main()
