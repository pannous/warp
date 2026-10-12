#!/usr/bin/env python3
"""Runs one Codex turn on the shared local app-server daemon, so the thread shows up in the Codex app:
    codex_app_run.py <cwd> <thread name> <prompt> <events.jsonl> <last message.md>
`codex exec` runs in-process (threads with source exec, which the app never lists); `codex app-server proxy` pipes
this script's stdio to the daemon's control socket instead. That socket speaks WebSocket (HTTP upgrade, then one
JSON-RPC message per text frame), so the framing is done here. JSON-RPC: initialize, thread/start, thread/name/set,
turn/start, then every message goes to <events.jsonl> until turn/completed; the last agent message goes to
<last message.md>. Exit 0 on a completed turn, 1 on a failed one, 2 when no daemon answers (caller falls back)."""
import base64
import json
import os
import signal
import struct
import subprocess
import sys

CLIENT_INFO = {"name": "codex_cron", "title": "warp cron worker", "version": "1"}
STARTUP_TIMEOUT_SECONDS = 30
TEXT, CLOSE, PING, PONG = 0x1, 0x8, 0x9, 0xA

cwd, thread_name, prompt, events_path, last_message_path = sys.argv[1:6]
proxy = subprocess.Popen(["codex", "app-server", "proxy"], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
events = open(events_path, "a")
next_id = 0


def write_frame(opcode, payload):
    mask = os.urandom(4)
    length = len(payload)
    if length < 126:
        header = struct.pack("!BB", 0x80 | opcode, 0x80 | length)
    elif length < 1 << 16:
        header = struct.pack("!BBH", 0x80 | opcode, 0x80 | 126, length)
    else:
        header = struct.pack("!BBQ", 0x80 | opcode, 0x80 | 127, length)
    masked = bytes(byte ^ mask[i % 4] for i, byte in enumerate(payload))
    proxy.stdin.write(header + mask + masked)
    proxy.stdin.flush()


def read_exactly(count):
    data = proxy.stdout.read(count)
    if len(data) < count:
        raise EOFError
    return data


def read_frame():
    first, second = read_exactly(2)
    length = second & 0x7F
    if length == 126:
        length, = struct.unpack("!H", read_exactly(2))
    elif length == 127:
        length, = struct.unpack("!Q", read_exactly(8))
    mask = read_exactly(4) if second & 0x80 else None
    payload = read_exactly(length)
    if mask:
        payload = bytes(byte ^ mask[i % 4] for i, byte in enumerate(payload))
    return first & 0x80, first & 0x0F, payload


def read_message():
    """the next text message, answering pings and joining fragments; None once the daemon closes"""
    parts = []
    while True:
        final, opcode, payload = read_frame()
        if opcode == PING:
            write_frame(PONG, payload)
        elif opcode == CLOSE:
            return None
        elif opcode != PONG:
            parts.append(payload)
            if final:
                return b"".join(parts).decode()


def open_websocket():
    key = base64.b64encode(os.urandom(16)).decode()
    proxy.stdin.write(f"GET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
                      f"Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n".encode())
    proxy.stdin.flush()
    response = b""
    while not response.endswith(b"\r\n\r\n"):
        response += read_exactly(1)
    if b" 101 " not in response.split(b"\r\n")[0]:
        sys.exit(f"app-server refused the WebSocket upgrade: {response.decode(errors='replace')}")


def send(message):
    write_frame(TEXT, json.dumps(message).encode())


def messages():
    try:
        while (text := read_message()) is not None:
            events.write(text + "\n")
            events.flush()
            yield json.loads(text)
    except EOFError:
        pass


def answer_server_request(message):
    # approval "never" means none should come; refuse anything that does rather than hang the unattended run
    send({"id": message["id"], "error": {"code": -32601, "message": "unattended cron run: no approvals"}})


def request(method, params):
    global next_id
    next_id += 1
    send({"id": next_id, "method": method, "params": params})
    for message in messages():
        if message.get("id") == next_id and "method" not in message:
            if "error" in message:
                sys.exit(f"{method} failed: {message['error']}")
            return message["result"]
        if "method" in message and "id" in message:
            answer_server_request(message)
    sys.exit(2)


def run_turn(thread_id):
    request("turn/start", {"threadId": thread_id, "input": [{"type": "text", "text": prompt}]})
    last_message = ""
    for message in messages():
        method = message.get("method")
        if method and "id" in message:
            answer_server_request(message)
        elif method == "item/completed" and message["params"]["item"].get("type") == "agentMessage":
            last_message = message["params"]["item"].get("text", "")
        elif method == "turn/completed":
            open(last_message_path, "w").write(last_message)
            turn = message["params"]["turn"]
            print(f"turn {turn.get('status')}: thread {thread_id}")
            return 0 if turn.get("status") == "completed" else 1
    sys.exit("app-server connection closed before turn/completed")


def startup_timed_out(*_):
    raise TimeoutError


try:
    signal.signal(signal.SIGALRM, startup_timed_out)
    signal.alarm(STARTUP_TIMEOUT_SECONDS)
    open_websocket()
    request("initialize", {"clientInfo": CLIENT_INFO, "capabilities": {"experimentalApi": True}})
    send({"method": "initialized"})
    thread = request("thread/start", {"cwd": cwd, "approvalPolicy": "never", "sandbox": "danger-full-access"})["thread"]
    request("thread/name/set", {"threadId": thread["id"], "name": thread_name})
    signal.alarm(0)
    sys.exit(run_turn(thread["id"]))
except (BrokenPipeError, EOFError, TimeoutError):
    sys.exit(2)
