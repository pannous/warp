#!/usr/bin/env python3
"""Program corpus: Sonnet writes small warp programs for Rosetta-Code-style tasks from the playground guides alone,
as a new user would; each runs through warp, Claude judges the result (Haiku triage, Sonnet for the doubtful ones)
and the failures that show a bug or a missing feature become board cards, grouped and deduplicated. Programs that
work are listed as candidates for samples/ or tests. Meant to be rerun weekly on the same task list.
Usage: scripts/llm/corpus.py [--dry-run] [--limit N] [--new-tasks]
       scripts/llm/corpus.py --file-cards data/llm/corpus/<date>.json5   (cards of a reviewed dry run)
Tasks: scripts/llm/corpus_tasks.json5 (written by Sonnet once with --new-tasks, then kept and committed).
Output: data/llm/corpus/<date>.json5, .md and <date>/<verdict>/<task>.warp. Guide: notes/llm_tools.md"""
import argparse
import os
import re

import llm

TOOL = "corpus"
TASKS_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "corpus_tasks.json5")
TASK_COUNT = 300
GUIDES = ["web/playground/guide.md", "web/playground/guide-expert.md"]
TASKS_SCHEMA = {"type": "object", "additionalProperties": False, "required": ["tasks"], "properties": {
    "tasks": {"type": "array", "items": {"type": "object", "additionalProperties": False,
              "required": ["name", "task"], "properties": {"name": {"type": "string"}, "task": {"type": "string"}}}}}}
TASKS_PROMPT = f"""List {TASK_COUNT} small programming tasks in the style of Rosetta Code, for testing a new
programming language: numbers and exact arithmetic, big integers, strings and text processing, lists, maps,
sorting and searching, recursion, higher-order functions and closures, classes and methods, pattern matching,
error handling, loops, units, dates and times, simple simulations, puzzles and classic algorithms. Each task must be
solvable in 3 to 40 lines, need no input, network, files, shell or window, and print a deterministic result that
the task states precisely (e.g. "print the first 10 Fibonacci numbers separated by spaces"). name: a short
lowercase identifier with underscores, unique. task: one or two sentences."""
PROGRAM_SCHEMA = {"type": "object", "additionalProperties": False, "required": ["program", "expected_output"],
                  "properties": {"program": {"type": "string"}, "expected_output": {"type": "string"}}}
WRITER_SYSTEM = """You are a programmer who has just read the guide of warp, a new programming language, below.
You know nothing else about warp. Write a complete warp program for the task you are given, using only what the
guide shows (in the style it shows), as a new user would. program: the source code only, no markdown fences.
expected_output: exactly what the program should print when run, line by line, followed by its final value if the
program's last line is a value (warp shows the last value after the printed lines).

"""
JUDGE_SYSTEM = """A new user wrote a warp program for a task, using only the language guide below, and ran it.
Decide what the result shows about warp.

""" + llm.VERDICT_MEANINGS + """

Here, doc_wrong means the guide taught something that does not work, or misled the user. A correct result with a
different but sensible format (an extra final value line, yes/no for booleans) is ok. Keep the reason to one or two
sentences naming the expected and the actual result and the warp feature involved.

"""


def guide_text():
    return "\n\n".join(open(os.path.join(llm.ROOT, page), encoding="utf-8").read() for page in GUIDES)


def tasks(new):
    if new or not os.path.exists(TASKS_FILE):
        answer = llm.run_batch(TOOL, "tasks", [llm.request("tasks", llm.SONNET, "You design test tasks.",
                               TASKS_PROMPT, TASKS_SCHEMA, 64000, "medium")], 500, 30000)["tasks"]
        unique = {task["name"]: task for task in answer["tasks"]}
        llm.write_json5(TASKS_FILE, list(unique.values()), "Rosetta-style tasks for scripts/llm/corpus.py (Sonnet)")
    return llm.read_json5(TASKS_FILE, [])


def unfenced(program):
    """The model sometimes wraps the program in a markdown fence anyway"""
    match = re.fullmatch(r"\s*```\w*\n(.*?)```\s*", program, re.S)
    return match.group(1) if match else program


def describe(item):
    return (f"Task: {item['task']}\n\nProgram:\n{item['code']}\n\nExpected output (the user's own expectation):\n"
            f"{item['expected']}\n\nActual output of `warp run`:\n{item['output'] or '(nothing)'}")


def save_programs(items, verdicts, base):
    for item in items:
        folder = os.path.join(base, verdicts.get(item["id"], {}).get("verdict", "unjudged"))
        os.makedirs(folder, exist_ok=True)
        open(os.path.join(folder, item["id"] + ".warp"), "w", encoding="utf-8").write(
            f"// {item['task']}\n{item['code']}")


def report(items, verdicts, plan, version, path):
    by_verdict = {}
    for item in items:
        by_verdict.setdefault(verdicts.get(item["id"], {}).get("verdict", "unjudged"), []).append(item)
    lines = [f"# Program corpus {llm.today()}", "", version, "",
             ", ".join(f"{verdict}: {len(group)}" for verdict, group in sorted(by_verdict.items())), "",
             "# Candidates for samples/ or tests (worked as the task asked)", ""]
    lines += [f"- {item['id']} ({item['code'].count(chr(10)) + 1} lines): {item['task']}"
              for item in sorted(by_verdict.get("ok", []), key=lambda item: -len(item["code"]))]
    for verdict in llm.PROBLEM_VERDICTS + ("unclear",):
        for item in by_verdict.get(verdict, []):
            lines += ["", f"## {verdict}: {item['id']}", verdicts[item["id"]]["reason"], "```warp",
                      item["code"].strip(), "```", "```", item["output"], "```"]
    lines += ["", "# Cards", ""] + [f"- new: {card['title']}" for card in plan["new_cards"]]
    lines += [f"- known: {known['finding_id']} → {known['card_key']}" for known in plan["known"]]
    open(path, "w", encoding="utf-8").write("\n".join(lines) + "\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dry-run", action="store_true", help="no cards, only the report")
    parser.add_argument("--limit", type=int, help="only the first N tasks (a cheap trial)")
    parser.add_argument("--new-tasks", action="store_true", help="let Sonnet write a fresh task list")
    parser.add_argument("--file-cards", metavar="REPORT.json5", help="add the cards of a reviewed dry run")
    options = parser.parse_args()
    if options.file_cards:
        return llm.file_saved_cards(options.file_cards, "corpus.py")
    chosen = tasks(options.new_tasks)[:options.limit]
    print(f"{len(chosen)} tasks")
    written = llm.run_batch(TOOL, "write", [llm.request(task["name"], llm.SONNET, WRITER_SYSTEM + guide_text(),
                            task["task"], PROGRAM_SCHEMA, 16000, "medium") for task in chosen], 13000, 3000)
    items = [{"id": task["name"], "task": task["task"], "code": unfenced(written[task["name"]]["program"]),
              "expected": written[task["name"]]["expected_output"]} for task in chosen if task["name"] in written]
    version = llm.warp_version()
    print(f"running {len(items)} programs with {version}")
    for item, output in zip(items, llm.run_all([item["code"] for item in items])):
        item["output"] = output
    verdicts = llm.judge(TOOL, items, describe, JUDGE_SYSTEM + guide_text())
    findings = [{"id": item["id"], "verdict": verdicts[item["id"]]["verdict"], "reason": verdicts[item["id"]]["reason"],
                 "code": item["code"], "output": item["output"]}
                for item in items if verdicts.get(item["id"], {}).get("verdict") in llm.PROBLEM_VERDICTS]
    print(f"{len(findings)} problems")
    plan = llm.file_cards(TOOL, findings, options.dry_run, "corpus.py")
    base = os.path.join(llm.DATA, TOOL, llm.today())
    save_programs(items, verdicts, base)
    llm.write_json5(base + ".json5", {"warp": version, "programs": items, "verdicts": verdicts,
                                      "cards": plan}, "scripts/llm/corpus.py run")
    report(items, verdicts, plan, version, base + ".md")
    print(f"report: {base}.md, programs in {base}/")


if __name__ == "__main__":
    main()
