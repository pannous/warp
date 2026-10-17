# Command buttons: the "run this command" box in Claude's apps

Source: the claude.ai web bundles cached by the Claude desktop app (`~/Library/Application
Support/Claude/Cache/Cache_Data`, zstd-compressed JS), checked against session transcripts.

## Summary

The box is not something Claude Code sends. The **app renders it from an ordinary markdown code block** in Claude's
reply, and adds the run button **only when the session it shows has a shell it can drive**:

| Session type (web client's name)                                     | Run button |
|----------------------------------------------------------------------|------------|
| Desktop app Code tab, session on this Mac (`local`)                  | yes        |
| Cloud session (`remote`), feature flag `ccr_cardboard_truck`         | maybe, flag-gated |
| **Remote Control**: terminal `claude --remote-control`, `claude-remote.sh` (`bridge`) | **never** |
| Local session over WSL, or SSH without the SSH-terminal feature      | no         |

So **in Remote Control it doesn't work**, whatever the message says. Our sessions are spawned with
`~/dev/bin/claude-remote.sh` (`claude --remote-control` in tmux), so they are Remote Control sessions. Clicked runs
show up in transcripts as `<bash-input>` entries with `entrypoint: claude-desktop`.

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

The button lives in the claude.ai and desktop web client, not the CLI (which has only the `claude-cli://` deep link
and the `! <command>` bash mode).

## How to re-check (the web client changes often)

Extract the cached bundles (Chromium simple cache: 24-byte header, key, then a zstd body) and grep for the shell-language
set `"bash","sh","shell","shellscript","zsh"` next to the command-word list (the code block component imports it), and for
the shell transport function that branches on `type==="landing"|"local"|"remote"` (`bridge` falls through to none).
About 1 GB of JS, of which a few files matter.
