#!/usr/bin/env python3
"""Exercise the scheduled Codex runner against the real desktop app-server daemon."""

import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import time


REPOSITORY = Path(__file__).resolve().parents[1]
RUNNER = REPOSITORY / "notes/agents/codex_app_run.py"
PROBE_ROOT = REPOSITORY / "data/agents/codex/lifecycle-probe"
SCHEDULED_CWD = Path(os.environ.get("CODEX_PROBE_CWD", "/Users/me/dev/angles/warp"))
PROCESS_EXIT_GRACE_SECONDS = 5
PROBE_TIMEOUT_SECONDS = 30
TURN_TIMEOUT_SECONDS = 3
START_TIMEOUT_SECONDS = 10


def wait_for_process(process):
    try:
        return process.communicate(timeout=PROBE_TIMEOUT_SECONDS)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGTERM)
        try:
            process.communicate(timeout=PROCESS_EXIT_GRACE_SECONDS)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.communicate()
        raise AssertionError(f"runner stayed alive longer than {PROBE_TIMEOUT_SECONDS}s")


def start_probe(label, prompt, turn_timeout_seconds):
    run_directory = PROBE_ROOT / time.strftime(f"%Y-%m-%d-%H%M%S-{label}")
    run_directory.mkdir(parents=True)
    events_path = run_directory / "events.jsonl"
    artifact_path = run_directory / "last.md"
    result_path = run_directory / "result.json"
    environment = os.environ | {"CODEX_TURN_TIMEOUT_SECONDS": str(turn_timeout_seconds)}
    process = subprocess.Popen(
        [
            "/usr/bin/python3",
            str(RUNNER),
            str(SCHEDULED_CWD),
            f"codex cron lifecycle {label} probe {time.strftime('%H:%M:%S')}",
            prompt,
            str(events_path),
            str(artifact_path),
            str(result_path),
        ],
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        start_new_session=True,
    )
    return process, events_path, artifact_path, result_path


def finished_probe(process, artifact_path, result_path):
    output, _ = wait_for_process(process)
    result = json.loads(result_path.read_text())
    assert artifact_path.read_text().strip(), f"{label} did not leave a final artifact"
    assert result["threadId"], result
    assert result["turnId"], result
    assert "started " in output, output
    return process.returncode, output, artifact_path, result


def run_probe(label, prompt, turn_timeout_seconds):
    process, _events_path, artifact_path, result_path = start_probe(label, prompt, turn_timeout_seconds)
    return finished_probe(process, artifact_path, result_path)


def wait_until_turn_started(events_path):
    deadline = time.monotonic() + START_TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        if events_path.exists() and '"method":"turn/started"' in events_path.read_text().replace(" ", ""):
            return
        time.sleep(0.1)
    raise AssertionError(f"turn did not start within {START_TIMEOUT_SECONDS}s: {events_path}")


def run_success_probe():
    return_code, output, artifact_path, result = run_probe("success", "Reply with just: ok", 60)
    assert return_code == 0, f"expected completed exit 0, got {return_code}:\n{output}"
    assert result["status"] == "completed", result
    assert "finished completed " in output, output
    assert artifact_path.read_text().strip() == "ok", artifact_path.read_text()
    print(f"completed lifecycle passed: {result['threadId']} -> {artifact_path}")


def run_poll_probe():
    return_code, output, artifact_path, result = run_probe(
        "poll",
        "Run `/bin/sleep 18` in the shell, then reply with just: ok",
        45,
    )
    assert return_code == 0, f"expected completed exit 0 after polling, got {return_code}:\n{output}"
    assert result["status"] == "completed", result
    assert artifact_path.read_text().strip() == "ok", artifact_path.read_text()
    print(f"quiet-turn polling passed: {result['threadId']} -> {artifact_path}")


def run_timeout_probe():
    return_code, output, artifact_path, result = run_probe(
        "timeout",
        "Run `/bin/sleep 30` in the shell, then reply with just: late",
        TURN_TIMEOUT_SECONDS,
    )
    assert return_code == 4, f"expected timeout exit 4, got {return_code}:\n{output}"
    assert result["status"] == "timedOut", result
    assert result["threadId"], result
    assert result["turnId"], result
    assert "started " in output and "finished timedOut " in output, output
    print(f"timeout lifecycle passed: {result['threadId']} -> {artifact_path}")


def run_interrupt_probe():
    process, events_path, artifact_path, result_path = start_probe(
        "interrupt",
        "Run `/bin/sleep 30` in the shell, then reply with just: late",
        60,
    )
    wait_until_turn_started(events_path)
    process.send_signal(signal.SIGTERM)
    return_code, output, artifact_path, result = finished_probe(process, artifact_path, result_path)
    assert return_code == 3, f"expected interrupted exit 3, got {return_code}:\n{output}"
    assert result["status"] == "interrupted", result
    assert result["remoteTurnStatus"] == "interrupted", result
    assert "finished interrupted " in output, output
    print(f"interrupt lifecycle passed: {result['threadId']} -> {artifact_path}")


def run_disconnect_probe():
    process, events_path, artifact_path, result_path = start_probe(
        "disconnect",
        "Run `/bin/sleep 30` in the shell, then reply with just: late",
        60,
    )
    wait_until_turn_started(events_path)
    child_pids = subprocess.check_output(["pgrep", "-P", str(process.pid)], text=True).split()
    assert len(child_pids) == 1, f"expected one app-server proxy child, got {child_pids}"
    os.kill(int(child_pids[0]), signal.SIGTERM)
    return_code, output, artifact_path, result = finished_probe(process, artifact_path, result_path)
    assert return_code == 5, f"expected disconnected exit 5, got {return_code}:\n{output}"
    assert result["status"] == "disconnected", result
    assert "finished disconnected " in output, output
    print(f"disconnect lifecycle passed: {result['threadId']} -> {artifact_path}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("scenario", choices=["success", "poll", "timeout", "interrupt", "disconnect"])
    scenario = parser.parse_args().scenario
    if scenario == "success":
        run_success_probe()
    elif scenario == "poll":
        run_poll_probe()
    elif scenario == "timeout":
        run_timeout_probe()
    elif scenario == "interrupt":
        run_interrupt_probe()
    else:
        run_disconnect_probe()


if __name__ == "__main__":
    main()
