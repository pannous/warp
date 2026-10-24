#!/usr/bin/env python3
"""Where a native warp run spends its time: the Time Profiler samples of xctrace, as self and total milliseconds per
function. Records when given a command, else summarizes an exported time-profile table.
Usage: xctrace_summary.py <trace dir> [--callers <name part>] [-- command args…]
       --callers: the time of the outermost function naming the part, by the two frames calling it   (WARP_* environment passes through)
       e.g. probes/perf/xctrace_summary.py data/perf/vis.trace -- ~/dev/bin/warp compile --wasm samples/sound/visualizer.warp"""
import collections, os, re, subprocess, sys, xml.etree.ElementTree as ElementTree

SHOWN = 30
# a launch xctrace never sees end (it hung once for an hour) stops here
TIME_LIMIT = "120s"
# Rust symbols carry a hash and generic noise; the path's last two parts name a function well enough
HASH = re.compile(r"::h[0-9a-f]{16}$")


def record(trace, command):
	subprocess.run(["rm", "-rf", trace])
	subprocess.run(["xcrun", "xctrace", "record", "--template", "Time Profiler", "--output", trace, "--time-limit", TIME_LIMIT, "--launch", "--", *command], capture_output=True)


def exported_rows(trace):
	xpath = '/trace-toc/run[@number="1"]/data/table[@schema="time-profile"]'
	text = subprocess.run(["xcrun", "xctrace", "export", "--input", trace, "--xpath", xpath], capture_output=True, text=True).stdout
	return ElementTree.fromstring(text).iter("row")


def short(name):
	return HASH.sub("", name)[-140:]


def summary(trace, callee=None):
	frames, backtraces = {}, {}
	own, total, callers = collections.Counter(), collections.Counter(), collections.Counter()
	for row in exported_rows(trace):
		weight = row.find("weight")
		milliseconds = int(weight.text) / 1e6 if weight is not None and weight.text else 1.0
		stack = row.find("tagged-backtrace")
		stack = stack if stack is not None else row.find("backtrace")
		if stack is None:
			continue
		if stack.get("ref"):
			names = backtraces[stack.get("ref")]
		else:
			names = []
			for frame in stack.iter("frame"):
				if frame.get("ref"):
					names.append(frames[frame.get("ref")])
				else:
					frames[frame.get("id")] = short(frame.get("name", "?"))
					names.append(frames[frame.get("id")])
			backtraces[stack.get("id")] = names
		if names:
			own[names[0]] += milliseconds
			for name in set(names):
				total[name] += milliseconds
			if callee:
				depths = [depth for depth, name in enumerate(names) if callee in name]
				if depths:
					callers[" < ".join(names[depths[-1] + 1:depths[-1] + 3])] += milliseconds
	if callee:
		print(f"callers of {callee}")
		for name, milliseconds in callers.most_common(SHOWN):
			print(f"{milliseconds:8.0f}  {name}")
		return
	print(f"self ms (of {sum(own.values()):.0f})")
	for name, milliseconds in own.most_common(SHOWN):
		print(f"{milliseconds:8.0f}  {name}")
	print("\ntotal ms")
	for name, milliseconds in total.most_common(SHOWN * 2):
		print(f"{milliseconds:8.0f}  {name}")


def main():
	trace = sys.argv[1]
	if "--" in sys.argv:
		record(trace, sys.argv[sys.argv.index("--") + 1:])
	callee = sys.argv[sys.argv.index("--callers") + 1] if "--callers" in sys.argv else None
	summary(trace, callee)


main()
