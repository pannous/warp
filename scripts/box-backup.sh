#!/usr/bin/env bash
# scripts/box-backup.sh — make sure no warp work lives only on the dev box.
#
# Sweeps every pannous/warp clone and worktree under $BOX_ROOT (default
# /workspace) and pushes to origin, WITHOUT touching any worktree, index,
# HEAD or local ref (other agents may be working there):
#   1. inventory clones/worktrees, local branches not contained in any origin
#      branch, and dirty worktrees;
#   2. unpushed local branches -> origin/<branch> when new or fast-forward,
#      otherwise origin/backup/<branch> (never force; local `main` always goes
#      to backup/main-<repo>);
#   3. dirty worktrees -> snapshot commit (temporary index, parent HEAD) pushed
#      to origin/backup/<worktree>; a clean detached HEAD that is not on origin
#      is pushed there too;
#   4. hand-written box notes (handovers, investigation .md, box scripts) are
#      copied to notes/box/ on main and pushed.
# Idempotent: refs already on origin and unchanged snapshots/notes are skipped.
# Secrets: diffs are grepped before every push; files with hits are left out,
# branches with hits are not pushed. Files > 5 MB, target/, *.wasm, *.log are
# never snapshotted.
#
# usage: scripts/box-backup.sh [-n|--dry-run] [--no-notes|--notes-only] [--root DIR]
# env: BOX_BACKUP_EXTRA=<file> also copies <file> to scripts/box-backup.sh in the notes commit.
set -uo pipefail

DRY=0 NOTES=1 REPOS=1 ROOT=${BOX_ROOT:-/workspace}
while [ $# -gt 0 ]; do
  case $1 in
    -n|--dry-run) DRY=1 ;;
    --no-notes) NOTES=0 ;;
    --notes-only) REPOS=0 ;;
    --root) ROOT=$2; shift ;;
    -h|--help) sed -n '2,/^set -uo/{/^set -uo/d;p}' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
  shift
done

URL=https://github.com/pannous/warp.git
STATE=$ROOT/.box-backup
MIRROR=$STATE/mirror.git
MAX_BYTES=$((5 * 1024 * 1024))
STAMP=$(date +%Y%m%d-%H%M)
SECRET_RE='ghp_[A-Za-z0-9]{20,}|gho_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|(^|[^A-Za-z0-9_-])sk-[A-Za-z0-9_-]{20,}|BEGIN [A-Z ]*PRIVATE KEY|xox[abprs]-[A-Za-z0-9-]{10,}|AKIA[0-9A-Z]{16}|whsec_[A-Za-z0-9]{10,}'
SECRET_NAME_RE='(^|/)(\.env(\..*)?|.*\.pem|.*\.key|id_rsa.*|id_ed25519.*|\.netrc|\.git-credentials|credentials(\.json)?)$'
SKIP_PATH_RE='(^|/)(target|node_modules)(/|$)|\.wasm$|\.log$'
NOTE_SCRIPTS="box-slot.sh build-fresh-warp.sh install-fresh-warp.sh selfcheck-box.sh selfcheck-branch.sh cow/box-self-check.sh"
NOTE_SKIP_RE='(^|/)(user-claude-global|chip)\.md$'  # personal / non-warp notes

REPORT=()
say() { echo "$*"; }
note() { REPORT+=("$*"); echo "  -> $*"; }
run() { if [ $DRY = 1 ]; then echo "  [dry-run] $*"; else "$@"; fi; }

# Identity for snapshot/notes commits (clones may lack user.* config).
export GIT_AUTHOR_NAME=${GIT_AUTHOR_NAME:-$(git -C "$ROOT/cow/warp" config user.name 2>/dev/null || echo pannous)}
export GIT_AUTHOR_EMAIL=${GIT_AUTHOR_EMAIL:-$(git -C "$ROOT/cow/warp" config user.email 2>/dev/null || echo info@pannous.com)}
export GIT_COMMITTER_NAME=$GIT_AUTHOR_NAME GIT_COMMITTER_EMAIL=$GIT_AUTHOR_EMAIL

# Reads stdin; true if it contains a secret pattern or a live token value.
has_secret() {
  local data; data=$(cat)
  printf '%s' "$data" | grep -aqE -- "$SECRET_RE" && return 0
  local v
  for v in "${GITHUB_TOKEN:-}" "${GH_PROJECT_TOKEN:-}"; do
    [ ${#v} -ge 8 ] && printf '%s' "$data" | grep -aqF -- "$v" && return 0
  done
  return 1
}

# --- private bare mirror of all origin branches (never touches shared repos)
mirror_sync() {
  if [ ! -d "$MIRROR" ]; then
    git init -q --bare "$MIRROR"
    git -C "$MIRROR" remote add origin "$URL"
    [ -d "$ROOT/cow/warp/.git/objects" ] && echo "$ROOT/cow/warp/.git/objects" > "$MIRROR/objects/info/alternates"
  fi
  git -C "$MIRROR" fetch -q --prune origin '+refs/heads/*:refs/remotes/origin/*'
}
mirror_ref() { git -C "$MIRROR" fetch -q origin "+refs/heads/$1:refs/remotes/origin/$1" 2>/dev/null; }
remote_sha() { git -C "$MIRROR" rev-parse -q --verify "refs/remotes/origin/$1^{commit}" 2>/dev/null; }
on_origin() {
  git -C "$MIRROR" cat-file -e "$1^{commit}" 2>/dev/null || return 1
  [ -n "$(git -C "$MIRROR" for-each-ref --count=1 --contains "$1" refs/remotes/origin)" ]
}
# Origin tips whose objects exist in repo $1 (to bound log -p scans).
known_tips() {
  git -C "$MIRROR" for-each-ref --format='%(objectname)' refs/remotes/origin | sort -u |
    git -C "$1" cat-file --batch-check='%(objectname) %(objecttype)' 2>/dev/null | awk '$2=="commit"{print "^"$1}'
}
# true if branch $3 may be pushed from repo $1 to ref name $2 as a fast-forward or new ref.
can_push_as() {
  local r; r=$(remote_sha "$2") || return 0
  git -C "$1" merge-base --is-ancestor "$r" "$3" 2>/dev/null
}
free_backup_name() { # repo base sha
  local n=$2
  can_push_as "$1" "$n" "$3" && { echo "$n"; return; }
  echo "$n-$STAMP"
}
push_sha() { # repo sha ref
  say "  push $(git -C "$1" rev-parse --short "$2") -> origin/$3"
  if [ $DRY = 1 ]; then echo "  [dry-run] git push origin $2:refs/heads/$3"; return 0; fi
  git -C "$1" push -q origin "$2:refs/heads/$3" && mirror_ref "$3"
}

# --- step 1+2: branches of one repository (common git dir)
backup_branches() { # repo-path repo-name
  local repo=$1 name=$2 b sha tgt
  while read -r b sha; do
    on_origin "$sha" && continue
    say "  unpushed branch $b ($(git -C "$repo" rev-parse --short "$sha"))"
    if git -C "$repo" log -p --format= "$sha" $(known_tips "$repo") -- 2>/dev/null | has_secret; then
      note "SKIPPED branch $name:$b — secret pattern in unpushed diff"; continue
    fi
    if [ "$b" = main ] || [ "$b" = master ]; then tgt=$(free_backup_name "$repo" "backup/$b-$name" "$sha")
    elif can_push_as "$repo" "$b" "$sha"; then tgt=$b
    else tgt=$(free_backup_name "$repo" "backup/$b" "$sha"); fi
    push_sha "$repo" "$sha" "$tgt" && note "branch $name:$b -> $tgt @ $sha" || note "FAILED push $name:$b -> $tgt"
  done < <(git -C "$repo" for-each-ref --format='%(refname:short) %(objectname)' refs/heads)
}

# Drop path $1 from the temp index (back to HEAD's version, or untracked).
drop_path() {
  local e; e=$(git ls-tree HEAD -- "$1")
  if [ -n "$e" ]; then set -- "$1" $e; git update-index --cacheinfo "$2,$4,$1"
  else git update-index -q --force-remove -- "$1"; fi
}

# --- step 3: snapshot one worktree via a temporary index
backup_worktree() { # wt-path wt-name
  local wt=$1 name=$2 head idx tmp tree ok=1
  head=$(git -C "$wt" rev-parse -q --verify HEAD) || { note "SKIPPED $wt — no HEAD"; return; }
  idx=$(git -C "$wt" rev-parse --path-format=absolute --git-path index)
  tmp=$(mktemp)
  (
    cd "$wt" || exit 1
    export GIT_INDEX_FILE=$tmp
    if [ -f "$idx" ]; then cp "$idx" "$tmp"; else git read-tree HEAD; fi
    git update-index -q --refresh >/dev/null 2>&1
    # candidate paths: modified, deleted, untracked-not-ignored
    git ls-files -z -m -d -o --exclude-standard | sort -zu |
      while IFS= read -r -d '' p; do
        if [[ $p =~ $SKIP_PATH_RE ]]; then continue; fi
        if [[ $p =~ $SECRET_NAME_RE ]]; then echo "SECRET-NAME $p" >> "$tmp.skip"; continue; fi
        if [ -f "$p" ] && [ "$(stat -c %s -- "$p")" -gt $MAX_BYTES ]; then echo "SIZE $p" >> "$tmp.skip"; continue; fi
        printf '%s\0' "$p"
      done | git add -A --ignore-errors --pathspec-from-file=- --pathspec-file-nul >/dev/null 2>&1
    git write-tree >/dev/null 2>&1 || git read-tree HEAD || exit 3
    mapfile -d '' changed < <(git diff --cached --name-only -z HEAD)
    for p in "${changed[@]}"; do
      if git diff --cached -p HEAD -- "$p" | grep -a '^+' | has_secret; then
        echo "SECRET $p" >> "$tmp.skip"; drop_path "$p"
      fi
    done
    git write-tree
  ) > "$tmp.tree" 2>/dev/null || ok=0
  tree=$(cat "$tmp.tree" 2>/dev/null)
  if [ -f "$tmp.skip" ]; then
    while read -r kind p; do note "SKIPPED file $name:$p ($kind)"; done < "$tmp.skip"
  fi
  rm -f "$tmp" "$tmp.tree" "$tmp.skip"
  [ $ok = 1 ] && [ -n "$tree" ] || { note "FAILED snapshot of $wt"; return; }

  if [ "$tree" = "$(git -C "$wt" rev-parse "HEAD^{tree}")" ]; then
    on_origin "$head" && return
    say "  $name: clean, detached HEAD not on origin"
    if git -C "$wt" log -p --format= "$head" $(known_tips "$wt") -- | has_secret; then
      note "SKIPPED HEAD of $name — secret pattern"; return; fi
    local tgt; tgt=$(free_backup_name "$wt" "backup/$name" "$head")
    push_sha "$wt" "$head" "$tgt" && note "head $name -> $tgt @ $head"
    return
  fi
  # unchanged since an earlier snapshot? (same tree, same parent)
  local r
  for r in $(git -C "$MIRROR" for-each-ref --format='%(refname:strip=3)' "refs/remotes/origin/backup/$name" "refs/remotes/origin/backup/$name-2*"); do
    local rs; rs=$(remote_sha "$r")
    if [ "$(git -C "$MIRROR" rev-parse "$rs^{tree}")" = "$tree" ] &&
       [ "$(git -C "$MIRROR" rev-parse "$rs^1" 2>/dev/null)" = "$head" ]; then
      say "  $name: dirty, snapshot already on origin/$r"; return; fi
  done
  say "  $name: dirty -> snapshot"
  if [ $DRY = 1 ]; then echo "  [dry-run] snapshot tree $tree -> backup/$name"; note "would snapshot $name"; return; fi
  local c tgt
  # chain onto the previous snapshot (2nd parent) so backup/<name> fast-forwards
  local prev parents=(-p "$head")
  prev=$(remote_sha "backup/$name") && git -C "$wt" cat-file -e "$prev^{commit}" 2>/dev/null && parents+=(-p "$prev")
  c=$(git -C "$wt" commit-tree "$tree" "${parents[@]}" -m "wip: box backup of $name ($(date '+%Y-%m-%d %H:%M %Z'))" -m "worktree: $wt") || { note "FAILED commit-tree $name"; return; }
  tgt=$(free_backup_name "$wt" "backup/$name" "$c")
  push_sha "$wt" "$c" "$tgt" && note "snapshot $name -> $tgt @ $c" || note "FAILED push snapshot $name"
}

# --- step 4: notes
collect_notes() { # prints relative paths under $ROOT
  (cd "$ROOT" || exit
   ls handover/*.md *.md 2>/dev/null
   find cow -maxdepth 3 -name '*.md' -not -path 'cow/trees/*' -not -path 'cow/warp/*' -not -path '*/target/*' 2>/dev/null
   find cow/scratch scratch -name 'STATUS.md' -not -path '*/target/*' 2>/dev/null
   find scratch -maxdepth 3 -name '*.md' -not -path '*/target/*' 2>/dev/null
   for s in $NOTE_SCRIPTS; do [ -f "$s" ] && echo "$s"; done
  ) | grep -vE "$NOTE_SKIP_RE" | sort -u
}
backup_notes() {
  local wt="$STATE/notes-wt" f n=0
  [ -e "$wt" ] && git -C "$MIRROR" worktree remove --force "$wt" 2>/dev/null
  git -C "$MIRROR" worktree prune
  git -C "$MIRROR" worktree add -q --detach "$wt" refs/remotes/origin/main || { note "FAILED notes worktree"; return; }
  while IFS= read -r f; do
    [ "$(stat -c %s -- "$ROOT/$f")" -gt $MAX_BYTES ] && { note "SKIPPED note $f (SIZE)"; continue; }
    if has_secret < "$ROOT/$f"; then note "SKIPPED note $f (SECRET)"; continue; fi
    mkdir -p "$wt/notes/box/$(dirname "$f")"
    cp -p "$ROOT/$f" "$wt/notes/box/$f"; n=$((n+1))
  done < <(collect_notes)
  [ -n "${BOX_BACKUP_EXTRA:-}" ] && cp -p "$BOX_BACKUP_EXTRA" "$wt/scripts/box-backup.sh"
  git -C "$wt" add -A notes/box scripts/box-backup.sh 2>/dev/null
  if git -C "$wt" diff --cached --quiet; then say "  notes: $n files, nothing new"; git -C "$MIRROR" worktree remove --force "$wt"; return; fi
  git -C "$wt" diff --cached --stat | tail -1
  if git -C "$wt" diff --cached -p | grep -a '^+' | has_secret; then
    note "ABORTED notes commit — secret pattern in staged diff (worktree kept at $wt)"; return; fi
  if [ $DRY = 1 ]; then echo "  [dry-run] commit+push notes"; git -C "$MIRROR" worktree remove --force "$wt"; return; fi
  git -C "$wt" commit -q -m "docs(box): back up box notes and handovers"
  local i
  for i in 1 2 3; do
    git -C "$wt" push -q origin HEAD:refs/heads/main && { note "notes -> main @ $(git -C "$wt" rev-parse HEAD)"; break; }
    git -C "$wt" pull -q --rebase=merges --autostash origin main
  done
  mirror_ref main
  git -C "$MIRROR" worktree remove --force "$wt"
}

# --- main
say "== box-backup $(date '+%F %T %Z') dry-run=$DRY"
mirror_sync || { echo "mirror fetch failed" >&2; exit 1; }
declare -A SEEN_COMMON SEEN_WT SEEN_NAME
while IFS= read -r g; do
  d=$(dirname "$g")
  case $(git -C "$d" remote get-url origin 2>/dev/null) in
    *github.com[:/]pannous/warp|*github.com[:/]pannous/warp.git) ;; *) continue ;;
  esac
  common=$(git -C "$d" rev-parse --path-format=absolute --git-common-dir) || continue
  [ -n "${SEEN_COMMON[$common]:-}" ] && continue; SEEN_COMMON[$common]=1
  top=$(dirname "$common")
  say "repo $top"
  backup_branches "$d" "$(basename "$top")"
  while IFS= read -r wt; do
    [ -d "$wt" ] && [ -z "${SEEN_WT[$wt]:-}" ] || continue; SEEN_WT[$wt]=1
    n=$(basename "$wt"); [ -n "${SEEN_NAME[$n]:-}" ] && n="$n-$(basename "$top")"; SEEN_NAME[$n]=1
    backup_worktree "$wt" "$n"
  done < <(git -C "$d" worktree list --porcelain | sed -n 's/^worktree //p')
done < <([ $REPOS = 1 ] && find "$ROOT" -maxdepth 4 -name .git -not -path '*/target/*' -not -path "$STATE/*" 2>/dev/null | sort)

[ $NOTES = 1 ] && { say "notes"; backup_notes; }
echo "== summary"; [ ${#REPORT[@]} -gt 0 ] && printf '  %s\n' "${REPORT[@]}"; exit 0
