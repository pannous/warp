#!/usr/bin/env python3
"""Run one visible Codex turn through the desktop app-server daemon.

Usage: codex_app_run.py <cwd> <thread name> <prompt> <events.jsonl> <last.md> [result.json]
"""

import base64
import json
import os
from pathlib import Path
import select
import signal
import struct
import subprocess
import sys
import time


CLIENT_INFO = {"name": "codex_cron", "title": "warp cron worker", "version": "2"}
STARTUP_TIMEOUT_SECONDS = 30
REQUEST_TIMEOUT_SECONDS = 30
STATUS_POLL_SECONDS = 15
DEFAULT_TURN_TIMEOUT_SECONDS = 4 * 60 * 60 + 30 * 60
INTERRUPT_GRACE_SECONDS = 30
PROXY_EXIT_GRACE_SECONDS = 5
EXIT_COMPLETED = 0
EXIT_FAILED = 1
EXIT_UNAVAILABLE = 2
EXIT_INTERRUPTED = 3
EXIT_TIMED_OUT = 4
EXIT_DISCONNECTED = 5
TEXT, CLOSE, PING, PONG = 0x1, 0x8, 0x9, 0xA


class ReadTimeout(Exception):
    pass


class ConnectionClosed(Exception):
    pass


class RunInterrupted(Exception):
    def __init__(self, signal_name):
        super().__init__(signal_name)
        self.signal_name = signal_name


cwd, thread_name, prompt, events_path, last_message_path = sys.argv[1:6]
result_path = sys.argv[6] if len(sys.argv) > 6 else f"{last_message_path}.result.json"
turn_timeout_seconds = int(os.environ.get("CODEX_TURN_TIMEOUT_SECONDS", DEFAULT_TURN_TIMEOUT_SECONDS))
events = open(events_path, "a")
proxy = subprocess.Popen(
    ["codex", "app-server", "proxy"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    bufsize=0,
)
next_id = 0
websocket_open = False
last_message = ""
partial_message = ""
last_error = ""
terminal_turn = None
run_result = {
    "status": "starting",
    "threadId": None,
    "turnId": None,
    "projectId": None,
    "remoteTurnStatus": None,
    "error": None,
    "exitCode": EXIT_DISCONNECTED,
}


def interrupt_on_signal(signal_number, _frame):
    raise RunInterrupted(signal.Signals(signal_number).name)


def atomic_write(path, contents):
    destination = Path(path)
    partial = destination.with_name(f"{destination.name}.partial")
    partial.write_text(contents)
    os.replace(partial, destination)


def artifact_contents():
    status = run_result["status"]
    lifecycle = [f"Codex lifecycle: **{status}**."]
    if run_result["threadId"]:
        lifecycle.append(f"Thread: `{run_result['threadId']}`.")
    if run_result["turnId"]:
        lifecycle.append(f"Turn: `{run_result['turnId']}`.")
    if run_result["error"]:
        lifecycle.append(f"Detail: {run_result['error']}")
    lifecycle_text = " ".join(lifecycle)
    if last_message and status == "completed":
        return f"{last_message.rstrip()}\n"
    if last_message:
        return f"{last_message.rstrip()}\n\n---\n{lifecycle_text}\n"
    return f"# Codex run {status}\n\n{lifecycle_text}\n"


def write_outputs():
    atomic_write(last_message_path, artifact_contents())
    atomic_write(result_path, json.dumps(run_result, indent=2) + "\n")


def write_frame(opcode, payload):
    mask = os.urandom(4)
    length = len(payload)
    if length < 126:
        header = struct.pack("!BB", 0x80 | opcode, 0x80 | length)
    elif length < 1 << 16:
        header = struct.pack("!BBH", 0x80 | opcode, 0x80 | 126, length)
    else:
        header = struct.pack("!BBQ", 0x80 | opcode, 0x80 | 127, length)
    masked = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
    proxy.stdin.write(header + mask + masked)
    proxy.stdin.flush()


def read_exactly(count, timeout_seconds):
    data = b""
    deadline = time.monotonic() + timeout_seconds
    while len(data) < count:
        remaining = deadline - time.monotonic()
        if remaining <= 0 or not select.select([proxy.stdout], [], [], remaining)[0]:
            raise ReadTimeout
        chunk = os.read(proxy.stdout.fileno(), count - len(data))
        if not chunk:
            raise ConnectionClosed
        data += chunk
    return data


def read_frame(timeout_seconds):
    first, second = read_exactly(2, timeout_seconds)
    length = second & 0x7F
    if length == 126:
        length, = struct.unpack("!H", read_exactly(2, timeout_seconds))
    elif length == 127:
        length, = struct.unpack("!Q", read_exactly(8, timeout_seconds))
    mask = read_exactly(4, timeout_seconds) if second & 0x80 else None
    payload = read_exactly(length, timeout_seconds)
    if mask:
        payload = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
    return first & 0x80, first & 0x0F, payload


def read_message(timeout_seconds):
    parts = []
    while True:
        final, opcode, payload = read_frame(timeout_seconds)
        if opcode == PING:
            write_frame(PONG, payload)
        elif opcode == CLOSE:
            raise ConnectionClosed
        elif opcode != PONG:
            parts.append(payload)
            if final:
                return b"".join(parts).decode()


def observe(message):
    global last_message, partial_message, last_error, terminal_turn
    method = message.get("method")
    params = message.get("params", {})
    item = params.get("item", {})
    if method == "item/agentMessage/delta":
        partial_message += params.get("delta", "")
    elif method == "item/completed" and item.get("type") == "agentMessage":
        last_message = item.get("text", "")
        partial_message = ""
    elif method == "error":
        last_error = params.get("error", {}).get("message", "Codex reported an unspecified error")
    elif method == "turn/completed" and params.get("threadId") == run_result["threadId"]:
        terminal_turn = params["turn"]


def receive(timeout_seconds):
    text = read_message(timeout_seconds)
    events.write(text + "\n")
    events.flush()
    message = json.loads(text)
    observe(message)
    return message


def send(message):
    write_frame(TEXT, json.dumps(message).encode())


def answer_server_request(message):
    send({"id": message["id"], "error": {"code": -32601, "message": "unattended cron run: no approvals"}})


def request(method, params, timeout_seconds=REQUEST_TIMEOUT_SECONDS):
    global next_id
    next_id += 1
    request_id = next_id
    send({"id": request_id, "method": method, "params": params})
    deadline = time.monotonic() + timeout_seconds
    while True:
        message = receive(max(0.1, deadline - time.monotonic()))
        if message.get("id") == request_id and "method" not in message:
            if "error" in message:
                raise RuntimeError(f"{method} failed: {message['error']}")
            return message["result"]
        if "method" in message and "id" in message:
            answer_server_request(message)


def open_websocket():
    global websocket_open
    key = base64.b64encode(os.urandom(16)).decode()
    request_text = (
        "GET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
        f"Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
    )
    proxy.stdin.write(request_text.encode())
    proxy.stdin.flush()
    response = b""
    while not response.endswith(b"\r\n\r\n"):
        response += read_exactly(1, STARTUP_TIMEOUT_SECONDS)
    if b" 101 " not in response.split(b"\r\n")[0]:
        raise RuntimeError(f"app-server refused the WebSocket upgrade: {response.decode(errors='replace')}")
    websocket_open = True


def turn_from_thread(thread):
    for turn in reversed(thread.get("turns", [])):
        if turn.get("id") == run_result["turnId"]:
            return turn
    return None


def wait_for_terminal(timeout_seconds, poll_status=True):
    deadline = time.monotonic() + timeout_seconds
    while True:
        if terminal_turn:
            return terminal_turn
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise ReadTimeout
        try:
            message = receive(min(STATUS_POLL_SECONDS, remaining))
            if "method" in message and "id" in message:
                answer_server_request(message)
        except ReadTimeout:
            if not poll_status:
                raise
            thread = request(
                "thread/read",
                {"threadId": run_result["threadId"], "includeTurns": True},
            )["thread"]
            persisted_turn = turn_from_thread(thread)
            if persisted_turn and persisted_turn.get("status") != "inProgress":
                return persisted_turn


def request_interrupt():
    global next_id
    if not run_result["threadId"] or not run_result["turnId"]:
        return
    next_id += 1
    send(
        {
            "id": next_id,
            "method": "turn/interrupt",
            "params": {"threadId": run_result["threadId"], "turnId": run_result["turnId"]},
        }
    )


def complete_from_turn(turn):
    global last_error
    remote_status = turn.get("status", "failed")
    error = turn.get("error") or {}
    if error.get("message"):
        last_error = error["message"]
    run_result["remoteTurnStatus"] = remote_status
    if remote_status == "completed":
        return "completed", EXIT_COMPLETED
    if remote_status == "interrupted":
        return "interrupted", EXIT_INTERRUPTED
    return "failed", EXIT_FAILED


def run_turn():
    turn = request(
        "turn/start",
        {"threadId": run_result["threadId"], "input": [{"type": "text", "text": prompt}]},
    )["turn"]
    run_result["turnId"] = turn["id"]
    try:
        terminal = wait_for_terminal(turn_timeout_seconds)
    except ReadTimeout:
        request_interrupt()
        try:
            terminal = wait_for_terminal(INTERRUPT_GRACE_SECONDS)
            run_result["remoteTurnStatus"] = terminal.get("status")
        except (ReadTimeout, ConnectionClosed):
            pass
        return "timedOut", f"turn exceeded {turn_timeout_seconds} seconds", EXIT_TIMED_OUT
    status, exit_code = complete_from_turn(terminal)
    return status, last_error, exit_code


def close_proxy():
    if websocket_open and proxy.poll() is None:
        try:
            write_frame(CLOSE, b"")
        except (BrokenPipeError, OSError):
            pass
    if proxy.stdin:
        try:
            proxy.stdin.close()
        except OSError:
            pass
    try:
        proxy.wait(timeout=PROXY_EXIT_GRACE_SECONDS)
    except subprocess.TimeoutExpired:
        proxy.terminate()
        try:
            proxy.wait(timeout=PROXY_EXIT_GRACE_SECONDS)
        except subprocess.TimeoutExpired:
            proxy.kill()
            proxy.wait()


def main():
    global last_message
    signal.signal(signal.SIGINT, interrupt_on_signal)
    signal.signal(signal.SIGTERM, interrupt_on_signal)
    signal.signal(signal.SIGHUP, interrupt_on_signal)
    status, detail, exit_code = "failed", "runner stopped before initialization", EXIT_FAILED
    try:
        open_websocket()
        request("initialize", {"clientInfo": CLIENT_INFO, "capabilities": {"experimentalApi": True}})
        send({"method": "initialized"})
        thread = request(
            "thread/start",
            {"cwd": cwd, "approvalPolicy": "never", "sandbox": "danger-full-access"},
        )["thread"]
        run_result["threadId"] = thread["id"]
        run_result["projectId"] = thread.get("projectId")
        request("thread/name/set", {"threadId": thread["id"], "name": thread_name})
        run_result["status"] = "active"
        print(
            f"started {thread['id']} status=active project={thread.get('projectId') or 'none'}",
            flush=True,
        )
        status, detail, exit_code = run_turn()
    except RunInterrupted as interruption:
        status, detail, exit_code = "interrupted", f"wrapper received {interruption.signal_name}", EXIT_INTERRUPTED
        try:
            request_interrupt()
            terminal = wait_for_terminal(INTERRUPT_GRACE_SECONDS)
            run_result["remoteTurnStatus"] = terminal.get("status")
        except (BrokenPipeError, ConnectionClosed, ReadTimeout, RuntimeError):
            pass
    except (BrokenPipeError, ConnectionClosed, ReadTimeout) as error:
        if run_result["threadId"]:
            status, detail, exit_code = (
                "disconnected",
                f"app-server connection closed: {error or type(error).__name__}",
                EXIT_DISCONNECTED,
            )
        else:
            status, detail, exit_code = (
                "unavailable",
                f"app-server daemon unavailable: {error or type(error).__name__}",
                EXIT_UNAVAILABLE,
            )
    except Exception as error:
        status, detail, exit_code = "failed", str(error), EXIT_FAILED
    finally:
        if partial_message and not last_message:
            last_message = partial_message
        run_result["status"] = status
        run_result["error"] = detail or None
        run_result["exitCode"] = exit_code
        write_outputs()
        events.close()
        close_proxy()
    print(f"finished {status} {run_result['threadId'] or 'no-thread'}", flush=True)
    return exit_code


sys.exit(main())
