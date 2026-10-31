#!/usr/bin/env python3
"""scripts/phase-times.py <command...>: runs it, and at each `§T <label>` line it prints takes the process's CPU time; prints each
label with the CPU seconds until the next one, the biggest first, and stops the run after the first `emit done`."""
import subprocess, sys

def cpu_seconds(pid):
    text = subprocess.run(["ps", "-o", "time=", "-p", str(pid)], capture_output=True, text=True).stdout.strip()
    total = 0.0
    for part in text.replace("-", ":").split(":"):
        total = total * 60 + float(part)
    return total

process = subprocess.Popen(sys.argv[1:], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
marks = []
for line in process.stdout:
    if line.startswith("§T "):
        label = line[3:].strip()
        marks.append((label, cpu_seconds(process.pid)))
        if label == "emit done":
            marks.append(("end", cpu_seconds(process.pid)))
            process.kill()
            break
    else:
        print(line, end="")
spans = {}
for (label, start), (_, stop) in zip(marks, marks[1:]):
    spans[label] = spans.get(label, 0) + stop - start
total = marks[-1][1] - marks[0][1] if marks else 0
print(f"{total:.1f} s CPU from the first mark to the last")
for label, seconds in sorted(spans.items(), key=lambda item: -item[1]):
    print(f"{seconds:8.1f} s {100 * seconds / max(total, 1e-9):5.1f}%  {label}")
