import json
import hashlib
import os
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

BIN = Path(os.environ.get("APORIC_PROBE_BINARY", Path(__file__).resolve().parents[2] / "target" / "debug" / "aporic"))

if not BIN.is_file():
    raise SystemExit(f"Build the local binary first: cargo build --quiet ({BIN})")

def send(proc, item):
    frame = (json.dumps(item, separators=(",", ":")) + "\n").encode()
    proc.stdin.write(frame)
    proc.stdin.flush()
    return len(frame)

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
    request_bytes = send(proc, {"jsonrpc":"2.0","id":request_id,"method":"tools/call","params":{"name":name,"arguments":arguments}})
    line, response = recv(proc, request_id)
    ms = (time.perf_counter() - t0) * 1000
    content = response["result"]["content"][0]["text"]
    body = json.loads(content)
    if not body.get("ok"):
        raise RuntimeError((name, body))
    return {"method": name, "ms": ms, "request_bytes": request_bytes, "frame_bytes": len(line), "body_bytes": len(content.encode()), "body": body}

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
            shape = {"parallel_paths":1,"material_change":False,
                     "worker":{"disposition":"skip","reason":"Sequential protocol fixture"},
                     "reviewer":{"disposition":"skip","reason":"Mechanical protocol fixture, no material product change"}}
            opened = call(proc, 2, "aporic_begin", {"workspace":workspace,"objective":"Measure core continuity protocol","work_shape":shape,"idempotency_key":f"begin-{iteration}"})
            sequence.append(opened)
            session_id = opened["body"]["result"]["session_id"]
            sequence.append(call(proc, 3, "aporic_finish", {"session_id":session_id,"disposition":"handoff","summary":"Initial inspection complete.","next_action":"Review the remaining case.","notes":[{"kind":"observation","content":"A bounded note for a later session.","evidence":None}],"idempotency_key":f"finish-{iteration}"}))
            resumed = call(proc, 4, "aporic_resume", {"workspace":workspace})
            sequence.append(resumed)
            assert resumed["body"]["result"]["selected"]["source_id"] == session_id
            continued = call(proc, 5, "aporic_begin", {"workspace":workspace,"objective":"Review the remaining case.","work_shape":shape,"idempotency_key":f"continue-{iteration}"})
            sequence.append(continued)
            selected = continued["body"]["result"]["context"]["selected_items"]
            assert any(item["content"] == "A bounded note for a later session." for item in selected)
            sequence.append(call(proc, 6, "aporic_finish", {"session_id":continued["body"]["result"]["session_id"],"disposition":"completed","summary":"Continuity note recovered.","next_action":None,"notes":[],"idempotency_key":f"complete-{iteration}"}))
            measurements.append(sequence)
        finally:
            proc.terminate()
            proc.communicate(timeout=3)

print("binary_sha256", hashlib.sha256(BIN.read_bytes()).hexdigest())
print("scope", "Local stdio only; five lifecycle calls across two durable sessions; excludes host/model/authoring cost")
for index, row in enumerate(measurements, 1):
    print("run", index)
    for item in row:
        print(item["method"], round(item["ms"],2), item["frame_bytes"], item["body_bytes"], item["request_bytes"])
    print("total", round(sum(item["ms"] for item in row),2), sum(item["frame_bytes"] for item in row), sum(item["body_bytes"] for item in row), sum(item["request_bytes"] for item in row))
print("median by method")
for i in range(5):
    items = [run[i] for run in measurements]
    print(items[0]["method"], round(statistics.median(item["ms"] for item in items),2), statistics.median(item["frame_bytes"] for item in items), statistics.median(item["body_bytes"] for item in items))

if os.environ.get("APORIC_PROBE_JSON"):
    Path(os.environ["APORIC_PROBE_JSON"]).write_text(json.dumps(measurements, indent=2) + "\n")
