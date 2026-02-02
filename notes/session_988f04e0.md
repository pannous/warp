# Claude Conversation (988f04e0)

## 👤 Human

git cli Protect the main branch.

---

## 🤖 Assistant

→ `Bash`: `git remote -v`

---

## 🤖 Assistant

→ `Bash`: `gh api repos/pannous/warp/branches/main/protection -X PUT -f required_status_checks='null' -f enforc...`

---

## 🤖 Assistant

→ `Bash`: `gh api repos/pannous/warp/branches/main/protection -X PUT --input - <<'EOF'
{
  "required_status_che...`

---

## 🤖 Assistant

Done. Main branch is now protected:
- **Force pushes**: disabled
- **Branch deletion**: disabled

---

## 👤 Human

I hope branch deletion only affects the main branch because I want to delete other temporary branches.

---

## 🤖 Assistant

Yes, the protection only applies to `main`. You can freely delete any other branches. Branch protection rules are per-branch, so your temporary/feature branches remain unaffected.

---

