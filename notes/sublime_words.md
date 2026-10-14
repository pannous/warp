# Sublime Text word lists

The Sublime package is its own repo: "/Users/me/Library/Application Support/Sublime Text 3/Packages/Warp"
(pannous/warp-sublime-text). Its scripts/update_words.py reads warp's keyword, operator, type, built-in and unit tables
and rewrites the GENERATED block of Warp.sublime-syntax and Warp.sublime-completions; scripts/check_syntax.py checks
the result headless (needs python modules regex + yaml and syntect's syntest example).

- SOURCES names tables and functions by name only, found anywhere under src/ and crates/*/src: moving a file never
  breaks it. A name defined twice with different contents needs a module or folder qualifier (`tasks::TASK_WORDS`).
  A renamed or removed table is a loud error listing every missing name at once.
- Cron 05:50 daily runs scripts/auto_update.sh (log ~/.companion/logs/sublime-words.log): regenerates from the main
  checkout, checks, commits + pushes when the lists changed. On failure: macOS notification and one `todo add` card per
  distinct failure (~/.cache/sublime-words.last-failure).
- update_words.py also fails when wiki/keyword.md misses a keyword warp has: the fix is usually a wiki row.
