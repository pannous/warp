#!/usr/bin/env python3
"""Exercise Warp's real CodeMirror mode without opening a browser."""

import json
import subprocess
from pathlib import Path


MODE_PATH = Path(__file__).parents[2] / "web/playground/codemirror-warp-mode.js"
NODE_PROBE = r"""
const vm = require("node:vm");
const source = process.argv[1];
let mode;
const context = {
    CodeMirror: { defineSimpleMode: (_name, specification) => { mode = specification; } },
    KEYWORDS: { hard: [], soft: [] },
};
vm.runInNewContext(source, context);

function anchored(regex) {
    return new RegExp(`^(?:${regex.source})`, regex.flags);
}

function tokens(line) {
    const found = [];
    while (line) {
        const rule = mode.start.find(candidate => anchored(candidate.regex).test(line));
        if (!rule) {
            line = line.slice(1);
            continue;
        }
        const text = anchored(rule.regex).exec(line)[0];
        if (rule.token) found.push({ text, token: rule.token });
        line = line.slice(text.length);
    }
    return found;
}

const cases = ["//note", "// note", "3/!4 //a note"].map(tokens);
process.stdout.write(JSON.stringify(cases));
"""


def main() -> None:
    result = subprocess.run(
        ["node", "-e", NODE_PROBE, MODE_PATH.read_text()],
        check=True,
        capture_output=True,
        text=True,
    )
    glued, spaced, after_code = json.loads(result.stdout)
    assert glued == [{"text": "//note", "token": "comment"}]
    assert spaced == [{"text": "// note", "token": "comment"}]
    assert {"text": "/!", "token": "operator"} in after_code
    assert after_code[-1] == {"text": "//a note", "token": "comment"}


if __name__ == "__main__":
    main()
