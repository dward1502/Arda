"""Fixed logical HTTP endpoint over the supervisor's inherited duplex stdin."""
import threading
_inference_transaction_lock = threading.Lock()

def broker_send(httpx, request):
    with _inference_transaction_lock:
        return _broker_exchange(httpx, request)

def _broker_exchange(httpx, request):
    import json
    import os
    import struct
    if request.method != "POST" or str(request.url) != "http://127.0.0.1:7171/v1/chat/completions":
        raise RuntimeError("inference broker rejects destination/method")
    payload = request.content
    if not 0 < len(payload) <= 2097152:
        raise RuntimeError("inference request exceeds broker bound")
    def write_all(data):
        view = memoryview(data)
        while view:
            written = os.write(0, view)
            if not written:
                raise RuntimeError("inference broker closed")
            view = view[written:]
    def read_exact(size):
        chunks = bytearray()
        while len(chunks) < size:
            chunk = os.read(0, size - len(chunks))
            if not chunk:
                raise RuntimeError("inference broker closed")
            chunks.extend(chunk)
        return chunks
    write_all(struct.pack("!I", len(payload)) + payload)
    size = struct.unpack("!I", read_exact(4))[0]
    if not 0 < size <= 16777216:
        raise RuntimeError("inference response exceeds broker bound")
    result = json.loads(read_exact(size))
    if "error" in result:
        raise RuntimeError("inference broker rejected request")
    return httpx.Response(result["status"], headers=result["headers"],
                          content=result["body"].encode("utf-8"), request=request)
