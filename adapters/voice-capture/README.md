# ARDA Voice Capture Adapter

An optional, supervised local speech-to-text input adapter for Personal Operations. It is not a clinical, diagnostic, monitoring, or surveillance system.

## Safety boundary

- Executes only the configured local program with an argument vector; it never invokes a shell or makes a network request.
- Does not download packages or models.
- A successful transcript is `transcript_pending_review`, editable, and not authorized for external send or governed action.
- Audio is ephemeral by default. The supervisor may delete it only after operator review; this adapter never deletes source audio itself.
- Transcript retention is ephemeral unless the request explicitly asks for `"transcript_retention": "retain"` under local policy.
- Backend absence, timeout, non-zero exit, malformed output, or bounded-output failure returns `recoverable_inbox` and preserves the audio reference for retry.
- Errors expose only a typed error class. Transcript, audio content, and backend stderr are not logged or returned on failure.

## Configuration

Copy `config/adapters/voice-capture.toml.example` to a local ignored configuration and set:

- `executable`: local STT executable name or absolute path
- `model`: an already-installed local model
- `audio_root`: supervised inbox root; paths outside it are rejected
- `arguments`: argument vector supporting `{model}` and `{audio_path}`
- timeout, input-size, output-size, extension, and retention limits

The example is inert until its paths and executable are configured. It contains no secret.

## Request and response

The CLI reads one JSON request from `--request FILE` or stdin:

```json
{"schema_version":"arda.voice-capture.request.v1","audio_path":"/supervised/inbox/capture.wav","transcript_retention":"ephemeral"}
```

It emits one `arda.voice-capture.result.v1` JSON response. Validly handled failures also exit zero so a supervisor can persist the typed `recoverable_inbox` result instead of treating it as lost process output. Argument parsing failures remain non-zero.

Example:

```sh
python adapters/voice-capture/arda_adapter.py \
  --config /path/to/local-voice-capture.toml \
  --request /path/to/request.json
```

## Verification

```sh
python -m unittest discover -s adapters/voice-capture/tests -v
python -m py_compile adapters/voice-capture/arda_adapter.py
```

The tests use an injected fake process runner. They validate supervision and policy contracts, not speech recognition quality. A real local model and microphone pipeline must be validated separately by the operator.
