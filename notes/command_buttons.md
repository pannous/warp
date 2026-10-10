# Command buttons: the "run this command" box in Claude's apps

Research by warp-command-buttons, 2026-10-10. Sources: the claude.ai web bundles cached by the Claude desktop app
(`~/Library/Application Support/Claude/Cache/Cache_Data`, zstd-compressed JS from assets-proxy.anthropic.com),
the Claude Code 2.1.296 binary and changelog, and session transcripts under `~/.claude/projects/`.

## Summary

The box is not something Claude Code sends. The **app renders it from an ordinary markdown code block** in Claude's
reply, and adds the run button **only when the session it shows has a shell it can drive**:

| Session type (web client's name)                                     | Run button |
|----------------------------------------------------------------------|------------|
| Desktop app Code tab, session on this Mac (`local`)                  | yes        |
| Cloud session (`remote`), feature flag `ccr_cardboard_truck`         | maybe, flag-gated |
| **Remote Control**: terminal `claude --remote-control`, `claude-remote.sh` (`bridge`) | **never** |
| Local session over WSL, or SSH without the SSH-terminal feature      | no         |

So **in Remote Control it doesn't work**, whatever the message says. Our worker sessions are spawned with
`~/dev/bin/claude-remote.sh` (`claude --remote-control` in tmux), so they are Remote Control sessions, and that is why
warp-interviewer could not produce a box. The boxes the user saw came from desktop-app sessions. All 5 transcripts
with a clicked run (`<bash-input>` entries) carry `entrypoint: claude-desktop`, and each one comes right after an
assistant message with a ```` ```bash ```` block holding exactly that command
(`-Users-me-dev-angles-warp/f8959e98…`, `-Users-me/d2c34965…`, `-Users-me-dev-script-python/91958fe5…`, …).

## Message format (code block component in the web bundle)

The button appears on a **fenced code block**, never on inline code or plain text, when one of these is true:

1. The language tag is a shell: `bash`, `sh`, `shell`, `shellscript`, `zsh` (plus `powershell`/`ps1`/`pwsh` when
   the session's shell is PowerShell). Any content, multi-line included; the whole block runs.
2. The block is `text` (an untagged fence also counts) and its first line starts with `!`. The `!` and the blanks
   after it are stripped before running.
3. The block is `text`/untagged and holds one simple command: no `| & ; < > ( ) `` ` `` $ \`. Its first word (after an
   optional `sudo` and `VAR=value` prefixes) must start with `./` or be one of: aws brew bun bunx cargo cd chmod
   claude cp curl deno docker gcloud gh git grep kubectl ls mkdir mv node npm npx pip pip3 pnpm python python3 rg
   rm sed ssh tar uv wget yarn.

Even then there is no button when:
- the block contains something the app masks as a secret (the masked-secret view of the block is on), or
- the command contains hidden characters (invisible or bidi control characters) or some lookalike-character
  sequences (two regex checks in the bundle).

Clicking the button runs the command in the session's terminal pane as a user `!` command: it shows up as a
`<bash-input>` turn with `<bash-stdout>`/`<bash-stderr>`, and Claude responds to the output. The block also gets an
"Open in terminal" icon for the output.

The desktop composer's own `!` bash mode has the same limit: in a non-local session it refuses with "Shell commands
are only available in local sessions."

## Can a session produce it on purpose?

Yes, if the person is viewing a desktop-app session (Code tab on the Mac). Write the command as a ```` ```bash ````
fence on its own:

````markdown
Run this to delete the merged branch:

```bash
git -C /Users/me/dev/angles/warp push origin --delete release-sha
```
````

No tool call, prefix or special message type is involved; the same text in a Remote Control session renders as a plain
code block with a copy button only. In Remote Control the options are: ask the user to type `! <command>` in the
session's own terminal (tmux), or have the user run it themselves.

## Clients

- **Claude desktop app (macOS), Code tab:** yes, for sessions the app runs on this Mac (the `local` type).
- **claude.ai/code in a browser:** the same React code, but the browser has no local shell. It only gets a shell for
  cloud sessions with the `ccr_cardboard_truck` flag (not verified), and never for Remote Control (`bridge`).
- **iOS / Android apps:** native, not inspected. They run no shell of their own, so at most a cloud session behind the
  same flag could offer it. Not verified.

## Changelog

The Claude Code changelog doesn't mention the button: it lives in the claude.ai and desktop web client, not the CLI.
The CLI only has the `claude-cli://` deep link with its "This command came from a link… Run this command?" prompt,
and the `! <command>` bash mode in the terminal prompt.

## How to re-check (the web client changes often)

Extract the cached bundles (Chromium simple cache: 24-byte header, key, then a zstd body) and grep for the shell-language
set `"bash","sh","shell","shellscript","zsh"` next to the command-word list (the code block component imports it), and for
the shell transport function that branches on `type==="landing"|"local"|"remote"` (`bridge` falls through to none).
The extraction script used is in `scratch/` (ignored); about 1 GB of JS, of which a few files matter.
