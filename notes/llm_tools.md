# LLM tools (scripts/llm/)

User decision 2026-10-10: the ~$200 Anthropic API credit on ANTHROPIC_KEY_WARP (~/.keys, the playground proxy's
workspace key) pays for a docs checker, a program corpus and, maybe, extra API workers. Spend stops at $150.

## Tools
- `scripts/llm/docs_check.py [--dry-run] [--limit N] [page.md …]`: every ```warp fence and inline `code` → `value`
  of web/playground/guide.md, guide-expert.md, primer.md and the wiki (read from the main checkout's wiki/) runs
  through `/Users/me/dev/bin/warp --sandbox --no-ask run` (WARP_NO_WINDOW=1, 20 s timeout). An explicit value equal
  to the last output line passes without a model; the rest go to Haiku 5.5 (triage), the non-ok ones to Sonnet 5.5.
  Wiki lines with "Warp before" / "before this change" record old behaviour on purpose and are skipped.
- `scripts/llm/corpus.py [--dry-run] [--limit N] [--new-tasks]`: Sonnet writes a program per task of
  scripts/llm/corpus_tasks.json5 (304 Rosetta-style tasks, written by Sonnet once; keep the list so weekly runs
  compare) knowing only guide.md + guide-expert.md; each runs, then the same judging. Programs land in
  data/llm/corpus/<date>/<verdict>/<task>.warp; the report lists the working ones as sample/test candidates.
- Both: verdicts warp_bug, missing_feature, doc_wrong become findings; one Sonnet call groups them and dedupes them
  against `todo list` (open cards → "known", a Done card's cause → regression card), then `todo add` per new card.
  `--dry-run` only writes the report; after reviewing it, `--file-cards data/llm/<tool>/<date>.json5` adds the
  planned cards without paying again.
- `scripts/llm/llm.py`: key loading, Message Batches (half price; the batch id is kept in data/llm/<tool>/<stage>.batch
  until its results are in, so a rerun resumes instead of paying twice), the cost tally data/llm/cost.json5 (usage
  of every response, file-locked) and the $150 budget check before each batch.

Weekly: `scripts/llm/docs_check.py --dry-run && scripts/llm/corpus.py --dry-run`, read the two .md reports, then
`--file-cards` each. A full run of both costs about $2–3 and takes 20–40 min (batches take minutes even when small).
Needs the anthropic SDK ≥ 1.0 (output_config) and json5: `pip3 install --upgrade --break-system-packages anthropic json5`.

## Claude Code on the API key (checked 2026-10-10, Claude Code 2.1.296)
- Headless works: `ANTHROPIC_API_KEY=$ANTHROPIC_KEY_WARP claude -p …` (apiKeySource ANTHROPIC_API_KEY), and its
  ListAgents and SendMessage reach the local sessions (a probe's message arrived in warp-llm).
- Interactive does not: the same key in an interactive session gets `400 Your credit balance is too low` on every
  turn, while `-p` requests a minute apart succeed. The only request difference in the debug log is the billing
  header `cc_entrypoint=cli` versus `sdk-cli`: the credit apparently covers API/SDK use, not the interactive CLI.
- Remote Control: `/remote-control` in a session on the key says "Remote Control is disabled by your organization's
  policy" (the key's organization, not the claude.ai one). So no claude.ai URL, no phone control.
- A trivial Opus turn costs ~$0.14 (37k tokens of system prompt and tools, cached once); a real worker session
  costs dollars to tens of dollars on Opus 5.5 ($4/$20 per MTok), a fraction on Sonnet 5.5.
- Wrapper used for the probes: source ~/.keys, `export ANTHROPIC_API_KEY=$ANTHROPIC_KEY_WARP`, `exec claude "$@"`
  (the key stays out of argv). The first interactive start asks "Do you want to use this API key?".
