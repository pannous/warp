#!/usr/bin/env python3
"""Warp vitals: the compiler's weight per landing and the test pass rates over time, one page.

scripts/vitals.py [since] [out.html]  (defaults: 2026-10-29, the start of self-hosting; data/vitals/vitals.html)
Weight: lines of code after every commit of origin/main, each by what it changed itself (a merge: its resolution),
the core without extensions and the extensions apart. Tests: every committed version of each results file (TIERS),
its pass count read from its summary or its status lines. Build time: the CPU seconds and peak memory of each
landing's compiler builds, read from the build records in the store (STORE). All of it is reconstructed (seconds) into scripts/vitals.html. The nightly run
(scripts/integrate/nightly.sh) calls it; a Claude session (the Integrator after a landing) republishes the page to its
artifact URL, https://claude.ai/code/artifact/b988ec61-123d-4e97-833b-0dd2342cac4c.
"""
import collections, concurrent.futures, datetime, glob, json, os, re, shutil, subprocess, sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CENSUS_JOB = "--census" in sys.argv  # the detached census job the hourly run starts, which writes no page
ARGUMENTS = [a for a in sys.argv[1:] if not a.startswith("--")]
SINCE = ARGUMENTS[0] if ARGUMENTS else "2026-10-29"
OUT = ARGUMENTS[1] if len(ARGUMENTS) > 1 else os.path.join(REPO, "data/vitals/vitals.html")
TEMPLATE = os.path.join(REPO, "scripts/vitals.html")
WAP_OUT = os.path.join(os.path.dirname(OUT), "wap-vitals.html")  # the same page with only wap's section (user 2026-11-20)
WAP_PAGE = [  # what the wap page says instead, applied to the filled template
    ("<title>Warp Vitals</title>", "<title>wap Vitals</title>"),
    ("<h1>Warp vitals</h1>", "<h1>wap vitals</h1>"),
    ("warp compiler · since self-hosting", "wap, the new compiler · its road to self-build"),
    ("How many tests pass in each lane, and how heavy the compiler is, rebuilt from git history.", "wap's size, self-build, tests and speed."),
    ("</style>", ".page > *:not(header):not(#wap-section) { display: none; }\n</style>"),
]
CODE_SUFFIXES = (".warp", ".c", ".h", ".js", ".sh", ".py")
CORE_GROUPS = [("parsing", ["core/parsing"]), ("passes", ["core/passes"]),
               ("analysis", ["core/analysis", "core/commands", "core/(top)"]), ("emitter", ["core/emitter"]),
               ("runtime", ["core/runtime"]), ("native", ["core/native"])]
EXTENSIONS = "extensions"  # extensions/; extensions/ and lib/sugar/ before it moved there
TIERS = [("native pairs and is! lines", "test/results.txt"),
         ("samples, native", "test/samples.txt"), ("samples, playground", "test/samples-web.txt"),
         ("tests, playground", "test/tests-web.txt")]
SUMMARY = re.compile(r"(\d+) of (\d+) failed")
RUN_MINUTES = re.compile(r" in (\d+) min$")  # a run's duration, which scripts/full-suite.sh ends its commit subject with
HEADER = re.compile(r"^# \S+ \S+ \S+ (\d+)/(\d+)")
STATUS = re.compile(r"^(ok|SRV|ran|✓|FAIL|✗|BOTH|PAGE|EXP|NONE|WIN|DIFF|TIME|ERR)\s")
PASSING = ("ok", "SRV", "ran", "✓")  # ran: a sample without an expected output that ran without error
GITHUB_REPO = "pannous/warp"
BOARD_CARD = "https://github.com/users/pannous/projects/1?pane=issue&itemId=%s"
RECENTLY_DONE = 8
FLIPS_SHOWN = 8  # named tests per lane that started or stopped passing
CARD_SNAPSHOTS = os.path.join(REPO, "data/vitals/cards.jsonl")  # one line per snapshot of the board's columns
# each commit's change against its first parent per component, kept between runs (a commit never changes): diffing
# main's 4000 merges again took 27 s a run (card vitals-py-slow)
CHANGES_CACHE = os.path.join(REPO, "data/vitals/changes.json")
BUILD_TIMES_SINCE = "2026-11-06"  # rebuild seconds before this date are outliers of the old build steps (user 11-12)
STORE = os.environ.get("WARP_STORE", "/Volumes/Build/compilers")  # build records (notes/compiler_rebuilds.md)
# (name, record, whether one compiler, the seed, runs many of them: its first run is cold; warp-self's compiler is the
# record's own warp.wasm, run once, so its records are alike)
BUILDS = [("warp.wasm, built by the seed", "warp.record", True),
          ("self-compile: warp.wasm compiles itself", "warp-self.record", False)]
HELLO = "test/gate/hello.warp"  # scripts/lib/platform.sh HELLO_PROGRAM
HELLO_RUNS = 3  # the cheapest of these runs counts: instructions and wall time only ever gain from the machine's load
# the startup a compile shares, warp.wasm loaded and started (notes/hello_startup.md): hello minus it is the compile
STARTUP = ["version"]
INSTALLS = os.path.expanduser("~/dev/bin/warp-builds")  # each install of a main tip (after-push.sh): warp and its compilers
HELLO_SNAPSHOTS = os.path.join(REPO, "data/vitals/hello.jsonl")  # one line per measured install, kept as installs go
# measured by hand before the series began: (label, revision, 10^9 instructions, seconds)
HELLO_REFERENCES = [("1.4.1", "v1.4.1", 0.09, 0.01), ("1.5.1", "v1.5.1", 1.9, 0.22), ("1.6.1", "v1.6.1", 6.1, 0.47),
                    ("before hello-takes", "d152612c4d^1", 5.4, 0.53), ("hello-takes", "d152612c4d", 3.1, 0.22)]
# wap, the new compiler (pannous/wap): its lines by stage folder, its road to self-build and its tests, as trends
# a stage is a folder of wap/core/ or of wap/, by the name it has now (WAP_RENAMED); the files beside them are "other"
WAP_RENAMED = {"preload": "types"}
WAP_OTHER = "other"  # compiler.wap, nodes.wap and wap/core/*.wap
WAP_LINE_LIMIT = 300  # a wap file past it is split (wap's long-file guardrail)
# a layer is a folder of wap/ (core, kernel, types, extensions …): its nodes (WAP_SIZE, comments count nothing) beside its
# code lines and its comment or blank lines, so a growth of comments shows apart (user 2026-11-21)
WAP_COMMENT_STARTS = (b"#", b"//")
WAP_SIZE = "scripts/wap-size.warp"  # wap's own measure: each file's nodes, Meta excluded
WAP_SNAPSHOTS = os.path.join(REPO, "data/vitals/wap.jsonl")  # a line per kind: "census" (by the detached job), "hello" and "size" (when wap/ changed), "tests"
# wap's own repository (pannous/wap) since it left warp at WAP_CUT, warp's last commit holding wap/: wap's history is
# warp's up to the cut, the checkout's after it
WAP_CHECKOUT = "/Users/me/dev/angles/wap"
WAP_CUT = "ffcad762e0"
# wap's tip, extracted whole to run it: one folder per job, by its real path, as a module's WASI follows no link (data/
# is one, to a scratch volume)
WAP_TREE = os.path.join(os.path.realpath(os.path.join(REPO, "data")), "vitals/wap-tree-%s")
WAP_CENSUS = "scripts/wap-census.warp"  # run on each wap/*.wap file
# what measures wap: the native wap (WAP_NATIVE) running the tip's measures, which use the tip's own compiler parts, so
# vitals needs no warp for wap; each snapshot says which ran it: old warp ran wap's source before 2026-11-20, then wap1
# (the tip compiled by wap's seed, run by warp) until 2026-11-22
WAP_MEASURED_BY = "native wap"
WAP_CENSUS_PID = os.path.join(REPO, "data/vitals/wap-census.pid")
WAP_CENSUS_LOG = os.path.join(REPO, "data/vitals/wap-census.log")
WAP_CENSUS_JOBS = 4  # census runs at once, each in a machine-wide job slot (scripts/job-slot.sh, niced): ~25 s each
JOB_SLOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "job-slot.sh")
WAP_CENSUS_FILE_SECONDS = 600  # one file's census; a file past it counts as none and says so
WAP_BLOCKERS = 8  # most frequent refusals kept per census
WAP_HELLO_SOURCE = 'print "Hello"\n'
# hello by the native wap as people run it (the install beside the binary, no warp in between): its first run after a
# change apart, then the cheapest of HELLO_RUNS warm runs
WAP_NATIVE = os.path.expanduser("~/dev/bin/wap")
# the warm state wap/compiler.wap embeds, a built file (wap's scripts/lib/wap-beside.sh WAP_WARM): empty in an extracted
# tip, as test/wap/compile.sh starts it, so the compiler's source reads its sources instead
WAP_WARM = "wap/warm.packed"
# the library compiled once that wap/compiler.wap embeds (wap's scripts/lib/wap-beside.sh WAP_LIBRARY_MODULE), a built
# file: empty in an extracted tip, as test/wap/compile.sh starts it, so the compiler's source compiles without one
WAP_LIBRARY_MODULE = "wap/types.wasm"
WAP_NATIVE_KEPT = os.path.expanduser("~/dev/bin/wap-seed/program-*.cwasm")  # the machine code it keeps of each program
# the run of all wap's tests, which only the cloud routine makes (user), its lines and summary last committed to main
WAP_TESTS_RESULTS = "test/results.txt"
WAP_TESTS_LINE = re.compile(r"(\d+) ✓, (\d+) ✗")
# how much of the library and the samples the native wap takes, wap's scripts/coverage.sh, run hourly by cron
WAP_COVERAGE = os.path.join(WAP_CHECKOUT, "data/wap-tests/coverage.txt")
WAP_COVERAGE_LINE = re.compile(r"^coverage: lib modules (\d+)/(\d+), samples (\d+)/(\d+) at (\w+) (.+)$", re.MULTILINE)
CENSUS_LINE = re.compile(r"(\d+) statements, (\d+) lowered, (\d+) bound(?:, typed: (\w+), emitted: (\w+))?")  # a preload file has no typed/emitted: it is typed by function
CENSUS_TOTAL = re.compile(r"(\d+) of (\d+) files typed whole, (\d+) of (\d+) functions typed")
REFUSAL_POSITION = re.compile(r"^\S+ line \d+:\d+: ")  # the file and place a refusal names, which differ per use


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


def git(*args, repository=REPO):
    # errors="replace": a results file may hold a test's raw bytes (bd625bdbdb's has a lone 0x83)
    return subprocess.run(["git", "-C", repository, *args], capture_output=True, text=True, errors="replace", check=True).stdout


# main read once: a fetch during the run must not mix two tips (KeyError in PARENTS)
TIP = git("rev-parse", "origin/main").strip()
WAP_TIP = git("rev-parse", "origin/main", repository=WAP_CHECKOUT).strip()


def file_contents(commit, classify=component, repository=REPO):
    """(path, content) of each file of <commit> that <classify> names a component of"""
    wanted = [f for f in git("ls-tree", "-r", "--name-only", commit, repository=repository).splitlines() if classify(f)]
    blob = subprocess.run(["git", "-C", repository, "cat-file", "--batch"], capture_output=True, check=True,
                          input="".join(f"{commit}:{f}\n" for f in wanted).encode()).stdout
    position = 0
    for f in wanted:
        header_end = blob.index(b"\n", position)
        size = int(blob[position:header_end].split()[-1])
        yield f, blob[header_end + 1:header_end + 1 + size]
        position = header_end + 2 + size


def line_count(content):
    """as git's numstat counts them: a last line without its newline is a line"""
    return content.count(b"\n") + (not content.endswith(b"\n") and bool(content))


def file_lines(commit, classify=component, repository=REPO):
    """(path, lines) of each file of <commit> that <classify> names a component of"""
    return ((f, line_count(content)) for f, content in file_contents(commit, classify, repository))


def lines_at(commit, classify=component):
    totals = collections.Counter()
    for f, lines in file_lines(commit, classify):
        totals[classify(f)] += lines
    return totals


def read_snapshots(path):
    if not os.path.exists(path):
        return []
    with open(path) as kept:
        return [json.loads(line) for line in kept if line.strip()]


def append_snapshot(path, snapshot):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "a") as kept:
        kept.write(json.dumps(snapshot) + "\n")


def commit_date(revision, repository=REPO):
    return git("log", "-1", "--format=%cd", "--date=iso-strict", revision, repository=repository).strip()[:16].replace("T", " ")


# wap/'s tree at wap's tip: a census or hello is due when it changed
def wap_tree():
    return git("rev-parse", f"{WAP_TIP}:wap", repository=WAP_CHECKOUT).strip()[:12]


# each commit of main that changed wap/, oldest first, [repository, sha, date]: warp's up to the cut, then wap's own
def wap_commits():
    return [[repository, *line.split()] for repository, tip in ((REPO, WAP_CUT), (WAP_CHECKOUT, WAP_TIP))
            for line in git("log", "--first-parent", "--reverse", "--format=%H %ad", "--date=iso-strict", tip, "--", "wap",
                            repository=repository).splitlines()]


def wap_sizes():
    """wap's lines at each commit of main that changed wap/ (warp's, then wap's own repository's): [date, commit, total,
    lines per stage, core lines then, files over WAP_LINE_LIMIT then, [code lines, comment or blank lines] per layer] and
    the files over the limit at the tip, [path, lines]"""
    classify = lambda path: path.endswith(".wap") and path.startswith("wap/") and wap_stage(path)
    series, over = [], []
    for repository, sha, date in wap_commits():
        stages, layers, files = collections.Counter(), {}, []
        for path, content in file_contents(sha, classify, repository):
            lines, commented, layer = line_count(content), comment_or_blank_lines(content), layers.setdefault(wap_layer(path), [0, 0])
            files.append((path, lines))
            stages[classify(path)] += lines
            layer[0] += lines - commented
            layer[1] += commented
        over = sorted(([path, lines] for path, lines in files if lines > WAP_LINE_LIMIT), key=lambda file: -file[1])
        date = date[:16].replace("T", " ")
        core = next((sum(row[3][:-1]) for row in reversed(rows) if row[0] <= date), 0)
        series.append([date, sha[:10], sum(stages.values()), dict(stages), core, len(over), layers])
    return series, over


def wap_folder(within):
    """the folder a path within wap/ starts with, by its name now, or WAP_OTHER for a file beside the folders"""
    folder, _, rest = within.partition("/")
    return WAP_RENAMED.get(folder, folder) if rest else WAP_OTHER


def wap_stage(path):
    """the stage folder of a wap/ file: a folder of wap/core/ or of wap/"""
    return wap_folder(path.removeprefix("wap/").removeprefix("core/"))


def wap_layer(path):
    """the layer of a wap/ file: its folder of wap/"""
    return wap_folder(path.removeprefix("wap/"))


def comment_or_blank_lines(content):
    return sum(1 for line in content.splitlines() if not line.strip() or line.lstrip().startswith(WAP_COMMENT_STARTS))


def wap_folders(sizes, field, weight=lambda lines: lines):
    """the folders a field of the sizes names (stages, layers), the largest at the tip first, "other" last"""
    named = {folder for row in sizes for folder in row[field]} - {WAP_OTHER}
    return sorted(named, key=lambda folder: -weight(sizes[-1][field][folder]) if folder in sizes[-1][field] else 0) + [WAP_OTHER]


def wap_extracted(kind):
    """wap's tip (its own repository), extracted whole and fresh into the folder of <kind> (the census job and the hello
    measure run apart, each in its own); its data/ is kept"""
    folder = WAP_TREE % kind
    os.makedirs(folder, exist_ok=True)
    for name in os.listdir(folder):
        if name != "data":
            path = os.path.join(folder, name)
            shutil.rmtree(path) if os.path.isdir(path) and not os.path.islink(path) else os.remove(path)
    subprocess.run(f"git -C {WAP_CHECKOUT} archive {WAP_TIP} | tar -x -C {folder}", shell=True, check=True)
    for built in (WAP_WARM, WAP_LIBRARY_MODULE):
        open(os.path.join(folder, built), "w").close()
    return folder


def wap_census(folder, files):
    """The census (wap's test/wap/ratchet_census.sh) of each of <files> in <folder>, each in a job slot, WAP_CENSUS_JOBS at a time, summed:
    statements, lowered, bound, files typed whole and emitted, functions typed, the files that timed out and those without
    a result, and the WAP_BLOCKERS most frequent refusals [stage, message, count]. The native wap runs the census from the
    folder, where it reads wap/ and notes/; its first file alone, which compiles the census once for the rest. A census
    of no file is no measurement: the job ends with the error and records nothing."""
    def run(file):
        try:
            ran = subprocess.run([JOB_SLOT, WAP_NATIVE, WAP_CENSUS, file], capture_output=True, text=True, cwd=folder,
                                 timeout=WAP_CENSUS_FILE_SECONDS)
            return ran.stdout + (f"\nexit {ran.returncode}: {ran.stderr.strip()}" if ran.returncode else "")
        except subprocess.TimeoutExpired:
            return "timeout\t"
    with concurrent.futures.ThreadPoolExecutor(WAP_CENSUS_JOBS) as pool:
        outputs = [run(files[0]), *pool.map(run, files[1:])]
    sums, blockers, timed_out = collections.Counter(files=len(files)), collections.Counter(), []
    for file, output in zip(files, outputs):
        for line in output.splitlines():
            stage, _, message = line.partition("\t")
            if stage == "ok" and (match := CENSUS_LINE.search(message)):
                statements, lowered, bound, _, emitted = match.groups()
                sums.update(statements=int(statements), lowered=int(lowered), bound=int(bound), emitted=emitted == "yes")
            elif stage == "total" and (match := CENSUS_TOTAL.search(message)):
                sums.update(typed=int(match[1]), functions=int(match[4]), functions_typed=int(match[3]))
            elif stage == "timeout":
                timed_out.append(file)
            elif stage in ("read", "declare", "lower", "bind", "type", "emit"):
                blockers[stage, REFUSAL_POSITION.sub("", message)] += 1
        if "ok\t" not in output and file not in timed_out:
            sums.update(missing=1)
            print(f"vitals: wap census of {file} gave no result: {output.strip()[-200:]}", file=sys.stderr)
    if sums["missing"] + len(timed_out) == len(files):
        sys.exit(f"vitals: the census measured none of the {len(files)} files, so it records nothing")
    return {**sums, "by": WAP_MEASURED_BY, "timed_out": timed_out, "blockers": [[stage, message, count] for (stage, message), count in blockers.most_common(WAP_BLOCKERS)]}


def census_job_running():
    try:
        with open(WAP_CENSUS_PID) as pid:
            os.kill(int(pid.read()), 0)
        return True
    except (OSError, ValueError):
        return False


def wap_census_job():
    """`vitals.py --census`, started detached by the hourly run: the census of main's tip, its snapshot appended when it ends;
    then the nodes of the commits not measured yet"""
    try:
        folder = wap_extracted("census")
        files = sorted(f for f in git("ls-tree", "-r", "--name-only", WAP_TIP, "wap", repository=WAP_CHECKOUT).splitlines() if f.endswith(".wap"))
        census = wap_census(folder, files)
        append_snapshot(WAP_SNAPSHOTS, {"kind": "census", "time": datetime.datetime.now().strftime("%Y-%m-%d %H:%M"),
                                         "commit": WAP_TIP[:10], "tree": wap_tree(), **census})
        wap_nodes_history(folder)
    finally:
        os.remove(WAP_CENSUS_PID)


def census_due(censuses, tree):
    """whether to start the census job: wap/ changed since the newest census and none runs (a census takes ~1 min)"""
    return not (censuses and censuses[-1]["tree"] == tree or census_job_running())


def wap_costs(folder):
    """What wap costs by the native wap (WAP_NATIVE), [10^9 instructions, seconds], None where a run failed: hello, its
    first run after a change and its cheapest warm run; and wap compiling its own compiler (milestone 2's self-compile)."""
    source = os.path.join(folder, "hello.wap")
    with open(source, "w") as hello:
        hello.write(WAP_HELLO_SOURCE)
    builds = [self_compiled(folder, source) for _ in range(HELLO_RUNS)]
    first = cheapest_cost(WAP_NATIVE, [source], "Hello", 1)
    warm = first and cheapest_cost(WAP_NATIVE, [source], "Hello")
    if None in (*builds, first, warm):
        return None
    costed = lambda cost: [round(cost[0], 3), round(cost[1], 3)]
    build = [min(b[0] for b in builds), min(b[1] for b in builds)]
    return {"by": WAP_MEASURED_BY, "native_build": costed(build), "native_hello_first": costed(first), "native_hello": costed(warm)}


def self_compiled(folder, hello):
    """One compile of wap's compiler by the native wap, [10^9 instructions, seconds]; None, said, unless the compiler it
    made compiles <hello> to a module that prints Hello. A copy made unique, so that no machine code kept from an earlier
    run stands in for the compile (the compiled compiler then runs without arguments and refuses: that run is part of
    the count, its status not). The copy, the machine code and hello's module are removed after."""
    copy, module = os.path.join(folder, "wap/compiler-measured.wap"), os.path.join(folder, "hello-measured.wasm")
    with open(copy, "w") as measured:
        measured.write(open(os.path.join(folder, "wap/compiler.wap")).read() + f"# measured {datetime.datetime.now().isoformat()}\n")
    before = set(glob.glob(WAP_NATIVE_KEPT))
    cost = cheapest_cost(WAP_NATIVE, [copy], "", 1)
    built = subprocess.run([WAP_NATIVE, copy, hello, module], capture_output=True, text=True, cwd=folder)
    said = subprocess.run([WAP_NATIVE, module], capture_output=True, text=True, cwd=folder) if os.path.exists(module) else built
    for path in [copy, *(set(glob.glob(WAP_NATIVE_KEPT)) - before), *glob.glob(module)]:
        os.remove(path)
    if "Hello" not in said.stdout:
        print(f"vitals: the compiler the native wap compiled does not compile hello: {(said.stdout + said.stderr).strip()[-300:]}", file=sys.stderr)
        return None
    return cost


def wap_nodes_in(folder, measured):
    """the nodes per layer ({"nodes": …}) of each wap/*.wap file of <measured>, by the native wap running the tip's
    WAP_SIZE (in <folder>; it reads old sources too); None when it fails, its error printed"""
    files = sorted(os.path.relpath(path, measured) for path in glob.glob(os.path.join(measured, "wap/**/*.wap"), recursive=True))
    run = subprocess.run([WAP_NATIVE, os.path.join(folder, WAP_SIZE), *files], capture_output=True, text=True, cwd=measured)
    if run.returncode != 0:
        print(f"vitals: {WAP_SIZE} failed in {measured}: {(run.stdout + run.stderr).strip()[-300:]}", file=sys.stderr)
        return None
    nodes = collections.Counter()
    for path, count in (line.split("\t") for line in run.stdout.splitlines() if line.startswith("wap/")):
        nodes[wap_layer(path)] += int(count)
    return {"nodes": dict(nodes)}


def wap_nodes(folder):
    """wap's nodes per layer at the tip extracted in <folder>"""
    return wap_nodes_in(folder, folder)


def wap_nodes_history(folder):
    """the nodes per layer of each commit that changed wap/ and has no size snapshot, by the tip's measure, each appended
    as it is measured: once, so node and line trends cover the same span. A commit whose files the measure fails on is
    kept with nodes null (its error printed), never measured again."""
    measured, past = {s["commit"] for s in read_snapshots(WAP_SNAPSHOTS) if s["kind"] == "size"}, os.path.join(folder, "data/past")
    for repository, sha, date in wap_commits():
        if sha[:10] in measured:
            continue
        shutil.rmtree(past, ignore_errors=True)
        os.makedirs(past)
        subprocess.run(f"git -C {repository} archive {sha} wap | tar -x -C {past}", shell=True, check=True)
        append_snapshot(WAP_SNAPSHOTS, {"kind": "size", "time": date[:16].replace("T", " "), "commit": sha[:10],
                                        "nodes": (wap_nodes_in(folder, past) or {}).get("nodes")})


def wap_road():
    """wap's snapshots: the newest census (taken by the detached census job, started here when due), hello by wap and its
    nodes per layer measured here when wap/ changed since their last snapshot, and the summary of each run of all wap's tests."""
    snapshots, tree = read_snapshots(WAP_SNAPSHOTS), wap_tree()
    if census_due([s for s in snapshots if s["kind"] == "census"], tree):
        with open(WAP_CENSUS_LOG, "w") as log:
            job = subprocess.Popen(["nice", "-n", "19", sys.executable, os.path.abspath(__file__), "--census"], stdout=log,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        with open(WAP_CENSUS_PID, "w") as pid:
            pid.write(str(job.pid))
    measures = {"hello": wap_costs, "size": wap_nodes}  # each measured in one extracted tip when wap/ changed
    due = [kind for kind in measures if not any(s["kind"] == kind and s.get("tree") == tree for s in snapshots)]
    folder = due and wap_extracted("hello")
    for kind in due:
        if measured := measures[kind](folder):
            snapshots.append({"kind": kind, "time": commit_date(WAP_TIP, WAP_CHECKOUT), "commit": WAP_TIP[:10], "tree": tree, **measured})
            append_snapshot(WAP_SNAPSHOTS, snapshots[-1])
    for snapshot in (wap_tests_run(), wap_coverage()):
        if snapshot and not any(s["kind"] == snapshot["kind"] and s["time"] == snapshot["time"] for s in snapshots):
            snapshots.append(snapshot)
            append_snapshot(WAP_SNAPSHOTS, snapshot)
    return snapshots


def wap_tests_run():
    """the newest run of all wap's tests, from main's WAP_TESTS_RESULTS: its counts, at the commit that wrote them;
    None, said, when main holds no summary"""
    commit = git("log", "-1", "--format=%h", "--abbrev=10", WAP_TIP, "--", WAP_TESTS_RESULTS, repository=WAP_CHECKOUT).strip()
    lines = git("show", f"{WAP_TIP}:{WAP_TESTS_RESULTS}", repository=WAP_CHECKOUT).splitlines() if commit else []
    if not (lines and (match := WAP_TESTS_LINE.search(lines[-1]))):
        print(f"vitals: wap's main holds no summary of a run of all tests in {WAP_TESTS_RESULTS}: the wap tests panel keeps its last count", file=sys.stderr)
        return None
    return {"kind": "tests", "time": commit_date(commit, WAP_CHECKOUT), "commit": commit, "passed": int(match[1]), "failed": int(match[2])}


def wap_coverage():
    """the newest coverage of the library and the samples (WAP_COVERAGE); None, said, when there is none"""
    match = os.path.exists(WAP_COVERAGE) and WAP_COVERAGE_LINE.search(open(WAP_COVERAGE).read())
    if not match:
        print(f"vitals: no coverage summary in {WAP_COVERAGE} (wap's scripts/coverage.sh): the coverage panels keep their last counts", file=sys.stderr)
        return None
    modules, module_count, samples, sample_count = map(int, match.groups()[:4])
    return {"kind": "coverage", "time": match[6], "commit": match[5], "modules": modules, "module_count": module_count, "samples": samples, "sample_count": sample_count}


def wap_track():
    sizes, over = wap_sizes()
    return {"sizes": sizes, "over": over, "limit": WAP_LINE_LIMIT, "stages": wap_folders(sizes, 3), "layers": wap_folders(sizes, 6, sum),
            "snapshots": wap_road(), "own_commits": git("log", "--format=%h", "--abbrev=10", WAP_TIP, repository=WAP_CHECKOUT).split()}


if CENSUS_JOB:
    wap_census_job()
    sys.exit()


def group_sums(counter):
    return [sum(counter.get(c, 0) for c in names) for _, names in CORE_GROUPS]


def first_parent_changes(shas):
    """Each commit's change against its first parent, per component, from one git log of them all."""
    changes, sha = {}, None
    logged = subprocess.run(["git", "-C", REPO, "log", "--no-walk=unsorted", "--stdin", "--numstat", "--no-renames",
                             "--diff-merges=first-parent", "--format=@%H"], input="\n".join(shas), capture_output=True,
                            text=True, errors="replace", check=True).stdout
    for line in logged.splitlines():
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


def changes_known(shas):
    """The whole changes of the commits, those not kept yet read by one git log for all of them (one call a commit took
    35 s of 105, card vitals-py-slow)"""
    missing = [sha for sha in shas if sha not in whole_changes]
    if missing:
        whole_changes.update(first_parent_changes(missing))


whole_changes = {}
if os.path.exists(CHANGES_CACHE):
    with open(CHANGES_CACHE) as kept:
        whole_changes = {sha: collections.Counter(change) for sha, change in json.load(kept).items()}


own_changes = {}
def own_change(sha):
    """What one commit changed itself, per component: a commit against its parent; a merge its whole change against its
    first parent less what the commits it brought in changed (their own changes), so only its resolution."""
    if sha not in own_changes:
        first_parent, *others = PARENTS[sha]
        brought = list(members(ANCESTORS[sha] & ~ANCESTORS[first_parent] & ~(1 << ORDER_INDEX[sha]))) if others else []
        changes_known([sha] + brought)
        change = whole_changes[sha].copy()
        for each in brought:
            change.subtract(own_change(each))
        own_changes[sha] = change
    return own_changes[sha]


ORDER, PARENTS, ANCESTORS = history_graph()
ORDER_INDEX = {sha: index for index, sha in enumerate(ORDER)}
# the commits that are no merges, as a bitset of ORDER's indices
SINGLE_PARENTED = sum(1 << index for index, sha in enumerate(ORDER) if len(PARENTS[sha]) <= 1)


# One row per commit by what it changed itself: measured against its first parent, a merge is charged with everything
# main gained since its branch forked when a worker pushes "Merge origin/main into <branch>" (the branch is its first
# parent), so the atomic commits carry their own lines and a merge only its resolution (merges resolving nothing are
# left out). Every commit since main's first-parent state before SINCE counts once, so the running totals end at the
# tip's (checked below).
base = git("rev-list", "-1", "--first-parent", "--before=" + SINCE, TIP).strip()
changes_known(git("rev-list", f"{base}..{TIP}").split())
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
    for line in git("log", "--reverse", "--format=%H %ad %s", "--date=iso-strict", "--since=" + SINCE, TIP, "--", path).splitlines():
        sha, date, subject = line.split(" ", 2)
        counts = pass_count(git("show", f"{sha}:{path}"))
        if counts and counts[1]:
            minutes = RUN_MINUTES.search(subject)
            points.append([date[:16].replace("T", " "), sha[:10], *counts, int(minutes[1]) if minutes else None])
    return points


def record_field(path, key):
    with open(path) as lines:
        return next((line.split() for line in lines if line.split()[1:2] == [key] or line.split()[:1] == [key]), None)


def landed_records(record, landed):
    """(folder, commit, build log) of each build record <record> in the store of a landed commit, the commit the record
    says it built (a build of uncommitted edits says `<sha>+dirty`, rebuild.sh built_commit, and is no landing's); the
    builds that rebake are no landing's cost"""
    for folder in glob.glob(os.path.join(STORE, "*", record)):
        try:
            commit = record_field(os.path.join(folder, "inputs.txt"), "commit")[1]
            with open(os.path.join(folder, "build.log")) as log:
                text = log.read()
        except (OSError, TypeError, IndexError):
            continue
        if commit in landed and "rebak" not in text:
            yield folder, commit, text


def step_seconds(folder):
    """{step: seconds since the build started, "end": its end} of a record's steps file (scripts/build-slot.sh), {} for
    a record from before it"""
    try:
        with open(os.path.join(folder, "steps")) as lines:
            return {name: int(seconds) for name, seconds in (line.split() for line in lines)}
    except (OSError, ValueError):
        return {}


def build_seconds(folder, text):
    """(core, libraries, everything): the wall seconds of rebuilding the compiler (the seed and optimize steps), of the
    runtime and modules steps, and of the whole build, None where the record can't tell: a build from before the steps
    file knows only its total, except a lib-only one (no seed step), whose total is the libraries"""
    total = float(record_field(os.path.join(folder, "time"), "real")[1])
    steps = step_seconds(folder)
    if "runtime" in steps and "seed" in steps:
        return steps["runtime"] - steps["seed"], total - steps["runtime"], total
    if "rebuild step seed" not in text:
        return None, total, None
    return None, None, total


def build_times(landed):
    """Each landing's [date, commit, core, libraries, everything] wall seconds of rebuilding warp.wasm, the leanest
    of the landing's records for each part"""
    leanest = {}
    for folder, commit, text in landed_records("warp.record", landed):
        try:
            parts = build_seconds(folder, text)
        except (OSError, TypeError, ValueError, IndexError):
            continue
        best = leanest.setdefault(commit, [None] * len(parts))
        leanest[commit] = [part if best[i] is None or (part is not None and part < best[i]) else best[i] for i, part in enumerate(parts)]
    return sorted([landed[commit], commit[:10], *(None if part is None else round(part, 1) for part in parts)] for commit, parts in leanest.items() if landed[commit] >= BUILD_TIMES_SINCE)


def build_history(record, by_seed, landed):
    """Each landing's instructions retired (in 10^12), peak memory and CPU seconds of building <record>, its leanest
    full build (a lib-only build reusing a stored compiler skipped: its log has no seed step; older ones say rebake), by the commit the record says it built: a build of uncommitted edits says
    `<sha>+dirty` (rebuild.sh built_commit) and is no landing's. Instructions, as CPU seconds swing with the machine's
    load and cores; <by_seed>: a seed's first build is left out, a compiler's cold first run reads ~3% more
    (notes/compiler_rebuilds.md)."""
    builds = []
    for folder, commit, text in landed_records(record, landed):
        try:
            seed = (record_field(os.path.join(folder, "inputs.txt"), "seed") or [None, None])[1]
            # only a seed build has steps; warp-self's log (warp.wasm compiling itself) has none, and the step test
            # dropped every self-compile record since b9af7cea67
            if by_seed and "rebuild step seed" not in text:
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
    """The pushes to main since SINCE, from GitHub's activity record of the branch, newest first: each one's local hour
    and the commits it brought (main's tip before and after it). git can't tell: a branch merged with main and pushed
    fast-forward puts main's own history on its second parent."""
    pushes, url = [], f"repos/{GITHUB_REPO}/activity?ref=refs/heads/main&activity_type=push&per_page=100"
    while url:
        response = subprocess.run(["gh", "api", "-i", url], capture_output=True, text=True, check=True).stdout
        head, body = response.split("\r\n\r\n" if "\r\n\r\n" in response else "\n\n", 1)
        events = [(datetime.datetime.fromisoformat(event["timestamp"].replace("Z", "+00:00")).astimezone()
                   .strftime("%Y-%m-%d %H"), event["before"], event["after"]) for event in json.loads(body)]
        pushes += [push for push in events if push[0] >= SINCE]
        following = re.search(r'<https://api\.github\.com/([^>]+)>; rel="next"', head)
        url = following.group(1) if following and events and min(push[0] for push in events) >= SINCE else None
    return pushes


def commits_brought(before, after):
    """How many commits of work a push brought to main: those reachable from its new tip and not from the old one, merges
    left out (a branch's "Merge origin/main into …" syncs and the landing merges were 338 of 792 in a day, 11-13); 0
    where main's history lacks either tip, a force push's lost one. Counted on the history graph in memory (one git
    rev-list a push took 42 s of 105, card vitals-py-slow)"""
    if before not in ANCESTORS or after not in ANCESTORS:
        return 0
    return (ANCESTORS[after] & ~ANCESTORS[before] & SINGLE_PARENTED).bit_count()


def hourly_pace():
    """Pushes to main and the commits they brought, per hour of the push, every hour since SINCE: both on one clock,
    so a commit made on a branch at 10 and landed at 16 counts at 16."""
    pushes_per_hour, commits_per_hour = collections.Counter(), collections.Counter()
    for hour, before, after in pushes_since():
        pushes_per_hour[hour] += 1
        commits_per_hour[hour] += commits_brought(before, after)
    first = datetime.datetime.strptime(min(pushes_per_hour), "%Y-%m-%d %H")
    hours = []
    while (label := first.strftime("%Y-%m-%d %H")) <= max(pushes_per_hour):
        hours.append([label, pushes_per_hour[label], commits_per_hour[label]])
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
    snapshots = read_snapshots(CARD_SNAPSHOTS)
    now = datetime.datetime.now()
    if not snapshots or now - datetime.datetime.fromisoformat(snapshots[-1]["time"]) > datetime.timedelta(hours=1):
        listing = subprocess.run(["todo", "list", "--json"], capture_output=True, text=True, cwd=REPO)
        if listing.returncode == 0:
            cards = json.loads(listing.stdout)
            counts = collections.Counter(card["column"] for card in cards)
            snapshots.append({"time": now.isoformat(timespec="minutes"), "columns": dict(counts),
                              "done": recently_done(cards)})
            append_snapshot(CARD_SNAPSHOTS, snapshots[-1])
        else:
            print("vitals: todo list failed, cards from the last snapshot: " + listing.stderr.strip()[-200:], file=sys.stderr)
    return snapshots


def cheapest_cost(warp, arguments, said, runs_count=HELLO_RUNS):
    """The cheapest of <runs_count> runs of `<warp> <arguments>`: 10^9 instructions retired and wall seconds, by
    /usr/bin/time -l; None when its output doesn't say `said`."""
    runs = []
    for _ in range(runs_count):
        run = subprocess.run(["/usr/bin/time", "-l", warp, *arguments], capture_output=True, text=True, cwd=REPO,
                             env={**os.environ, "WARP_WINDOW": "0"})
        instructions, seconds = re.search(r"(\d+)\s+instructions retired", run.stderr), re.search(r"([\d.]+) real", run.stderr)
        if said not in run.stdout or not instructions or not seconds:
            print(f"vitals: {warp} {' '.join(arguments)} failed: {(run.stdout + run.stderr).strip()[-200:]}", file=sys.stderr)
            return None
        runs.append((int(instructions.group(1)) / 1e9, float(seconds.group(1))))
    return min(r[0] for r in runs), min(r[1] for r in runs)


def hello_cost(warp):
    """hello compiled and run (10^9 instructions, seconds), and the compile alone: hello's instructions minus those of
    STARTUP, the same warp loading and starting warp.wasm (still holding hello's own run, ~0.01 G); None when hello
    doesn't say Hello."""
    hello, startup = cheapest_cost(warp, [HELLO], "Hello"), cheapest_cost(warp, STARTUP, "Warp")
    if not hello:
        return None
    return round(hello[0], 2), round(hello[1], 2), round(hello[0] - startup[0], 3) if startup else None


def hello_track():
    """hello's compile and run per main tip: each install in INSTALLS not measured yet is measured once (its
    compilers' commit, warp.wasm.inputs) and kept in HELLO_SNAPSHOTS; then HELLO_REFERENCES, measured by hand."""
    # a later line of a commit adds to its earlier one (the compile, measured since the series began)
    points = {}
    for point in read_snapshots(HELLO_SNAPSHOTS):
        points.setdefault(point["commit"], {}).update(point)
    measured = list(points.values())
    known = {point["commit"] for point in measured if "compile" in point}
    for folder in sorted(glob.glob(os.path.join(INSTALLS, "*"))):
        try:
            with open(os.path.join(folder, "warp.wasm.inputs")) as inputs:
                commit = re.search(r"commit=(\w+)", inputs.read()).group(1)
        except (OSError, AttributeError):
            continue
        if commit[:10] in known or not (cost := hello_cost(os.path.join(folder, "warp"))):
            continue
        known.add(commit[:10])
        point = points.setdefault(commit[:10], {"time": commit_date(commit), "commit": commit[:10]})
        if "instructions" not in point:
            point.update({"instructions": cost[0], "seconds": cost[1]})
            measured.append(point)
        point["compile"] = cost[2]
        append_snapshot(HELLO_SNAPSHOTS, point)
    series = sorted([p["time"], p["commit"], p["instructions"], p["seconds"], "", p.get("compile")] for p in measured)
    references = [[commit_date(revision), git("rev-parse", "--short=10", revision).strip(), instructions, seconds, label]
                  for label, revision, instructions, seconds in HELLO_REFERENCES]
    return {"series": series, "references": references}


landed = {line.split()[0]: line.split()[1][:16].replace("T", " ") for line in
          git("log", "--format=%H %ad", "--date=iso-strict", "--since=" + SINCE, TIP).splitlines()}
data = {"cards": card_counts(), "hello": hello_track(), "wap": wap_track(), "pace": hourly_pace(), "builds": [[name, record, build_history(record, by_seed, landed)] for name, record, by_seed in BUILDS], "build_times": build_times(landed), "tiers": [[name, path, tier_history(path), latest_flips(path)] for name, path in TIERS], "groups": [name for name, _ in CORE_GROUPS] + [EXTENSIONS], "base": base[:10], "rows": rows,
        "tip": git("log", "-1", "--format=%h%x09%s%x09%ad", "--date=format:%Y-%m-%d %H:%M", TIP).strip().split("\t"),
        "generated": datetime.datetime.now().strftime("%Y-%m-%d %H:%M")}
os.makedirs(os.path.dirname(OUT), exist_ok=True)
os.makedirs(os.path.dirname(CHANGES_CACHE), exist_ok=True)
with open(CHANGES_CACHE + ".new", "w") as kept:
    json.dump(whole_changes, kept, separators=(",", ":"))
os.replace(CHANGES_CACHE + ".new", CHANGES_CACHE)
with open(TEMPLATE) as template:
    filled = template.read().replace("__DATA__", json.dumps(data, separators=(",", ":")).replace("</", "<\\/"))
with open(OUT, "w") as page:
    page.write(filled)
for old, new in WAP_PAGE:
    assert filled.count(old) >= 1, f"the wap page's replacement no longer finds {old!r} in {TEMPLATE}"
    filled = filled.replace(old, new, 1)
with open(WAP_OUT, "w") as page:
    page.write(filled)
first, last = rows[0], rows[-1]
core = lambda row: sum(row[3][:-1])
latest = "; ".join(f"{name} {points[-1][2]}/{points[-1][3]}" for name, _, points, _ in data["tiers"] if points)
builds = "; ".join(f"{record} {points[-1][2]} T instructions ({points[-1][4]} s CPU)" for _, record, points in data["builds"] if points)
print(f"{latest}; {builds}; core {core(first)} -> {core(last)}, extensions {first[3][-1]} -> {last[3][-1]}, "
      f"lib {first[6]} -> {last[6]}, test {first[7]} -> {last[7]} over {len(rows)} commits: {OUT}")
