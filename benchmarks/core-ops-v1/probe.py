import json
import hashlib
import os
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

BIN = Path(__file__).resolve().parents[2] / "target" / "debug" / "aporic"

if not BIN.is_file():
    raise SystemExit(f"Build the local binary first: cargo build --quiet ({BIN})")

def send(proc, item):
    proc.stdin.write((json.dumps(item, separators=(",", ":")) + "\n").encode())
    proc.stdin.flush()

def recv(proc, request_id):
    while True:
        line = proc.stdout.readline()
        if not line:
            raise RuntimeError(f"server exited: {proc.poll()}")
        response = json.loads(line)
        if response.get("id") == request_id:
            return line, response

def call(proc, request_id, name, arguments):
    t0 = time.perf_counter()
    send(proc, {"jsonrpc":"2.0","id":request_id,"method":"tools/call","params":{"name":name,"arguments":arguments}})
    line, response = recv(proc, request_id)
    ms = (time.perf_counter() - t0) * 1000
    content = response["result"]["content"][0]["text"]
    body = json.loads(content)
    if not body.get("ok"):
        raise RuntimeError((name, body))
    return {"method": name, "ms": ms, "frame_bytes": len(line), "body_bytes": len(content.encode()), "body": body}

measurements = []
for iteration in range(3):
    with tempfile.TemporaryDirectory(prefix="aporic-cost-") as area:
        workspace = os.path.join(area, "workspace")
        os.mkdir(workspace)
        environment = os.environ.copy()
        environment["APORIC_DATABASE"] = os.path.join(area, "aporic.sqlite3")
        proc = subprocess.Popen([BIN, "mcp", "serve", "--stdio"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=environment)
        try:
            send(proc, {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"cost-probe","version":"1"}}})
            _, initialized = recv(proc, 1)
            if "error" in initialized:
                raise RuntimeError(initialized)
            send(proc, {"jsonrpc":"2.0","method":"notifications/initialized"})
            sequence = []
            opened = call(proc, 2, "aporic_open", {"workspace":workspace,"objective":"Measure core continuity protocol","idempotency_key":f"open-{iteration}"})
            sequence.append(opened)
            session_id = opened["body"]["result"]["session_id"]
            sequence.append(call(proc, 3, "aporic_record", {"session_id":session_id,"kind":"observation","content":"A bounded note for a later session.","evidence":None,"idempotency_key":f"record-{iteration}"}))
            sequence.append(call(proc, 4, "aporic_close", {"session_id":session_id,"disposition":"handoff","summary":"Initial inspection complete.","next_action":"Review the remaining case.","idempotency_key":f"close-{iteration}"}))
            sequence.append(call(proc, 5, "aporic_resume", {"workspace":workspace}))
            sequence.append(call(proc, 6, "aporic_recall", {"workspace":workspace,"limit":5,"objective":"Review the remaining case.","compact":False}))
            measurements.append(sequence)
        finally:
            proc.terminate()
            proc.communicate(timeout=3)

print("binary_sha256", hashlib.sha256(BIN.read_bytes()).hexdigest())
for index, row in enumerate(measurements, 1):
    print("run", index)
    for item in row:
        print(item["method"], round(item["ms"],2), item["frame_bytes"], item["body_bytes"])
    print("total", round(sum(item["ms"] for item in row),2), sum(item["frame_bytes"] for item in row), sum(item["body_bytes"] for item in row))
print("median by method")
for i in range(5):
    items = [run[i] for run in measurements]
    print(items[0]["method"], round(statistics.median(item["ms"] for item in items),2), statistics.median(item["frame_bytes"] for item in items), statistics.median(item["body_bytes"] for item in items))
