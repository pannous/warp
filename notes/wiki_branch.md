# Wiki branch: master vs main (investigated 2026-10-03)

## Verdict
A GitHub wiki serves **only `master`**. There is no setting (web or API) to change it, and GitHub has not
changed this as of June 2026. `main` cannot become the served branch, so it cannot be "like the code repo".

## Evidence
- GitHub Support, quoted in community discussion #48537 (Jan 2024): "The default branch for GitHub Wiki's is
  master, and we do not provide a method to change this on the GitHub website ... only the contents of the master
  branch will be displayed." They mention an internal issue; nothing shipped.
- Same thread, Sep 2023: renaming master → main made the wiki disappear. Jun 9 2026: "Confirmed this is still true".
  https://github.com/orgs/community/discussions/48537
- The 2020 main rename (github/renaming, roadmap #63) covered normal repos only; github/renaming is archived since 2023.
- Harmless checks here:
  - `gh api repos/pannous/warp.wiki` → 404: the wiki is not an API repository, so it has no `default_branch` to read
    or PATCH. The code repo pannous/warp has `default_branch: main`, which does not carry over to the wiki.
  - `git ls-remote --symref https://github.com/pannous/warp.wiki.git HEAD` → `ref: refs/heads/master`.
  - Both `main` and `master` point at ddbb3fd (merged today), so nothing is lost by dropping either one.
- Pushing only `main` does not show up on the web wiki (that is how the drift that needed ddbb3fd happened).

## Recommendation (cleanest setup)
1. Keep `master` as the only remote branch: delete remote `main` (`git push origin --delete main` in the wiki
   checkout). Risk: none for content (identical tip); a session that still pushes `main` would recreate it,
   so the rule below must be the only one agents read. **User decision** (deletes a branch).
2. Agents: `git push origin HEAD:master` (already in notes/agents/common.md). After step 1, drop the line
   "`main` there is only a mirror kept for old habits" from common.md.
3. The user's checkout /Users/me/dev/angles/warp/wiki: already correct. Its local `main` tracks and pushes to
   `master` (`branch.main.merge=refs/heads/master`, `remote.origin.push=refs/heads/main:refs/heads/master`), so the
   user can keep calling it main locally. Optional cosmetic alternative: `git branch -m main master`
   and unset `remote.origin.push`. No change needed.
4. Not recommended: a server-side mirror (no Actions/hooks run in wiki repos; an Action in pannous/warp on the
   `gollum` event could sync, but it only fires for web edits and adds a token + moving part for no gain).
