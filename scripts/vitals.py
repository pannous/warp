#!/usr/bin/env python3
"""Warp vitals: the compiler's weight per landing and the test pass rates over time, one page.

scripts/vitals.py [since] [out.html]  (defaults: 2026-10-29, the start of self-hosting; data/vitals/vitals.html)
Weight: lines of code after every commit of origin/main, each by what it changed itself (a merge: its resolution),
the core without extensions and the extensions apart. Tests: every committed version of each results file (TIERS),
its pass count read from its summary or its status lines. Build time: the CPU seconds and peak memory of each
landing's compiler builds, read from the build records in the store (STORE). All of it is reconstructed (seconds) into scripts/vitals.html. The nightly run
(scripts/integrate/nightly.sh) calls it; a Claude session republishes the page to its artifact URL
(notes/core_hygiene.md "Warp vitals").
"""
import collections, datetime, glob, json, os, re, subprocess, sys

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
GITHUB_REPO = "pannous/warp"
BOARD_CARD = "https://github.com/users/pannous/projects/1?pane=issue&itemId=%s"
RECENTLY_DONE = 8
FLIPS_SHOWN = 8  # named tests per lane that started or stopped passing
CARD_SNAPSHOTS = os.path.join(REPO, "data/vitals/cards.jsonl")  # one line per snapshot of the board's columns
STORE = os.environ.get("WARP_STORE", "/Volumes/Build/compilers")  # build records (notes/compiler_rebuilds.md)
# (name, record, whether one compiler, the seed, runs many of them: its first run is cold; warp-self's compiler is the
# record's own warp.wasm, run once, so its records are alike)
BUILDS = [("warp.wasm, built by the seed", "warp.record", True),
          ("self-compile: warp.wasm compiles itself", "warp-self.record", False)]
HELLO = "samples/hello/hello.warp"
HELLO_RUNS = 3  # the cheapest of these runs counts: instructions and wall time only ever gain from the machine's load
INSTALLS = os.path.expanduser("~/dev/bin/warp-builds")  # each install of a main tip (after-push.sh): warp and its compilers
HELLO_SNAPSHOTS = os.path.join(REPO, "data/vitals/hello.jsonl")  # one line per measured install, kept as installs go
# measured by hand before the series began: (label, revision, 10^9 instructions, seconds)
HELLO_REFERENCES = [("1.4.1", "v1.4.1", 0.09, 0.01), ("1.5.1", "v1.5.1", 1.9, 0.22), ("1.6.1", "v1.6.1", 6.1, 0.47),
                    ("before hello-takes", "d152612c4d^1", 5.4, 0.53), ("hello-takes", "d152612c4d", 3.1, 0.22)]


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
    # errors="replace": a results file may hold a test's raw bytes (bd625bdbdb's has a lone 0x83)
    return subprocess.run(["git", "-C", REPO, *args], capture_output=True, text=True, errors="replace", check=True).stdout


# main read once: a fetch during the run must not mix two tips (KeyError in PARENTS)
TIP = git("rev-parse", "origin/main").strip()


def lines_at(commit):
    totals = collections.Counter()
    wanted = [f for f in git("ls-tree", "-r", "--name-only", commit).splitlines() if component(f)]
    blob = subprocess.run(["git", "-C", REPO, "cat-file", "--batch"], capture_output=True, check=True,
                          input="".join(f"{commit}:{f}\n" for f in wanted).encode()).stdout
    position = 0
    for f in wanted:
        header_end = blob.index(b"\n", position)
        size = int(blob[position:header_end].split()[-1])
        content = blob[header_end + 1:header_end + 1 + size]
        # as git's numstat counts them: a last line without its newline is a line
        totals[component(f)] += content.count(b"\n") + (not content.endswith(b"\n") and bool(content))
        position = header_end + 2 + size
    return totals


def group_sums(counter):
    return [sum(counter.get(c, 0) for c in names) for _, names in CORE_GROUPS]


def first_parent_changes(*revisions):
    """Each commit's change against its first parent, per component, from one git log."""
    changes, sha = {}, None
    for line in git("log", "--numstat", "--no-renames", "--diff-merges=first-parent", "--format=@%H", *revisions).splitlines():
        if line.startswith("@"):
            sha = line[1:]
            changes[sha] = collections.Counter()
        elif line.count("\t") >= 2:
            added, removed, path = line.split("\t", 2)
            if added != "-" and component(path):
                changes[sha][component(path)] += int(added) - int(removed)
    return changes


def history_graph():
    """main's whole history, parents before children: each commit's parents and its ancestors as a bitset of indices."""
    order, parents, ancestors = [], {}, {}
    for line in git("rev-list", "--topo-order", "--reverse", "--parents", TIP).splitlines():
        sha, *its_parents = line.split()
        parents[sha] = its_parents
        ancestors[sha] = 1 << len(order)
        for parent in its_parents:
            ancestors[sha] |= ancestors[parent]
        order.append(sha)
    return order, parents, ancestors


def members(bits):
    while bits:
        lowest = bits & -bits
        yield ORDER[lowest.bit_length() - 1]
        bits ^= lowest


own_changes = {}
def own_change(sha):
    """What one commit changed itself, per component: a commit against its parent; a merge its whole change against its
    first parent less what the commits it brought in changed (their own changes), so only its resolution."""
    if sha not in own_changes:
        if sha not in whole_changes:  # a commit from before the base that a later merge brought in
            whole_changes.update(first_parent_changes("-1", sha))
        change = whole_changes[sha].copy()
        first_parent, *others = PARENTS[sha]
        if others:
            for brought in members(ANCESTORS[sha] & ~ANCESTORS[first_parent] & ~(1 << ORDER_INDEX[sha])):
                change.subtract(own_change(brought))
        own_changes[sha] = change
    return own_changes[sha]


ORDER, PARENTS, ANCESTORS = history_graph()
ORDER_INDEX = {sha: index for index, sha in enumerate(ORDER)}


# One row per commit by what it changed itself: measured against its first parent, a merge is charged with everything
# main gained since its branch forked when a worker pushes "Merge origin/main into <branch>" (the branch is its first
# parent), so the atomic commits carry their own lines and a merge only its resolution (merges resolving nothing are
# left out). Every commit since main's first-parent state before SINCE counts once, so the running totals end at the
# tip's (checked below).
base = git("rev-list", "-1", "--first-parent", "--before=" + SINCE, TIP).strip()
whole_changes = first_parent_changes(f"{base}..{TIP}")
commits = git("log", "--topo-order", "--reverse", "--format=%H\t%P\t%ad\t%s", "--date=iso-strict",
              f"{base}..{TIP}").splitlines()
totals = lines_at(base)
rows = []
for line in commits:
    sha, parents, date, subject = line.split("\t", 3)
    change = own_change(sha)
    totals.update(change)
    if len(parents.split()) > 1 and not any(change.values()):
        continue
    rows.append([date[:16].replace("T", " "), sha[:10], subject[:140], group_sums(totals) + [totals[EXTENSIONS]],
                 sum(group_sums(change)), change[EXTENSIONS], totals["lib"], totals["test"]])
tip = lines_at(TIP)
drift = sorted(name for name in set(tip) | set(totals) if tip[name] != totals[name])
if drift:
    print("vitals: atomic commits sum to other totals than main's tip (merge resolutions): " +
          ", ".join(f"{name} {totals[name]} summed, {tip[name]} at the tip" for name in drift), file=sys.stderr)

def pass_count(text):
    for line in text.splitlines()[:3] + text.splitlines()[-3:]:
        if match := SUMMARY.search(line):
            failed, total = map(int, match.groups())
            return total - failed, total
        if match := HEADER.match(line):
            return tuple(map(int, match.groups()))
    statuses = [m.group(1) for m in map(STATUS.match, text.splitlines()) if m]
    return (sum(s in PASSING for s in statuses), len(statuses)) if statuses else None


def outcomes(text):
    """Each test's name (its file without the line number or a sample's reason, and its code) and whether it passed;
    None for one that hit a time limit, which says nothing about the code on a loaded machine."""
    named = {}
    for line in text.splitlines():
        if match := STATUS.match(line):
            name = re.sub(r"^(\S+\.warp)(:\d+)?: .*", r"\1", line[match.end():].strip().split(": got ")[0])
            name = re.sub(r"(\.warp):\d+", r"\1", name)
            named[name] = None if match.group(1) == "TIME" else match.group(1) in PASSING
    return named


def latest_flips(path):
    """The tests the newest run of a lane passes and the run before failed, and the other way round."""
    runs = git("log", "-2", "--format=%H %ad", "--date=iso-strict", TIP, "--", path).splitlines()
    if len(runs) < 2:
        return None
    (new, date), (old, _) = (run.split(" ") for run in runs)
    before, after = outcomes(git("show", f"{old}:{path}")), outcomes(git("show", f"{new}:{path}"))
    if any(before.values()) or any(after.values()):
        fixed = [name for name, ok in after.items() if ok and before.get(name) is False]
        broke = [name for name, ok in after.items() if ok is False and before.get(name) is True]
    else:  # a lane listing only its failures (results-cloud.txt): a failure gone passes now, unless its test went
        fixed = [name for name in before if name not in after]
        broke = [name for name in after if name not in before]
    return [date[:16].replace("T", " "), old[:10], new[:10], fixed[:FLIPS_SHOWN], broke[:FLIPS_SHOWN], len(fixed), len(broke)]


def tier_history(path):
    points = []
    for line in git("log", "--reverse", "--format=%H %ad", "--date=iso-strict", "--since=" + SINCE, TIP, "--", path).splitlines():
        sha, date = line.split(" ")
        counts = pass_count(git("show", f"{sha}:{path}"))
        if counts and counts[1]:
            points.append([date[:16].replace("T", " "), sha[:10], *counts])
    return points


def record_field(path, key):
    with open(path) as lines:
        return next((line.split() for line in lines if line.split()[1:2] == [key] or line.split()[:1] == [key]), None)


def build_history(record, by_seed, landed):
    """Each landing's instructions retired (in 10^12), peak memory and CPU seconds of building <record>, its leanest
    full build (a lib-only build reusing a stored compiler skipped: its log has no seed step; older ones say rebake), by the commit the record says it built: a build of uncommitted edits says
    `<sha>+dirty` (rebuild.sh built_commit) and is no landing's. Instructions, as CPU seconds swing with the machine's
    load and cores; <by_seed>: a seed's first build is left out, a compiler's cold first run reads ~3% more
    (notes/compiler_rebuilds.md)."""
    builds = []
    for folder in glob.glob(os.path.join(STORE, "*", record)):
        try:
            commit = record_field(os.path.join(folder, "inputs.txt"), "commit")[1]
            seed = (record_field(os.path.join(folder, "inputs.txt"), "seed") or [None, None])[1]
            with open(os.path.join(folder, "build.log")) as log:
                text = log.read()
                # only a seed build has steps; warp-self's log (warp.wasm compiling itself) has none, and the step test
                # dropped every self-compile record since b9af7cea67
                lib_only = by_seed and "rebuild step seed" not in text
                if commit not in landed or "rebak" in text or lib_only:
                    continue
            timing = os.path.join(folder, "time")
            instructions = int(record_field(timing, "instructions")[0])
            cpu = sum(float(record_field(timing, key)[1]) for key in ("user", "sys"))
            peak = int(record_field(timing, "maximum")[0]) // 2**20
            megabytes = os.path.getsize(folder.removesuffix(".record") + ".wasm") / 1e6
            builds.append((os.path.getmtime(timing), seed, commit, instructions, peak, cpu, megabytes))
        except (OSError, TypeError, ValueError, IndexError):
            continue
    seen_seeds, leanest = set(), {}
    for _, seed, commit, instructions, peak, cpu, megabytes in sorted(builds):
        cold = by_seed and seed is not None and seed not in seen_seeds
        seen_seeds.add(seed)
        if not cold and (commit not in leanest or instructions < leanest[commit][2] * 1e12):
            leanest[commit] = [landed[commit], commit[:10], round(instructions / 1e12, 3), peak, round(cpu, 1), round(megabytes, 2)]
    return sorted(leanest.values())


def pushes_since():
    """Local times of the pushes to main since SINCE, from GitHub's activity record of the branch, newest first.
    git can't tell: a branch merged with main and pushed fast-forward puts main's own history on its second parent."""
    times, url = [], f"repos/{GITHUB_REPO}/activity?ref=refs/heads/main&activity_type=push&per_page=100"
    while url:
        response = subprocess.run(["gh", "api", "-i", url], capture_output=True, text=True, check=True).stdout
        head, body = response.split("\r\n\r\n" if "\r\n\r\n" in response else "\n\n", 1)
        local = [datetime.datetime.fromisoformat(event["timestamp"].replace("Z", "+00:00")).astimezone()
                 .strftime("%Y-%m-%d %H") for event in json.loads(body)]
        times += [t for t in local if t >= SINCE]
        following = re.search(r'<https://api\.github\.com/([^>]+)>; rel="next"', head)
        url = following.group(1) if following and local and min(local) >= SINCE else None
    return times


def hourly_pace():
    """Pushes to main and the commits reaching it (by commit date), per hour, every hour since SINCE."""
    hour = lambda date: date[:13].replace("T", " ")
    commits_per_hour = collections.Counter(hour(d) for d in git("log", "--format=%cd", "--date=iso-strict",
                                                                 "--since=" + SINCE, TIP).split())
    landings_per_hour = collections.Counter(pushes_since())
    first = datetime.datetime.strptime(min(commits_per_hour), "%Y-%m-%d %H")
    hours = []
    while (label := first.strftime("%Y-%m-%d %H")) <= max(commits_per_hour):
        hours.append([label, landings_per_hour[label], commits_per_hour[label]])
        first += datetime.timedelta(hours=1)
    return hours


def recently_done(cards):
    """The RECENTLY_DONE newest Done cards: when, key, title and their link on the board."""
    done = sorted((c for c in cards if c["column"] == "Done"), key=lambda c: c["column_since"])[-RECENTLY_DONE:][::-1]
    ids = json.dumps([card["id"] for card in done])
    nodes = json.loads(subprocess.run(["gh", "api", "graphql", "-f", "query=query { nodes(ids: %s) { ... on "
                                       "ProjectV2Item { databaseId } } }" % ids], capture_output=True, text=True,
                                      check=True).stdout)["data"]["nodes"]
    local = lambda t: datetime.datetime.fromisoformat(t.replace("Z", "+00:00")).astimezone().strftime("%Y-%m-%d %H:%M")
    return [[local(card["column_since"]), card["key"], card["title"], BOARD_CARD % node["databaseId"]]
            for card, node in zip(done, nodes)]


def card_counts():
    """The board's cards per column, one snapshot an hour kept in CARD_SNAPSHOTS (todo list costs GraphQL quota)."""
    snapshots = []
    if os.path.exists(CARD_SNAPSHOTS):
        with open(CARD_SNAPSHOTS) as kept:
            snapshots = [json.loads(line) for line in kept if line.strip()]
    now = datetime.datetime.now()
    if not snapshots or now - datetime.datetime.fromisoformat(snapshots[-1]["time"]) > datetime.timedelta(hours=1):
        listing = subprocess.run(["todo", "list", "--json"], capture_output=True, text=True, cwd=REPO)
        if listing.returncode == 0:
            cards = json.loads(listing.stdout)
            counts = collections.Counter(card["column"] for card in cards)
            snapshots.append({"time": now.isoformat(timespec="minutes"), "columns": dict(counts),
                              "done": recently_done(cards)})
            os.makedirs(os.path.dirname(CARD_SNAPSHOTS), exist_ok=True)
            with open(CARD_SNAPSHOTS, "a") as kept:
                kept.write(json.dumps(snapshots[-1]) + "\n")
        else:
            print("vitals: todo list failed, cards from the last snapshot: " + listing.stderr.strip()[-200:], file=sys.stderr)
    return snapshots


def hello_cost(warp):
    """The cheapest of HELLO_RUNS runs of `<warp> HELLO` (compile and run): 10^9 instructions retired and wall
    seconds, by /usr/bin/time -l; None when it doesn't say Hello."""
    runs = []
    for _ in range(HELLO_RUNS):
        run = subprocess.run(["/usr/bin/time", "-l", warp, HELLO], capture_output=True, text=True, cwd=REPO,
                             env={**os.environ, "WARP_NO_WINDOW": "1"})
        instructions, seconds = re.search(r"(\d+)\s+instructions retired", run.stderr), re.search(r"([\d.]+) real", run.stderr)
        if "Hello" not in run.stdout or not instructions or not seconds:
            print(f"vitals: {warp} {HELLO} failed: {(run.stdout + run.stderr).strip()[-200:]}", file=sys.stderr)
            return None
        runs.append((int(instructions.group(1)) / 1e9, float(seconds.group(1))))
    return round(min(r[0] for r in runs), 2), round(min(r[1] for r in runs), 2)


def commit_date(revision):
    return git("log", "-1", "--format=%cd", "--date=iso-strict", revision).strip()[:16].replace("T", " ")


def hello_track():
    """hello's compile and run per main tip: each install in INSTALLS not measured yet is measured once (its
    compilers' commit, warp.wasm.inputs) and kept in HELLO_SNAPSHOTS; then HELLO_REFERENCES, measured by hand."""
    measured = []
    if os.path.exists(HELLO_SNAPSHOTS):
        with open(HELLO_SNAPSHOTS) as kept:
            measured = [json.loads(line) for line in kept if line.strip()]
    known = {point["commit"] for point in measured}
    for folder in sorted(glob.glob(os.path.join(INSTALLS, "*"))):
        try:
            with open(os.path.join(folder, "warp.wasm.inputs")) as inputs:
                commit = re.search(r"commit=(\w+)", inputs.read()).group(1)
        except (OSError, AttributeError):
            continue
        if commit in known or not (cost := hello_cost(os.path.join(folder, "warp"))):
            continue
        known.add(commit)
        measured.append({"time": commit_date(commit), "commit": commit[:10], "instructions": cost[0], "seconds": cost[1]})
        with open(HELLO_SNAPSHOTS, "a") as kept:
            kept.write(json.dumps(measured[-1]) + "\n")
    series = sorted([p["time"], p["commit"], p["instructions"], p["seconds"], ""] for p in measured)
    references = [[commit_date(revision), git("rev-parse", "--short=10", revision).strip(), instructions, seconds, label]
                  for label, revision, instructions, seconds in HELLO_REFERENCES]
    return {"series": series, "references": references}


landed = {line.split()[0]: line.split()[1][:16].replace("T", " ") for line in
          git("log", "--format=%H %ad", "--date=iso-strict", "--since=" + SINCE, TIP).splitlines()}
data = {"cards": card_counts(), "hello": hello_track(), "pace": hourly_pace(), "builds": [[name, record, build_history(record, by_seed, landed)] for name, record, by_seed in BUILDS], "tiers": [[name, path, tier_history(path), latest_flips(path)] for name, path in TIERS], "groups": [name for name, _ in CORE_GROUPS] + [EXTENSIONS], "base": base[:10], "rows": rows}
os.makedirs(os.path.dirname(OUT), exist_ok=True)
with open(TEMPLATE) as template, open(OUT, "w") as page:
    page.write(template.read().replace("__DATA__", json.dumps(data, separators=(",", ":")).replace("</", "<\\/")))
first, last = rows[0], rows[-1]
core = lambda row: sum(row[3][:-1])
latest = "; ".join(f"{name} {points[-1][2]}/{points[-1][3]}" for name, _, points, _ in data["tiers"] if points)
builds = "; ".join(f"{record} {points[-1][2]} T instructions ({points[-1][4]} s CPU)" for _, record, points in data["builds"] if points)
print(f"{latest}; {builds}; core {core(first)} -> {core(last)}, extensions {first[3][-1]} -> {last[3][-1]}, "
      f"lib {first[6]} -> {last[6]}, test {first[7]} -> {last[7]} over {len(rows)} commits: {OUT}")
