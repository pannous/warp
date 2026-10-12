#!/bin/sh
# card sound-gain-ramps: host-files.js fadedGain against the values src/sound.rs faded_gain gives
cd "$(dirname "$0")/../.." && node - <<'EOF'
const source = require("fs").readFileSync("web/playground/host-files.js", "utf8");
const fadedGain = new Function(`${source.match(/function fadedGain[\s\S]*?\n}\n/)[0]}; return fadedGain;`)();
const fades = [{ place: 1, gain: 0, seconds: 2 }, { place: 2, gain: 1, seconds: 1 }];
const expected = [[0, 1], [1, 1], [1.5, 0.75], [2, 0.5], [2.5, 0.75], [3, 1], [9, 1]];
for (const [place, gain] of expected) console.log(place, fadedGain(fades, place), Math.abs(fadedGain(fades, place) - gain) < 1e-9 ? "ok" : `WRONG, expected ${gain}`);
EOF
