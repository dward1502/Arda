"""Hermes execution adapter for the native Arda upper-monitor registry.

No desktop clicks, remote listener, knowledge ingestion, or fabricated render
receipts. Messaging authentication remains owned by Hermes/Oromë ingress.
"""
import hashlib
import json
import os
from pathlib import Path
import socket
import stat
import subprocess
import tempfile

MAX_ASSET = 128 * 1024 * 1024


def exchange(payload, path=None):
    if path is None:
        path = Path(os.environ['XDG_RUNTIME_DIR'])/'arda-hud/presentation.sock'
    metadata = path.stat()
    if not stat.S_ISSOCK(metadata.st_mode) or metadata.st_uid != os.getuid() or metadata.st_mode & 0o077:
        raise ValueError('Native presentation socket is not private and same-user')
    body = json.dumps(payload).encode() + b'\n'
    if len(body) > 65536:
        raise ValueError('Presentation request exceeds 64 KiB')
    with socket.socket(socket.AF_UNIX) as client:
        client.settimeout(8)
        client.connect(str(path))
        client.sendall(body)
        with client.makefile('rb') as stream:
            response = stream.readline(1024 * 1024)
    if not response.endswith(b'\n'):
        raise ValueError('Native presentation response is incomplete')
    return json.loads(response)


def stage_asset(source, allowed_roots, arda_root):
    source = Path(source).expanduser().resolve(strict=True)
    roots = [Path(root).resolve() for root in allowed_roots]
    if not any(source.is_relative_to(root) for root in roots):
        raise ValueError('Source is outside authenticated attachment/generated-artifact roots')
    # Use a bounded read and classify the exact staged bytes, not the sender's MIME.
    with source.open('rb') as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError('Source must be a regular file')
        data = stream.read(MAX_ASSET + 1)
    if len(data) > MAX_ASSET:
        raise ValueError('Media exceeds 128 MiB presentation limit')
    imports = arda_root/'data/media/imports'
    imports.mkdir(mode=0o700, parents=True, exist_ok=True)
    digest = hashlib.sha256(data).hexdigest()
    target = imports/digest
    # Atomic, deduplicated immutable asset. Never overwrite an unexpected file.
    if target.exists():
        if target.is_symlink() or hashlib.sha256(target.read_bytes()).hexdigest() != digest:
            raise ValueError('Existing media asset has an integrity conflict')
    else:
        fd, tmp = tempfile.mkstemp(dir=imports, prefix='.staging-')
        try:
            with os.fdopen(fd, 'wb') as stream:
                stream.write(data)
                stream.flush()
                os.fsync(stream.fileno())
            try:
                os.link(tmp, target)
            except FileExistsError:
                if target.is_symlink() or hashlib.sha256(target.read_bytes()).hexdigest() != digest:
                    raise ValueError('Concurrent media asset has an integrity conflict')
        finally:
            Path(tmp).unlink(missing_ok=True)
    result = subprocess.run(['file', '--brief', '--mime-type', '--', str(target)], capture_output=True, text=True, check=True, timeout=5)
    return {'kind':'asset', 'path':str(target.relative_to(arda_root)), 'mime':result.stdout.strip()}


def handle(args, **kwargs):
    try:
        action = args.get('action', 'status')
        if action == 'status':
            return json.dumps(exchange({'action':'status'}))
        if action != 'present':
            raise ValueError('Unsupported presentation action')
        if args.get('ambient_allowed') is not True:
            raise ValueError('Explicit operator permission for ambient display is required')
        request_id = args.get('request_id', '')
        if not request_id or len(request_id) > 128 or any(not (c.isascii() and (c.isalnum() or c in '-_:')) for c in request_id):
            raise ValueError('A stable 1-128 character request_id is required')
        source = args.get('source', '')
        if not isinstance(source, str) or not source:
            raise ValueError('Source URL or attachment path is required')
        # Read native readiness before copying media; no successful-looking local
        # import is substituted for an unavailable HUD.
        status = exchange({'action':'status'})
        if not status.get('ok'):
            return json.dumps(status)
        if source.startswith(('https://', 'http://')):
            descriptor = {'kind':'web', 'url':source}
        else:
            home = Path(os.environ.get('HERMES_HOME', '~/.hermes')).expanduser()
            root = Path(os.environ.get('ARDA_ROOT', Path(__file__).resolve().parents[3]))
            allowed = [home/p for p in ['cache/images','cache/audio','cache/video','cache/documents','image_cache','audio_cache','video_cache','document_cache']]
            allowed.append(root/'data/artifacts')
            descriptor = stage_asset(source, allowed, root)
        request = {'requestId':request_id, 'ambientAllowed':True, 'operation':args.get('operation','show'), 'source':descriptor, 'slotId':args.get('slot_id')}
        response = exchange({'action':'present','request':request})
        if response.get('ok'):
            session = response['result']['session']
            readback = exchange({'action':'status'})
            current = readback.get('registry',{}).get('sessions',{}).get(session['slotId'])
            if current != session:
                return json.dumps({'ok':False,'error':'Publication readback mismatch; rendering is unverified'})
            response['rendering_verified'] = False
            response['message'] = 'Native registry publication verified. Rendering/playback remains unverified; do not report shown or playing.'
        return json.dumps(response)
    except Exception as exc:
        return json.dumps({'ok':False,'error':str(exc),'rendering_verified':False})


def register(ctx):
    ctx.register_tool(name='arda_present', toolset='arda_presentation', schema={
        'name':'arda_present',
        'description':'Present operator-requested media on Arda HUD upper monitors, or inspect native availability. Use for requests to show a shared attachment/link on an upper monitor. Never use desktop clicks or write registry files. Requires explicit ambient-display permission. Publication is NOT rendering/playback evidence. Unsupported media is reported honestly; no knowledge retention occurs. Reuse request_id for retries; use a new ID for a new request.',
        'parameters':{'type':'object','properties':{
            'action':{'type':'string','enum':['status','present']},
            'source':{'type':'string','description':'Exact public HTTP(S) URL or cached attachment/generated artifact path'},
            'request_id':{'type':'string','description':'Stable request identity for this intent; ASCII letters/digits, -_: only'},
            'ambient_allowed':{'type':'boolean','description':'True only when operator explicitly requested ambient/upper-monitor display'},
            'operation':{'type':'string','enum':['show','play']},
            'slot_id':{'type':'string','enum':[f'monitor_{n}' for n in range(1,6)]}
        },'required':['action'],'additionalProperties':False}
    }, handler=handle)
