"""Local Hermes tool/CLI intake. OS-private capability authenticates; history is provenance.

Commands are interpreted agent actions bound to a real user turn, not verbatim
human transport messages. Never reuse gateway credentials or invent platform IDs.
"""
from __future__ import annotations

import fcntl
import hashlib
import json
import os
from pathlib import Path
import sqlite3
from datetime import datetime, timezone
from urllib.request import Request, urlopen
from urllib.error import HTTPError


def _home():
    return Path(os.environ.get('HERMES_HOME', str(Path.home() / '.hermes'))).expanduser().resolve()


def _session():
    # This entry point is CLI-only; do not expose it as an in-process gateway tool.
    return os.environ.get('HERMES_SESSION_ID', '')


def _private_json(path, value):
    temporary = path.with_suffix('.tmp')
    with os.fdopen(os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), 'w') as stream:
        json.dump(value, stream, sort_keys=True)
        stream.flush()
        os.fsync(stream.fileno())
    temporary.replace(path)
    directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def prepare(operation, command, message_id):
    session = _session()
    if not session or not operation.strip() or not command.strip().lower().startswith('arda '):
        raise ValueError('Active Hermes CLI session, operation key and Arda command required')
    home = _home()
    db = sqlite3.connect(f'{(home / "state.db").as_uri()}?mode=ro', uri=True)
    try:
        lineage = []
        cursor = session
        while cursor:
            if cursor in lineage or len(lineage) >= 8192:
                raise ValueError('Invalid CLI compression lineage')
            source = db.execute('SELECT source, parent_session_id, model_config, end_reason FROM sessions WHERE id=?', (cursor,)).fetchone()
            if not source or source[0] != 'cli':
                raise ValueError('CLI session required')
            config = json.loads(source[2] or '{}')
            if config.get('_delegate_from') is not None or config.get('_branched_from') is not None:
                raise ValueError('Delegated or branched provenance is not supported')
            if lineage and source[3] != 'compression':
                raise ValueError('Parent is not a compression continuation')
            lineage.append(cursor)
            cursor = source[1]
        row = db.execute('SELECT content, timestamp, session_id, _compressed_summary FROM messages WHERE id=? AND role=?',
                         (message_id, 'user')).fetchone()
    finally:
        db.close()
    if (not row or row[2] not in lineage or not row[0] or row[3]
            or row[0].lstrip().startswith(('[CONTEXT COMPACTION', '[ASYNC DELEGATION', '[System:',
                                         '[Your active task list', '[IMPORTANT: Background process '))):
        raise ValueError('A real user message in the active non-delegated CLI session is required')
    session = lineage[-1]
    operator = os.environ.get('ARDA_OPERATOR_ID', 'operator:mythos').strip()
    if not operator:
        raise ValueError('Operator identity is empty')
    content_hash = hashlib.sha256(row[0].encode()).hexdigest()
    key = hashlib.sha256(json.dumps([operator, session, message_id, operation]).encode()).hexdigest()
    event_id = f'cli:{session}:{message_id}:{key}'
    record = {
        'operator': {'operator_id': operator, 'authenticated': True,
                     'authentication_method': 'local_session', 'authenticated_at': ''},
        'adapter_id': 'hermes-cli',
        'event': {'text': command, 'message_type': 'text', 'user_id': operator, 'user_name': None,
                  'source': {'platform': 'cli', 'chat_id': session, 'chat_type': 'private',
                             'thread_id': None, 'message_id': event_id},
                  'message_id': event_id, 'media_urls': [], 'media_types': [],
                  'timestamp': datetime.fromtimestamp(row[1], timezone.utc).isoformat(),
                  'prompt_response': None},
        'provenance': {'session_id': row[2], 'user_message_id': message_id,
                       'user_content_sha256': content_hash, 'operation': operation,
                       'interpretation': 'agent_command_from_user_instruction'},
    }
    directory = home / 'state' / 'arda-operator-bridge' / 'local'
    directory.mkdir(parents=True, exist_ok=True, mode=0o700)
    path = directory / f'{key}.json'
    with os.fdopen(os.open(directory / '.lock', os.O_RDWR | os.O_CREAT, 0o600), 'w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if path.exists():
            saved = json.loads(path.read_text())
            if any(saved[k] != record[k] for k in ('event', 'provenance', 'adapter_id')):
                raise ValueError('Operation key conflicts with retained command/provenance')
            record = saved
        else:
            _private_json(path, record)
    return record


def submit(operation, command, message_id):
    record = prepare(operation, command, message_id)
    capability_path = _home() / 'state' / 'arda-operator-bridge' / 'local-capability'
    stat = capability_path.stat()
    if stat.st_uid != os.getuid() or stat.st_mode & 0o077:
        raise PermissionError('Local capability must be owned by this user and private')
    capability = capability_path.read_text().strip()
    if not capability:
        raise PermissionError('Local capability is empty')
    payload = {key: record[key] for key in ('operator', 'adapter_id', 'event')}
    payload['operator'] = dict(payload['operator'], authenticated_at=datetime.now(timezone.utc).isoformat())
    request = Request('http://127.0.0.1:7878/v1/operator/local-messages',
                      data=json.dumps(payload).encode(), method='POST',
                      headers={'Content-Type': 'application/json', 'x-arda-local-capability': capability})
    try:
        with urlopen(request, timeout=65) as response:
            result = {'http_status': response.status, 'response': json.load(response)}
    except HTTPError as error:
        result = {'http_status': error.code, 'response': json.load(error)}
    result['provenance'] = record['provenance']
    return result


def setup(parser):
    parser.add_argument('--operation', required=True, help='Stable action key; reuse for retries')
    parser.add_argument('--message-id', required=True, type=int, help='Existing user record in active session')
    parser.add_argument('--command', required=True, help='Interpreted Arda action; never grants project permissions')


def main(args):
    result = submit(args.operation, args.command, args.message_id)
    print(json.dumps(result, indent=2))
    if result['http_status'] >= 400:
        raise SystemExit(1)


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    setup(parser)
    main(parser.parse_args())
