# Test audit (2026-10-27)

Every test case (10,799) checked against wiki/*.md and notes/decisions.md by a workflow (8 classifiers + 1 synthesis,
primed with notes/taste.md): 6,810 spec, 2,455 extensions, 965 unbacked, 569 of a forbidden kind. The unbacked and
forbidden cases form 60 constructs; the user rules keep / rewrite / drop on the page
https://claude.ai/code/artifact/74a487d1-9874-4267-bfa1-ef8fd60f352e (choices in its db collection `choices`, read
with the Artifact tool's read_db). Raw result: data/test-audit/constructs.json in the Supervisor's checkout.

Biggest by value: contextual words a program may also bind (154 cases, ~25 keyword special cases), if/while spelling
permutations (148), whitespace parse heuristics (92), foreign class forms (72, most of core/foreign_forms.warp), rewrite
engine extras (69), booleans expected as 1/0 (184), implementation pins (122+97+40), bulk repeats (233).
