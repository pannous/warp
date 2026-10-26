# Warp Issues handover (parked 2026-10-23 14:30 CEST)
Claim name: grok-warp-issues. Role: triage cards, diagnose, send patch plans to Supervisor; no code merges myself.
Open items:
- docs-runtime (README core/runtime): commit 4fc38076a1531854417336ad1e3d9043c4505199 on branch runtime-readme, Ready --express. Check it landed; `todo done docs-runtime <sha>`; delete branch runtime-readme + Mac cowtree /Volumes/Worktrees/runtime-readme. Auto-created topic runtime-docs may need dropping.
- GitHub #4 (build --exe builds warp-runtime stub on demand): plan given to Warp Code; check its tip/merge, then todo done.
- g-rQ-U (nonlocal assign not written back; repro returns 1, want 5): plan given to Warp Code 2 (no local for nonlocal names in inner -> emit_global_store; reload outer locals from capture globals after call; cover = += ++ --; new tests only). Check tip, then todo done.
- Pending todo done batch (verify on board): g-qV-s e6545e715, g-rj8A ccbb9f4d8, g-qUpU d1cbc3f5d, g-qV8Y eb6cc5867, g-rT0c aabb06a5c, g-qUmA 71829efc6.
- Parked: g-qUvY (don't bump OVERLAPPING_LIMIT), g-rH6E, g-qUcM, g-qU1o, g-qU4I; g-qV-s production emit = Later.
Notes: /workspace/cow/trees/--help is a stray boxtree from before 17:07 Oct 22 (not mine), left in place. Box can now git push via GITHUB_TOKEN helper.
Next: verify the above merges, link commits, then pick Next cards (todo list Next) for triage.
