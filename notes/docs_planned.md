# Planned examples in the wiki and guide

A doc example of a feature that is not built yet stays in the doc, fenced as ```` ```warp planned ````, with a one-line
`*Planned:* …` sentence above it, and a feature card in column Later. Examples: wiki/full-evaluation.md (card
runtime-dates), wiki/plural.md (plural-names), wiki/result.md (result-keyword). When the feature lands, its worker drops
`planned` from the fence.

The weekly docs checker (scripts/llm/docs_check.py, reports in data/llm/docs_check/) runs every warp fence and skips
fences marked `compiles` or `planned`.

A doc example that fails only because of a renamed or reserved word (`double` is a type, `fun` a keyword) is a doc
fix: rename it (`twice`, `area`) and check it with /Users/me/dev/bin/warp.
