# Wiki branch: master only

A GitHub wiki serves **only `master`**; no setting (web or API) changes it (GitHub Support in community discussion
#48537, still true in 2026). `gh api repos/pannous/warp.wiki` is a 404: the wiki has no API `default_branch`.
Pushing only `main` never shows on the web wiki.

- Agents push wiki edits with `git push origin HEAD:master` (notes/agents/common.md).
- The user's checkout /Users/me/dev/angles/warp/wiki keeps a local `main` that tracks and pushes to `master`
  (`branch.main.merge=refs/heads/master`, `remote.origin.push=refs/heads/main:refs/heads/master`).
- A remote `main` is a leftover: the user deletes it (`git push origin --delete main` in the wiki checkout; the
  git-destructive hook blocks agents). Any one there is contained in master.
- No server-side mirror: wiki repos run no Actions or hooks, and a `gollum`-event Action fires only for web edits.
