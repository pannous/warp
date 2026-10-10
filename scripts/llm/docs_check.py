#!/usr/bin/env python3
"""Docs checker: runs every example of the wiki, the playground guides and the primer through warp, has Claude
compare each result with what the doc claims (Haiku triage, Sonnet for the doubtful ones, Message Batches) and
files each real mismatch as a board card, deduplicated against the existing cards. Meant to be rerun weekly.
Usage: scripts/llm/docs_check.py [--dry-run] [--limit N] [page.md …]   (default: every doc page)
       scripts/llm/docs_check.py --file-cards data/llm/docs_check/<date>.json5   (cards of a reviewed dry run)
Output: data/llm/docs_check/<date>.json5 and .md; cost in data/llm/cost.json5. Guide: notes/llm_tools.md"""
import argparse
import glob
import json
import os
import re

import llm

TOOL = "docs_check"
GUIDES = ["web/playground/guide.md", "web/playground/guide-expert.md", "web/playground/primer.md"]
WARP_LANGUAGES = {"", "wasp", "warp", "angle"}
FENCE = re.compile(r"^```\s*(\w*)(.*)$")
INLINE_EXAMPLE = re.compile(r"`([^`]+)`\s*→\s*`([^`]+)`")
NEWLINE_MARK = "⏎"  # the wiki writes a line break inside inline code as ⏎
VALUE_MARK = "=>"  # ```warp => value: the value the guide shows
PRINTED_FENCE = "printed"  # a ```printed block right after a snippet: what it prints
# ```warp compiles: servers and windows, compiled only (tests/web/test_primer.rs); ```warp planned: an unbuilt
# feature with its own card (warp-docs, 2026-10-10)
SKIP_MARKS = ("compiles", "planned")
HISTORY_MARKS = ("Warp before", "before this change")  # wiki lines that record old behaviour on purpose
CONTEXT_BEFORE, CONTEXT_AFTER = 900, 400  # characters of the doc around an example shown to the model
SYSTEM = """You check the documentation of warp, a data format and wasm-first programming language, against the
actual behaviour of its current build. The language guide follows for reference.

""" + llm.VERDICT_MEANINGS + """

The doc's claim may be explicit (a value after => or →, a printed block, a comment like // 6 or # 6) or only in the
prose around the example ("gives 3", "is an error"). Without any claim, ok means it ran without an error that the
doc would not expect. Examples that show an error on purpose are ok when warp gives that kind of error. Claims
about other languages (Python, JS, C, …) or about warp's past behaviour ("before", "used to", "was") are not claims
about the current warp: not_runnable. An inline example may use a name (x, s) that an earlier example of the same
paragraph defined; when the code fails or differs only because that definition is missing, it is not_runnable.
Judge only this example. Keep the reason to one or two sentences naming the claimed and the actual result.

"""


def guide_text():
    return "\n\n".join(open(os.path.join(llm.ROOT, page), encoding="utf-8").read() for page in GUIDES[:2])


def doc_pages(arguments):
    if arguments:
        return [os.path.abspath(page) for page in arguments]
    wiki = sorted(glob.glob(os.path.join(llm.MAIN_CHECKOUT, "wiki", "*.md")))
    return [os.path.join(llm.ROOT, page) for page in GUIDES] + wiki


def short_name(page):
    return os.path.relpath(page, llm.MAIN_CHECKOUT if "/wiki/" in page else llm.ROOT)


def examples(page):
    """Every fenced warp block and every inline `code` → `value` of a page, with the doc text around it"""
    text = open(page, encoding="utf-8").read()
    lines = text.splitlines(keepends=True)
    offsets = [0]
    for line in lines:
        offsets.append(offsets[-1] + len(line))
    name, found, number = short_name(page), [], 0

    def context(start, end):
        return text[max(0, start - CONTEXT_BEFORE):start] + "⟦EXAMPLE⟧" + text[end:end + CONTEXT_AFTER]

    while number < len(lines):
        fence = FENCE.match(lines[number].strip())
        if not fence:
            inline = [] if any(mark in lines[number] for mark in HISTORY_MARKS) else INLINE_EXAMPLE.findall(lines[number])
            for position, (code, claim) in enumerate(inline):
                suffix = f"#{position + 1}" if len(inline) > 1 else ""
                found.append({"id": f"{name}:{number + 1}{suffix}", "code": code.replace(NEWLINE_MARK, "\n"), "claim": claim,
                              "context": context(offsets[number], offsets[number + 1])})
            number += 1
            continue
        language, info = fence.group(1), fence.group(2).strip()
        end = next((index for index in range(number + 1, len(lines)) if lines[index].strip().startswith("```")),
                   len(lines))
        code = "".join(lines[number + 1:end])
        claim = info[len(VALUE_MARK):].strip() if info.startswith(VALUE_MARK) else None
        following = end + 1
        if following < len(lines) and lines[following].strip() == "```" + PRINTED_FENCE:
            printed_end = next(index for index in range(following + 1, len(lines)) if lines[index].strip() == "```")
            claim = (claim or "") + " printed: " + "".join(lines[following + 1:printed_end]).strip()
        if language in WARP_LANGUAGES and code.strip() and not any(mark in info for mark in SKIP_MARKS):
            found.append({"id": f"{name}:{number + 1}", "code": code, "claim": claim,
                          "context": context(offsets[number], offsets[min(end + 1, len(lines))])})
        number = end + 1
    return found


def normalized(text):
    text = str(text).strip().replace('"', "'").replace(" ", "")
    return {"true": "yes", "false": "no"}.get(text, text.strip("'"))


def obviously_matches(example):
    """An explicit value equal to the last output line needs no model"""
    claim, output = example["claim"], example["output"]
    if not claim or "printed:" in claim or not output:
        return False
    return normalized(output.splitlines()[-1]) == normalized(claim)


def describe(example):
    return (f"Page and line: {example['id']}\n\nDoc around the example:\n{example['context']}\n\n"
            f"Example code:\n{example['code']}\n\nExplicit claim: {example['claim'] or 'none'}\n\n"
            f"Actual output of `warp run` (printed lines, then the result):\n{example['output'] or '(nothing)'}")


def report(examples_run, verdicts, plan, version, path):
    by_verdict = {}
    for example in examples_run:
        by_verdict.setdefault(verdicts.get(example["id"], {}).get("verdict", "ok"), []).append(example)
    lines = [f"# Docs check {llm.today()}", "", version, "",
             ", ".join(f"{verdict}: {len(items)}" for verdict, items in sorted(by_verdict.items())), ""]
    for verdict in llm.PROBLEM_VERDICTS + ("unclear", "program_wrong"):
        for example in by_verdict.get(verdict, []):
            lines += [f"## {verdict}: {example['id']}", verdicts[example["id"]]["reason"], "```warp",
                      example["code"].strip(), "```", "```", example["output"], "```", ""]
    lines += ["# Cards", ""] + [f"- new: {card['title']}" for card in plan["new_cards"]]
    lines += [f"- known: {known['finding_id']} → {known['card_key']}" for known in plan["known"]]
    open(path, "w", encoding="utf-8").write("\n".join(lines) + "\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dry-run", action="store_true", help="no cards, only the report")
    parser.add_argument("--limit", type=int, help="only the first N examples (a cheap trial)")
    parser.add_argument("--file-cards", metavar="REPORT.json5", help="add the cards of a reviewed dry run")
    parser.add_argument("pages", nargs="*")
    options = parser.parse_args()
    if options.file_cards:
        return llm.file_saved_cards(options.file_cards, "docs_check.py")
    found = [example for page in doc_pages(options.pages) for example in examples(page)][:options.limit]
    version = llm.warp_version()
    print(f"{len(found)} examples, running them with {version}")
    for example, output in zip(found, llm.run_all([example["code"] for example in found])):
        example["output"] = output
    doubtful = [example for example in found if not obviously_matches(example)]
    print(f"{len(found) - len(doubtful)} match their explicit value, {len(doubtful)} go to the model")
    verdicts = llm.judge(TOOL, doubtful, describe, SYSTEM + guide_text())
    findings = [{"id": example["id"], "verdict": verdicts[example["id"]]["verdict"],
                 "reason": verdicts[example["id"]]["reason"], "code": example["code"], "output": example["output"]}
                for example in doubtful if verdicts.get(example["id"], {}).get("verdict") in llm.PROBLEM_VERDICTS]
    print(f"{len(findings)} problems: {', '.join(finding['id'] for finding in findings)}")
    plan = llm.file_cards(TOOL, findings, options.dry_run, "docs_check.py")
    base = os.path.join(llm.DATA, TOOL, llm.today())
    llm.write_json5(base + ".json5", {"warp": version, "examples": found, "verdicts": verdicts,
                                      "cards": plan}, "scripts/llm/docs_check.py run")
    report(found, verdicts, plan, version, base + ".md")
    print(f"report: {base}.md")


if __name__ == "__main__":
    main()
