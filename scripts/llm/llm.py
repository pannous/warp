"""Shared pieces of the LLM tools (docs_check.py, corpus.py): the API key, Message Batches with a cost tally and a
hard budget, running warp programs, judging their results and turning confirmed problems into board cards.
Guide: notes/llm_tools.md"""
import concurrent.futures
import datetime
import fcntl
import json
import os
import re
import subprocess
import sys
import tempfile
import time

import anthropic
import json5

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
# the wiki is its own repo inside the user's main checkout, so a worktree has none: find the main checkout
MAIN_CHECKOUT = os.path.dirname(os.path.abspath(subprocess.run(
    ["git", "-C", ROOT, "rev-parse", "--git-common-dir"], capture_output=True, text=True).stdout.strip()))
DATA = os.path.join(ROOT, "data", "llm")
SCRATCH = os.path.join(ROOT, "scratch", "llm")
COST_FILE = os.path.join(DATA, "cost.json5")
BUDGET_DOLLARS = 150.0  # user, 2026-10-10: stop at $150 of the ~$200 credit
KEYS_FILE = os.path.expanduser("~/.keys")
KEY_NAME = "ANTHROPIC_KEY_WARP"

HAIKU = "claude-haiku-5-5"
SONNET = "claude-sonnet-5-5"
# $ per million tokens (input, output); batches cost half, cache reads 0.1× and cache writes 1.25× of input
PRICES = {HAIKU: (0.10, 0.50), SONNET: (2.00, 10.00)}
BATCH_DISCOUNT = 0.5
CACHE_READ_FACTOR = 0.1
CACHE_WRITE_FACTOR = 1.25
BATCH_POLL_SECONDS = 30

WARP = os.environ.get("WARP", "/Users/me/dev/bin/warp")  # main's debug build, refreshed per pushed batch
WARP_FLAGS = ["--sandbox", "--no-ask"]  # no shell, C or file runtime; never wait for "got it?"
RUN_TIMEOUT_SECONDS = 20
PARALLEL_RUNS = 4
OUTPUT_LIMIT = 3000  # characters of a program's output shown to the model
TODO = os.path.expanduser("~/dev/bin/todo")


def api_key():
    for line in open(KEYS_FILE, encoding="utf-8"):
        match = re.match(rf"\s*(?:export\s+)?{KEY_NAME}=['\"]?([^'\"\s]+)", line)
        if match:
            return match.group(1)
    sys.exit(f"{KEY_NAME} not found in {KEYS_FILE}")


def client():
    return anthropic.Anthropic(api_key=api_key())


def today():
    return datetime.date.today().isoformat()


def write_json5(path, value, comment=""):
    """JSON is valid JSON5; the comment line says what the file is"""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as file:
        file.write(f"// {comment}\n" if comment else "")
        json.dump(value, file, indent=1, ensure_ascii=False)


def read_json5(path, default):
    return json5.load(open(path, encoding="utf-8")) if os.path.exists(path) else default


# ---- cost tally ----

def dollars(model, usage, batch=True):
    input_price, output_price = PRICES[model]
    read = getattr(usage, "cache_read_input_tokens", 0) or 0
    written = getattr(usage, "cache_creation_input_tokens", 0) or 0
    input_cost = (usage.input_tokens + read * CACHE_READ_FACTOR + written * CACHE_WRITE_FACTOR) * input_price
    total = (input_cost + usage.output_tokens * output_price) / 1e6
    return total * (BATCH_DISCOUNT if batch else 1)


def tally():
    return read_json5(COST_FILE, {"total_dollars": 0.0, "budget_dollars": BUDGET_DOLLARS, "by_tool": {}, "runs": []})


def record_cost(tool, stage, model, cost, requests):
    os.makedirs(DATA, exist_ok=True)
    with open(COST_FILE + ".lock", "w") as lock:  # two tools running at once must not lose an update
        fcntl.flock(lock, fcntl.LOCK_EX)
        add_cost(tool, stage, model, cost, requests)


def add_cost(tool, stage, model, cost, requests):
    costs = tally()
    costs["total_dollars"] = round(costs["total_dollars"] + cost, 4)
    costs["by_tool"][tool] = round(costs["by_tool"].get(tool, 0) + cost, 4)
    costs["runs"].append({"date": today(), "tool": tool, "stage": stage, "model": model, "requests": requests,
                          "dollars": round(cost, 4)})
    write_json5(COST_FILE, costs, "API spend of scripts/llm (usage from the API responses); stops at budget_dollars")
    print(f"  cost {stage}: ${cost:.3f}, total ${costs['total_dollars']:.2f} of ${BUDGET_DOLLARS:.0f}")


def check_budget(estimate):
    spent = tally()["total_dollars"]
    if spent + estimate > BUDGET_DOLLARS:
        sys.exit(f"budget: ${spent:.2f} spent + ~${estimate:.2f} would pass ${BUDGET_DOLLARS:.0f}; stopping")


def estimate(model, requests, input_tokens_each, output_tokens_each):
    input_price, output_price = PRICES[model]
    return requests * (input_tokens_each * input_price + output_tokens_each * output_price) / 1e6 * BATCH_DISCOUNT


# ---- Message Batches ----

def request(custom_id, model, system, prompt, schema, max_tokens=4000, effort="low"):
    """One batch request whose answer is JSON matching schema; a long system text is cached across the batch"""
    return {"custom_id": custom_id, "params": {
        "model": model, "max_tokens": max_tokens,
        "system": [{"type": "text", "text": system, "cache_control": {"type": "ephemeral"}}],
        "messages": [{"role": "user", "content": prompt}],
        "output_config": {"effort": effort, "format": {"type": "json_schema", "schema": schema}}}}


def run_batch(tool, stage, requests, input_tokens_each, output_tokens_each):
    """Send requests as one Message Batch (half price), wait, tally the cost; custom_id → parsed JSON answer.
    A batch id is kept in data/llm/<tool>/<stage>.batch until its results are in, so a rerun after an interruption
    picks the running batch up instead of paying twice."""
    if not requests:
        return {}
    model = requests[0]["params"]["model"]
    check_budget(estimate(model, len(requests), input_tokens_each, output_tokens_each))
    api = client()
    # custom ids allow only [a-zA-Z0-9_-]{1,64}: send positions, answer by the caller's ids
    ids = [entry["custom_id"] for entry in requests]
    requests = [dict(entry, custom_id=f"r{position}") for position, entry in enumerate(requests)]
    pending = os.path.join(DATA, tool, f"{stage}.batch")
    if os.path.exists(pending):
        batch_id = open(pending).read().strip()
        print(f"  {stage}: resuming batch {batch_id}")
    else:
        batch_id = api.messages.batches.create(requests=requests).id
        os.makedirs(os.path.dirname(pending), exist_ok=True)
        open(pending, "w").write(batch_id)
        print(f"  {stage}: batch {batch_id} with {len(requests)} requests to {model}")
    while (batch := api.messages.batches.retrieve(batch_id)).processing_status != "ended":
        counts = batch.request_counts
        print(f"  {stage}: {counts.processing} processing, {counts.succeeded} done", flush=True)
        time.sleep(BATCH_POLL_SECONDS)
    answers, cost, failures = {}, 0.0, []
    for result in api.messages.batches.results(batch_id):
        custom_id = ids[int(result.custom_id[1:])]
        if result.result.type != "succeeded":
            failures.append(f"{custom_id}: {result.result.type}")
            continue
        message = result.result.message
        cost += dollars(message.model if message.model in PRICES else model, message.usage)
        text = "".join(block.text for block in message.content if block.type == "text")
        try:
            answers[custom_id] = json.loads(text)
        except json.JSONDecodeError:
            failures.append(f"{custom_id}: no JSON ({message.stop_reason}): {text[:200]}")
    record_cost(tool, stage, model, cost, len(requests))
    os.remove(pending)
    if failures:
        print(f"  {stage}: {len(failures)} requests failed, first: {failures[:3]}", file=sys.stderr)
    return answers


# ---- running warp ----

def run_warp(code):
    """Run a program the way a user would (WARP_NO_WINDOW keeps paint() off the screen); its whole output"""
    os.makedirs(SCRATCH, exist_ok=True)
    handle, path = tempfile.mkstemp(".warp", dir=SCRATCH)  # one file per run: parallel runs may share code
    with os.fdopen(handle, "w", encoding="utf-8") as file:
        file.write(code)
    try:
        done = subprocess.run([WARP, *WARP_FLAGS, "run", path], capture_output=True, text=True, errors="replace", cwd=SCRATCH,
                              stdin=subprocess.DEVNULL, timeout=RUN_TIMEOUT_SECONDS,
                              env=dict(os.environ, WARP_NO_WINDOW="1"))
        output = (done.stdout + done.stderr).strip()
    except subprocess.TimeoutExpired:
        output = f"TIMEOUT after {RUN_TIMEOUT_SECONDS} s"
    finally:
        os.remove(path)
    return output[:OUTPUT_LIMIT]


def run_all(codes):
    with concurrent.futures.ThreadPoolExecutor(PARALLEL_RUNS) as pool:
        return list(pool.map(run_warp, codes))


def warp_version():
    done = subprocess.run([WARP, "--version"], capture_output=True, text=True)
    return (done.stdout + done.stderr).strip().replace("\n", " · ")


# ---- judging: Haiku triages every item, Sonnet decides the unclear and the failing ones ----

VERDICTS = ["ok", "warp_bug", "missing_feature", "doc_wrong", "program_wrong", "not_runnable", "unclear"]
JUDGE_SCHEMA = {"type": "object", "additionalProperties": False, "required": ["verdict", "reason"], "properties": {
    "verdict": {"type": "string", "enum": VERDICTS}, "reason": {"type": "string"}}}
VERDICT_MEANINGS = """Verdicts:
- ok: the actual output is what was expected (same value; formatting differences like quotes, spacing, yes/true,
  trailing zeros, or extra printed lines that the expectation does not contradict are fine)
- warp_bug: warp produced a wrong value, crashed, panicked, gave an internal error or an error a correct program
  should not get
- missing_feature: the code uses a reasonable feature the documentation promises or a user would expect, and warp
  says it is unknown, undefined or not supported
- doc_wrong: warp behaves sensibly and the documentation's claim or code is outdated or wrong
- program_wrong: the code itself is mistaken (a typo, a wrong algorithm, syntax the documentation never shows)
- not_runnable: not meant to run as a whole program: pseudo-code, another language, a fragment, grammar notation,
  a deliberately wrong example, or it needs a window, a server, network, files or a shell (programs run with
  --sandbox, which refuses shell, C and file access, and a 20 s timeout ends servers)
- unclear: you cannot tell from what is shown"""


def judge(tool, items, describe, system):
    """items: dicts with an "id"; describe(item) → the prompt text. Returns {id: {verdict, reason, model}}"""
    triage = run_batch(tool, "triage", [request(item["id"], HAIKU, system, describe(item), JUDGE_SCHEMA, 2000)
                                        for item in items], 3000, 600)
    verdicts = {id: dict(answer, model=HAIKU) for id, answer in triage.items()}
    doubtful = [item for item in items if verdicts.get(item["id"], {}).get("verdict", "unclear")
                not in ("ok", "not_runnable")]
    second = run_batch(tool, "review", [request(item["id"], SONNET, system, describe(item) + "\n\nA first reviewer "
                       f"said: {json.dumps(verdicts.get(item['id']))}. Decide yourself.", JUDGE_SCHEMA, 6000, "medium")
                       for item in doubtful], 4000, 2000)
    verdicts.update({id: dict(answer, model=SONNET) for id, answer in second.items()})
    return verdicts


# ---- board cards ----

PROBLEM_VERDICTS = ("warp_bug", "missing_feature", "doc_wrong")
CARDS_SCHEMA = {"type": "object", "additionalProperties": False, "required": ["new_cards", "known"], "properties": {
    "new_cards": {"type": "array", "items": {"type": "object", "additionalProperties": False,
                  "required": ["key", "title", "body", "finding_ids"], "properties": {
                      "key": {"type": "string"}, "title": {"type": "string"}, "body": {"type": "string"},
                      "finding_ids": {"type": "array", "items": {"type": "string"}}}}},
    "known": {"type": "array", "items": {"type": "object", "additionalProperties": False,
              "required": ["finding_id", "card_key"], "properties": {
                  "finding_id": {"type": "string"}, "card_key": {"type": "string"}}}}}}
CARDS_SYSTEM = """You maintain the to-do board of warp, a programming language. You get the board's current cards
(column, key, title) and new findings from an automatic check. Group findings with the same cause into one card.
A finding that an open card (Now, Next, Soon, Later) already covers goes in "known" with that card's key; a finding
whose cause a Done card claims fixed is a regression and gets a new card naming that key. Each new card: a key of
one to three lowercase words joined by hyphens, a one-line title that starts with the key and a colon and states
the defect concretely with the smallest code and the wrong versus expected result, and a body listing every
finding id (file:line or task name) with its code and output. Write plainly, no markdown headings."""


def board():
    return todo("list")


def file_cards(tool, findings, dry_run, source):
    """findings: dicts with id, verdict, reason, code, output. Groups and dedupes them against the board with
    Sonnet, adds the new cards (column Next) unless dry_run; returns the plan"""
    if not findings:
        return {"new_cards": [], "known": []}
    prompt = "Board:\n" + board() + "\n\nFindings:\n" + json.dumps(findings, ensure_ascii=False, indent=1)
    plan = run_batch(tool, "cards", [request("cards", SONNET, CARDS_SYSTEM, prompt, CARDS_SCHEMA, 32000, "medium")],
                     60000, 12000).get("cards", {"new_cards": [], "known": []})
    add_cards(plan, dry_run, source)
    return plan


def add_cards(plan, dry_run, source):
    """Adds a plan's new cards to the board (column Next); dry_run only prints them"""
    for card in plan["new_cards"]:
        body = f"{card['body']}\n\nFound by scripts/llm/{source} on {today()} ({warp_version()})."
        if dry_run:
            print(f"  would add: {card['title']}")
        else:
            added = todo("add", card["title"], "Next", "-b", body)
            print(added)
            given_key = added.split()[0] if added else card["key"]
            if given_key != card["key"]:  # todo shortens keys to two words; the planned key says more
                todo("key", given_key, card["key"])


def todo(*arguments):
    return subprocess.run([TODO, *arguments], capture_output=True, text=True, timeout=120).stdout.strip()


def file_saved_cards(path, source):
    """--file-cards: add the cards of a reviewed dry run (its .json5 report) without asking the model again"""
    add_cards(read_json5(path, {})["cards"], False, source)
