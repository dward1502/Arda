"""One bounded routed subscription probe; no tools or fallback."""
import json
from pathlib import Path
import time
import urllib.request
import urllib.error

payload = {'model': 'openai_sub/gpt-6-astra', 'allow_forced_provider_fallback': False,
           'workload_role': 'interactive', 'context_window_target': 4096,
           'max_tokens': 64, 'stream': False,
           'messages': [{'role': 'user', 'content': 'Reply with exactly ROUTE_READY.'}]}
started = time.monotonic()
result: dict = {'request': payload}
request = urllib.request.Request('http://127.0.0.1:7171/v1/chat/completions',
    data=json.dumps(payload).encode(), headers={'Content-Type':'application/json', 'x-manwe-task-type':'chat'})
try:
    with urllib.request.urlopen(request, timeout=90) as response:
        result.update(status=response.status, body=json.load(response), headers={k.lower():v for k,v in response.headers.items() if k.lower().startswith('x-manwe')})
except urllib.error.HTTPError as error:
    result.update(status=error.code, error=error.read(8192).decode(errors='replace'))
except Exception as error:
    result['error'] = str(error)
result['elapsed_s'] = time.monotonic() - started
Path(__file__).with_suffix('.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2))
