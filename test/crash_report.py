"""The report of test.sh's rerun of a crashed test binary: libtest's JSON events on stdin (a test program's own output
may glue onto them or come between), out the failed tests with their panic, the tests still running when the rerun
crashed too, and the counts. Lines of the test queue (a kill over the time or memory limit) pass through."""
import json
import re
import sys

EVENT_START = re.compile(r'\{ ?"type": ?"')
PANIC_MARK = "panicked at"
PANIC_LINES = 2  # the location and the first line of the message


def events(lines):
    for line in lines:
        if line.startswith("test queue:"):
            print(line.rstrip())
        start = EVENT_START.search(line)
        if not start:
            continue
        try:
            yield json.loads(line[start.start() :])
        except json.JSONDecodeError:
            continue


def panic_of(output):
    lines = output.splitlines()
    for index, line in enumerate(lines):
        if PANIC_MARK in line:
            # from the mark on: the thread's id differs every run, and the report goes into test_results.txt
            panic = [line[line.index(PANIC_MARK) :], *lines[index + 1 : index + PANIC_LINES]]
            return " ".join(part.strip() for part in panic)
    return lines[-1].strip() if lines else "(no output)"


def main():
    running, passed, failed, suite_finished = set(), 0, [], False
    for event in events(sys.stdin):
        kind, outcome, name = event.get("type"), event.get("event"), event.get("name")
        if kind == "suite" and outcome in ("ok", "failed"):
            suite_finished = True
        if kind != "test":
            continue
        if outcome == "started":
            running.add(name)
            continue
        running.discard(name)
        if outcome == "ok":
            passed += 1
        elif outcome in ("failed", "timeout"):
            failed.append((name, panic_of(event.get("stdout", ""))))
    for name, panic in sorted(failed):
        print(f"✗ {name}: {panic}")
    if not suite_finished:
        print(f"CRASHED again, while running: {' '.join(sorted(running)) or '(no test started)'}")
    print(f"rerun: {passed} passed, {len(failed)} failed{'' if suite_finished else ', crashed'}")


main()
