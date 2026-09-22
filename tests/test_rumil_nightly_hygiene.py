import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / 'scripts/rumil_nightly_hygiene.py'

class NightlyHygieneTests(unittest.TestCase):
    def run_audit(self, root, *args):
        result = subprocess.run([sys.executable, str(SCRIPT), '--root', str(root), *args], capture_output=True, text=True)
        self.assertTrue((root / 'data/rumil/hygiene/latest.json').exists(), result.stderr)
        return result, json.loads((root / 'data/rumil/hygiene/latest.json').read_text())

    def test_findings_are_not_execution_failures_and_repeat_is_quiet(self):
        with tempfile.TemporaryDirectory() as d:
            r = Path(d)
            (r/'bad.py').write_text('def broken(:\n')
            a, first = self.run_audit(r)
            self.assertEqual(a.returncode, 0)
            self.assertEqual(first['outcome'], 'findings')
            self.assertEqual(len(first['changes']['new']), 1)
            b, second = self.run_audit(r)
            self.assertEqual(b.returncode, 0)
            self.assertEqual(second['changes']['new'], [])
            self.assertEqual(b.stdout, '')
            self.assertEqual(first['findings'][0]['id'], second['findings'][0]['id'])
            (r/'bad.py').write_text('x = 1\n')
            c, third = self.run_audit(r)
            self.assertEqual(len(third['changes']['resolved']), 1)
            self.assertIn('resolved=1', c.stdout)

    def test_all_roots_metadata_and_sensitive_generated_content_boundaries(self):
        with tempfile.TemporaryDirectory() as d, tempfile.TemporaryDirectory() as outside:
            r=Path(d)
            for rel in ['crates/a.rs','apps/a.ts','vendor/bad.py','target/a.bin','.git/config','config/.env','docs/archive/a.md','books/a.txt']:
                p=r/rel; p.parent.mkdir(parents=True, exist_ok=True); p.write_text('SECRET invalid syntax')
            (r/'outside').symlink_to(outside, target_is_directory=True)
            (Path(outside)/'bad.py').write_text('def broken(:')
            result, data=self.run_audit(r)
            self.assertEqual(result.returncode,0)
            self.assertEqual(data['coverage']['files'],8)
            self.assertEqual(data['coverage']['symlinks'],1)
            self.assertEqual(data['findings'],[])
            self.assertNotIn('SECRET', json.dumps(data))
            self.assertEqual(set(data['coverage']['roots']),{'crates','apps','vendor','target','.git','config','docs','books','outside'})

    def test_broken_link_grace_move_hint_and_strict_gate(self):
        with tempfile.TemporaryDirectory() as d:
            r=Path(d); (r/'README.md').write_text('[plan](docs/plans/old.md)\n')
            p=r/'docs/archive/old.md'; p.parent.mkdir(parents=True); p.write_text('# old\n')
            _, first=self.run_audit(r)
            self.assertEqual(first['proposals'],[])
            _, second=self.run_audit(r)
            self.assertEqual(len(second['proposals']),1)
            self.assertEqual(second['findings'][0]['suggested_targets'],['docs/archive/old.md'])
            self.assertFalse(second['policy']['execution_performed'])
            strict, _=self.run_audit(r,'--strict')
            self.assertEqual(strict.returncode,1)
            self.assertEqual((r/'README.md').read_text(),'[plan](docs/plans/old.md)\n')

    def test_cap_is_partial_and_does_not_resolve_previous_findings(self):
        with tempfile.TemporaryDirectory() as d:
            r=Path(d); (r/'bad.py').write_text('def broken(:')
            self.run_audit(r)
            result, data=self.run_audit(r,'--max-files','0')
            self.assertEqual(result.returncode,2)
            self.assertEqual(data['outcome'],'partial')
            self.assertEqual(data['changes']['resolved'],[])

    def test_content_symlink_not_read_and_no_source_mutation(self):
        with tempfile.TemporaryDirectory() as d, tempfile.TemporaryDirectory() as e:
            r=Path(d); p=Path(e)/'secret.py'; p.write_text('def broken(:')
            (r/'linked.py').symlink_to(p)
            _, data=self.run_audit(r)
            self.assertEqual(data['findings'],[])
            self.assertTrue((r/'linked.py').is_symlink())
            self.assertEqual(p.read_text(),'def broken(:')

    def test_jsonc_configuration_is_not_reported_as_broken_json(self):
        with tempfile.TemporaryDirectory() as d:
            r=Path(d); (r/'tsconfig.json').write_text('{ // valid JSONC\n "compilerOptions": {},\n}')
            _, data=self.run_audit(r)
            self.assertEqual(data['findings'],[])
            self.assertEqual(data['coverage']['content_policy_counts']['jsonc_syntax_not_checked'],1)

    def test_source_conflict_markers_and_resolved_history(self):
        with tempfile.TemporaryDirectory() as d:
            r=Path(d); p=r/'main.rs'
            p.write_text('<<<<<<< HEAD\nfn main() {}\n=======\n>>>>>>> branch\n')
            _, first=self.run_audit(r)
            self.assertEqual(first['findings'][0]['kind'],'merge_conflict')
            p.write_text('fn main() {}\n')
            _, fixed=self.run_audit(r)
            self.assertEqual(len(fixed['resolved_findings']),1)
            p.write_text('<<<<<<< HEAD\n')
            _, again=self.run_audit(r)
            self.assertEqual(again['findings'][0]['first_seen'],first['findings'][0]['first_seen'])
            self.assertEqual(len(again['changes']['reopened']),1)

    def test_output_symlink_refused(self):
        with tempfile.TemporaryDirectory() as d, tempfile.TemporaryDirectory() as e:
            r=Path(d); (r/'data/rumil').mkdir(parents=True)
            (r/'data/rumil/hygiene').symlink_to(e,target_is_directory=True)
            result=subprocess.run([sys.executable,str(SCRIPT),'--root',str(r)],capture_output=True)
            self.assertEqual(result.returncode,2)
            self.assertEqual(list(Path(e).iterdir()),[])

if __name__ == '__main__': unittest.main()
