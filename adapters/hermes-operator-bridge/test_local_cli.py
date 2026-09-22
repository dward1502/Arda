"""Local intake binds real persisted user turns, not fabricated transport events."""
import importlib.util
import json
import os
import sqlite3
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


class LocalCliTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)
        self.db = sqlite3.connect(self.home / 'state.db')
        self.addCleanup(self.db.close)
        self.db.executescript('CREATE TABLE sessions(id TEXT, source TEXT, parent_session_id TEXT); CREATE TABLE messages(id INTEGER, session_id TEXT, role TEXT, content TEXT, timestamp REAL);')
        self.db.execute('INSERT INTO sessions VALUES (?,?,?)', ('real-session', 'cli', None))
        self.db.execute('INSERT INTO messages VALUES (?,?,?,?,?)', (17, 'real-session', 'user', 'Continue the existing inspection', 1789580000))
        self.db.commit()
        spec = importlib.util.spec_from_file_location('local_cli', Path(__file__).with_name('local_cli.py'))
        self.db.executescript("ALTER TABLE sessions ADD COLUMN model_config TEXT; ALTER TABLE sessions ADD COLUMN end_reason TEXT; ALTER TABLE messages ADD COLUMN _compressed_summary INTEGER DEFAULT 0;")
        assert spec is not None and spec.origin is not None and spec.loader is not None
        self.assertTrue(Path(spec.origin).exists(), 'local CLI entry point is missing')
        self.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.module)

    def test_payload_binds_user_turn_and_retry_is_stable(self):
        with patch.dict(os.environ, {'HERMES_HOME': str(self.home), 'HERMES_SESSION_ID': 'real-session'}):
            first = self.module.prepare('intake', 'arda status', 17)
            second = self.module.prepare('intake', 'arda status', 17)
            self.assertEqual(first['event'], second['event'])
            self.assertEqual(first['operator']['authentication_method'], 'local_session')
            self.assertEqual(first['event']['source']['platform'], 'cli')
            self.assertIn('17', first['event']['message_id'])
            self.assertEqual(first['provenance']['user_content_sha256'], second['provenance']['user_content_sha256'])

    def test_assistant_or_foreign_message_cannot_supply_user_provenance(self):
        self.db.execute('UPDATE messages SET role=?', ('assistant',))
        self.db.commit()
        with patch.dict(os.environ, {'HERMES_HOME': str(self.home), 'HERMES_SESSION_ID': 'real-session'}):
            with self.assertRaises(ValueError):
                self.module.prepare('intake', 'arda status', 17)

    def test_gateway_or_delegated_session_cannot_claim_local_identity(self):
        self.db.execute('UPDATE sessions SET source=?', ('discord',))
        self.db.commit()
        with patch.dict(os.environ, {'HERMES_HOME': str(self.home), 'HERMES_SESSION_ID': 'real-session'}):
            with self.assertRaises(ValueError):
                self.module.prepare('intake', 'arda status', 17)

    def test_same_operation_cannot_change_command(self):
        with patch.dict(os.environ, {'HERMES_HOME': str(self.home), 'HERMES_SESSION_ID': 'real-session'}):
            self.module.prepare('intake', 'arda status', 17)
            with self.assertRaises(ValueError):
                self.module.prepare('intake', 'arda objectives', 17)


    def test_compression_keeps_conversation_and_real_ancestor_turn(self):
        with patch.dict(os.environ, {'HERMES_HOME': str(self.home), 'HERMES_SESSION_ID': 'real-session'}):
            first = self.module.prepare('intake', 'arda status', 17)
        self.db.execute("UPDATE sessions SET end_reason='compression'")
        self.db.execute("INSERT INTO sessions(id,source,parent_session_id) VALUES ('child','cli','real-session')")
        self.db.execute("INSERT INTO messages(id,session_id,role,content,timestamp) VALUES (18,'child','user','Check status now',1789580100)")
        self.db.commit()
        with patch.dict(os.environ, {'HERMES_HOME': str(self.home), 'HERMES_SESSION_ID': 'child'}):
            self.assertEqual(first['event'], self.module.prepare('intake', 'arda status', 17)['event'])
            fresh = self.module.prepare('status', 'arda status', 18)
            self.assertEqual(fresh['event']['source']['chat_id'], 'real-session')
            self.assertEqual(fresh['provenance']['session_id'], 'child')

    def test_delegated_cli_session_is_rejected(self):
        self.db.execute('UPDATE sessions SET model_config=?', ('{"_delegate_from":"parent"}',))
        self.db.commit()
        with patch.dict(os.environ, {'HERMES_HOME': str(self.home), 'HERMES_SESSION_ID': 'real-session'}):
            with self.assertRaises(ValueError):
                self.module.prepare('intake', 'arda status', 17)

    def test_foreign_session_record_is_rejected(self):
        self.db.execute("UPDATE messages SET session_id='unrelated'")
        self.db.commit()
        with patch.dict(os.environ, {'HERMES_HOME': str(self.home), 'HERMES_SESSION_ID': 'real-session'}):
            with self.assertRaises(ValueError):
                self.module.prepare('intake', 'arda status', 17)

    def test_synthetic_summary_is_not_user_authorization(self):
        self.db.execute('UPDATE messages SET _compressed_summary=1')
        self.db.commit()
        with patch.dict(os.environ, {'HERMES_HOME': str(self.home), 'HERMES_SESSION_ID': 'real-session'}):
            with self.assertRaises(ValueError):
                self.module.prepare('intake', 'arda status', 17)


if __name__ == '__main__':
    unittest.main()
