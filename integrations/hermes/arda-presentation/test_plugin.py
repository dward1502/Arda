import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('presentation_plugin', Path(__file__).with_name('__init__.py'))
plugin = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(plugin)

class PresentationTests(unittest.TestCase):
    def test_private_input_is_not_copied_or_submitted(self):
        result = json.loads(plugin.handle({'action':'present','source':'/etc/passwd','request_id':'one','ambient_allowed':False}))
        self.assertFalse(result['ok'])
        self.assertIn('permission', result['error'])

    def test_stage_rejects_non_attachment_and_symlink_escape(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); cache = root/'cache'; cache.mkdir()
            outside = root/'secret'; outside.write_text('not shared')
            (cache/'escape').symlink_to(outside)
            for source in [outside, cache/'escape']:
                with self.assertRaises(ValueError):
                    plugin.stage_asset(source, [cache], root/'arda')

    def test_actual_mime_and_deduplicated_asset(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); cache = root/'cache'; cache.mkdir()
            source = cache/'wrong.jpeg'; source.write_text('plain text, not a photograph\n')
            first = plugin.stage_asset(source, [cache], root/'arda')
            second = plugin.stage_asset(source, [cache], root/'arda')
            self.assertEqual(first, second)
            self.assertEqual(first['mime'], 'text/plain')
            self.assertEqual(len(list((root/'arda/data/media/imports').iterdir())), 1)

    def test_missing_hud_is_not_a_success(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(OSError):
                plugin.exchange({'action':'status'}, Path(tmp)/'missing.sock')

if __name__ == '__main__': unittest.main()
