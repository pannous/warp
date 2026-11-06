# Card sprint in the cloud (2026-11-06)

Ten cloud sessions take the sprint's Later-column lib cards, cloud-NN.tsv each (key, topic, triage evidence, area).
The local area workers (../README.md) no longer hold these cards.

## Rules for a cloud sprinter
- Judge each card first (user): an esoteric feature (foreign syntax, desk-invented phrasing, bloat) or important to
  the language? Esoteric: don't build it, report `obsolete: <key> esoteric: <why>`. Unsure: ask warp-interviewer
  (cloud: mark the result line `question`) and take the next card meanwhile.
- The sprint rules in ../README.md hold, except: no board, no SendMessage, no lib/prelude.warp edits.
- Branch cloud-NN from origin/main; one commit per card, the key in the message (`fix(<key>): …`).
- Tools: install the Linux release (https://github.com/pannous/warp/releases/latest,
  warp-v*-x86_64-unknown-linux-gnu.tar.gz) and check behavior with `WARP_LIB=$PWD/lib warp <file|code>`; if lib
  changes need a rebuilt compiler/modules and that does not work in the cloud, say "unsmoked" on the result line.
- Write notes/sprint/cloud/cloud-NN.result.tsv on the branch, one line a card:
  `key<TAB>done|core|obsolete|prelude|skipped<TAB>commit or reason` (prelude: the hunk it needs, for warp-sprint-spec).
  A card whose change affects core/ is not approached: `core` with the reason.
- Push the branch often (a cloud box can vanish). The Integrator smokes and lands cloud-NN on the Mac; the
  Supervisor applies the result file to the board.
01 https://claude.ai/code/session_018Xoawma9cQcHffHi4hbYQk
02 https://claude.ai/code/session_018XoawogY9jcGdddiomjgDd
03 https://claude.ai/code/session_018XoawnuFqdCf49XHg2Lbv5
04 https://claude.ai/code/session_018XoawnRp6T7Y9pr4aSTXjf
05 https://claude.ai/code/session_018XoawmxFemegg7pK561MXW
06 https://claude.ai/code/session_018XoawmiSBtjcZ6tXCf9K66
07 https://claude.ai/code/session_018XoawmUo9fEn92KbeFYbvM
08 https://claude.ai/code/session_018Xoawmpw7euUA2kkVY5NRV
09 https://claude.ai/code/session_018Xoawnw1X8FbJwH4dZafsM
10 https://claude.ai/code/session_018XoawkmPGHcg2gyscamE2q
