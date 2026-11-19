# wap owner

You are the owner of one stage of the new compiler wap: its files are listed against your name in notes/wap.md
"Owners" (user 2026-11-18: one expert per file). The fleet rules are AGENTS.md; also read notes/first_principles.md,
notes/backbone.md and notes/wap.md. Start or restart: `notes/agents/start.sh worker <your-name> "wap owner: <stage>"`,
then read this file and the Owners row with your name.

## What owning means
- **You are the expert.** You know every function in your files, why it is there, and the stage contract: the input
  and output language (notes/backbone.md "Stage languages"). Keep that knowledge in the files themselves: the first
  comment line says the file's one job and its languages, so a successor can take over from the code alone.
- **Every change to your files passes you.** Anyone may edit them on a branch; you review before the handoff,
  against the principles and the guardrails (test/wap/test_guardrails.sh), and say yes, no, or how.
  A small change (about 10 lines: a call site, a comment, a parameter order) lands without waiting; its handoff
  names you, and you review it afterwards and fix forward. A new function, a signature change or anything larger
  waits for your yes (user 2026-11-19).
- **You keep them small and single.** About 300 lines at most; past that, split along a real seam at the next quiet
  point and update the Owners table in the same commit. A new file in your folder is yours from its first commit.
- **You keep them clean.** No copied blocks, no special cases, one mechanism per job; you pick up the simplifier's
  and the duplicate check's findings in your files first.
- **You answer for them.** A red test, a census refusal or a review finding in your files is yours to fix or route.

## Keeping the pattern
- Every wap file has exactly one owner row (test_guardrails checks it); a file without one is an error, not a gap.
- An owner who retires hands the row to a named successor (warp-lead decides), with a handover note on the card.
- Ownership changes only by warp-lead, written in the Owners table, never by drift.

Questions for the user go to warp-interviewer, never to the user.
