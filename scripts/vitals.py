#!/usr/bin/env python3
"""Warp vitals: the compiler's weight per landing and the test pass rates over time, one page.

scripts/vitals.py [since] [out.html]  (defaults: 2026-10-29, the start of self-hosting; data/vitals/vitals.html)
Weight: lines of code for every first-parent commit of origin/main, the core without extensions and the extensions
apart. Tests: every committed version of each results file (TIERS), its pass count read from its summary or its
status lines. Build time: the CPU seconds and peak memory of each landing's compiler builds, read from the build
records in the store (STORE). All of it is reconstructed (seconds) into scripts/vitals.html. The nightly run
(scripts/integrate/nightly.sh) calls it; a Claude session republishes the page to its artifact URL
(notes/core_hygiene.md "Warp vitals").
"""
import collections, glob, json, os, re, subprocess, sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SINCE = sys.argv[1] if len(sys.argv) > 1 else "2026-10-29"
OUT = sys.argv[2] if len(sys.argv) > 2 else os.path.join(REPO, "data/vitals/vitals.html")
TEMPLATE = os.path.join(REPO, "scripts/vitals.html")
CODE_SUFFIXES = (".warp", ".c", ".h", ".js", ".sh", ".py")
CORE_GROUPS = [("parsing", ["core/parsing"]), ("passes", ["core/passes"]),
               ("analysis", ["core/analysis", "core/commands", "core/(top)"]), ("emitter", ["core/emitter"]),
               ("runtime", ["core/runtime"]), ("native", ["core/native"])]
EXTENSIONS = "extensions"  # extensions/; extensions/ and lib/sugar/ before it moved there (2026-11-03)
TIERS = [("native pairs and is! lines", "test/results.txt"), ("cloud full suite", "test/results-cloud.txt"),
         ("samples, native", "test/samples.txt"), ("samples, playground", "test/samples-web.txt"),
         ("tests, playground", "test/tests-web.txt")]
SUMMARY = re.compile(r"(\d+) of (\d+) failed")
HEADER = re.compile(r"^# \S+ \S+ \S+ (\d+)/(\d+)")
STATUS = re.compile(r"^(ok|SRV|ran|✓|FAIL|✗|BOTH|PAGE|EXP|NONE|WIN|DIFF|TIME|ERR)\s")
PASSING = ("ok", "SRV", "ran", "✓")  # ran: a sample without an expected output that ran without error
STORE = os.environ.get("WARP_STORE", "/Volumes/Build/compilers")  # build records (notes/compiler_rebuilds.md)
BUILDS = [("warp.wasm, built by the seed", "warp.record"), ("self-compile: warp.wasm compiles itself", "warp-self.record")]


def component(path):
    if not path.endswith(CODE_SUFFIXES):
        return None
    parts = path.split("/")
    if parts[0] == "extensions" or parts[0] == "core" and len(parts) > 2 and parts[1] == "extensions":
        return EXTENSIONS
    if parts[0] == "core":
        return "core/" + parts[1] if len(parts) > 2 else "core/(top)"
    if parts[0] == "lib":
        return EXTENSIONS if len(parts) > 2 and parts[1] == "sugar" else "lib"
    return parts[0] if parts[0] in ("test", "samples") else None


def git(*args):
    return subprocess.run(["git", "-C", REPO, *args], capture_output=True, text=True, check=True).stdout


def lines_at(commit):
    totals = collections.Counter()
    wanted = [f for f in git("ls-tree", "-r", "--name-only", commit).splitlines() if component(f)]
    blob = subprocess.run(["git", "-C", REPO, "cat-file", "--batch"], capture_output=True, check=True,
                          input="".join(f"{commit}:{f}\n" for f in wanted).encode()).stdout
    position = 0
    for f in wanted:
        header_end = blob.index(b"\n", position)
        size = int(blob[position:header_end].split()[-1])
        totals[component(f)] += blob[header_end + 1:header_end + 1 + size].count(b"\n")
        position = header_end + 2 + size
    return totals


def group_sums(counter):
    return [sum(counter.get(c, 0) for c in names) for _, names in CORE_GROUPS]


commits = git("log", "--first-parent", "--reverse", "--format=%H %ad %s", "--date=iso-strict",
              "--since=" + SINCE, "origin/main").splitlines()
base = git("rev-parse", commits[0].split()[0] + "^1").strip()
totals = lines_at(base)
rows = []
for line in commits:
    sha, date, subject = line.split(" ", 2)
    change = collections.Counter()
    for stat in git("diff", "--numstat", "--no-renames", sha + "^1", sha).splitlines():
        added, removed, path = stat.split("\t", 2)
        if added != "-" and component(path):
            change[component(path)] += int(added) - int(removed)
    totals.update(change)
    rows.append([date[:16].replace("T", " "), sha[:10], subject[:140], group_sums(totals) + [totals[EXTENSIONS]],
                 sum(group_sums(change)), change[EXTENSIONS], totals["lib"], totals["test"]])

def pass_count(text):
    for line in text.splitlines()[:3] + text.splitlines()[-3:]:
        if match := SUMMARY.search(line):
            failed, total = map(int, match.groups())
            return total - failed, total
        if match := HEADER.match(line):
            return tuple(map(int, match.groups()))
    statuses = [m.group(1) for m in map(STATUS.match, text.splitlines()) if m]
    return (sum(s in PASSING for s in statuses), len(statuses)) if statuses else None


def tier_history(path):
    points = []
    for line in git("log", "--reverse", "--format=%H %ad", "--date=iso-strict", "--since=" + SINCE, "origin/main", "--", path).splitlines():
        sha, date = line.split(" ")
        counts = pass_count(git("show", f"{sha}:{path}"))
        if counts and counts[1]:
            points.append([date[:16].replace("T", " "), sha[:10], *counts])
    return points


def record_field(path, key):
    with open(path) as lines:
        return next((line.split() for line in lines if line.split()[1:2] == [key] or line.split()[:1] == [key]), None)


def build_history(record, landed):
    """Each landing's CPU seconds and peak memory of building <record>, its fastest full build (rebakes skipped)."""
    fastest = {}
    for folder in glob.glob(os.path.join(STORE, "*", record)):
        try:
            commit = record_field(os.path.join(folder, "inputs.txt"), "commit")[1]
            with open(os.path.join(folder, "build.log")) as log:
                if commit not in landed or "rebak" in log.read():
                    continue
            cpu = sum(float(record_field(os.path.join(folder, "time"), key)[1]) for key in ("user", "sys"))
            peak = int(record_field(os.path.join(folder, "time"), "maximum")[0]) // 2**20
        except (OSError, TypeError, ValueError, IndexError):
            continue
        if commit not in fastest or cpu < fastest[commit][2]:
            fastest[commit] = [landed[commit], commit[:10], round(cpu, 1), peak]
    return sorted(fastest.values())


def hourly_pace():
    """Landings (first-parent commits of main) and all commits reaching main, per hour, every hour since SINCE."""
    import datetime
    hour = lambda date: date[:13].replace("T", " ")
    commits_per_hour = collections.Counter(hour(d) for d in git("log", "--format=%ad", "--date=iso-strict",
                                                                 "--since=" + SINCE, "origin/main").split())
    landings_per_hour = collections.Counter(hour(row[0]) for row in rows)
    first = datetime.datetime.strptime(min(commits_per_hour), "%Y-%m-%d %H")
    hours = []
    while (label := first.strftime("%Y-%m-%d %H")) <= max(commits_per_hour):
        hours.append([label, landings_per_hour[label], commits_per_hour[label]])
        first += datetime.timedelta(hours=1)
    return hours


landed = {line.split()[0]: line.split()[1][:16].replace("T", " ") for line in commits}
data = {"pace": hourly_pace(), "builds": [[name, record, build_history(record, landed)] for name, record in BUILDS], "tiers": [[name, path, tier_history(path)] for name, path in TIERS], "groups": [name for name, _ in CORE_GROUPS] + [EXTENSIONS], "base": base[:10], "rows": rows}
os.makedirs(os.path.dirname(OUT), exist_ok=True)
with open(TEMPLATE) as template, open(OUT, "w") as page:
    page.write(template.read().replace("__DATA__", json.dumps(data, separators=(",", ":")).replace("</", "<\\/")))
first, last = rows[0], rows[-1]
core = lambda row: sum(row[3][:-1])
latest = "; ".join(f"{name} {points[-1][2]}/{points[-1][3]}" for name, _, points in data["tiers"] if points)
builds = "; ".join(f"{record} {points[-1][2]} s" for _, record, points in data["builds"] if points)
print(f"{latest}; {builds}; core {core(first)} -> {core(last)}, extensions {first[3][-1]} -> {last[3][-1]}, "
      f"lib {first[6]} -> {last[6]}, test {first[7]} -> {last[7]} over {len(rows)} landings: {OUT}")
