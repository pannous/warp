#!/usr/bin/env bash
# core/native/warp.warp, the native warp (decision native-warp-in-core), built by `warp build native`: it runs a file
# with its arguments, a module, code, and warp.wasm's own commands (`warp build`). Run it with
# bash test/native/test_warp_native.sh; the warp that builds it is WARP (default the warp on PATH), the native warp's
# lib/ WARP_LIB as usual
checkout=$(cd "$(dirname "$0")/../.." && pwd)
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0

check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}

cp "$checkout"/core/native/*.warp "$checkout/samples/hello/hello.warp" "$stage/"
(cd "$stage" && "$warp" build native warp.warp >"$stage/build.log" 2>&1)
if [ -x "$stage/warp" ]; then
	native() { (cd "$stage" && ./warp "$@" 2>&1); }
	check "a file" "$(native hello.warp)" "Hello 🌍2026"
	printf 'use os\nprint args\n' >"$stage/arguments.warp"
	check "a file's arguments" "$(native arguments.warp eins zwei)" '["eins" "zwei"]'
	(cd "$stage" && "$warp" build --wasm hello.warp >/dev/null 2>&1)
	check "a module" "$(native hello.wasm)" "Hello 🌍2026"
	check "code" "$(native '3 + 3')" 6
	check "help, warp.wasm's command" "$(native help | head -1)" "warp, the compiler written in warp (core/compiler.warp), its commands:"
	mkdir "$stage/built" && cp "$stage/hello.warp" "$stage/built/"
	(cd "$stage/built" && ../warp build hello.warp >/dev/null 2>&1)
	check "warp build by the native warp" "$("$stage/built/hello" 2>&1)" "Hello 🌍2026"
	check "run, a file" "$(native run hello.warp)" "Hello 🌍2026"
	check "the arguments after --" "$(native arguments.warp -- eins zwei)" '["eins" "zwei"]'
	check "self, code by warp-self.wasm" "$(native self '3 + 4')" 7
	printf 'raise "boom"\n' >"$stage/raising.warp"
	check "what a program raised" "$(native raising.warp)" "boom at raising.warp:1:1"
	check "the console, its lines staying" "$(printf 'a = 7\nx = [1\na * 6\n' | native repl | sed -n 4p)" "🌀42"
	check "bare warp, a piped program" "$(printf 'a = 7\na * 6\n' | native)" 42
	check "exit(code) ends the run" "$(native 'exit(3)'; echo $?)" 3
	printf 'x  =  1\n' >"$stage/spaced.warp"
	check "format check" "$(native format check spaced.warp | head -1)" "spaced.warp:1: layout: spaces"
	check "eval, code" "$(native eval '3 * 4')" 12
	check "a command the native warp lacks yet" "$(native verify 1)" "warp verify is not in the native warp yet: warp-rust verify runs it"
	mkdir "$stage/below"
	check "a file above the current folder" "$(cd "$stage/below" && ../warp ../hello.warp 2>&1)" "Hello 🌍2026"
	check "a command's path above the current folder" "$(cd "$stage/below" && ../warp compile ../hello.warp ../above.wasm >/dev/null 2>&1; ls "$stage/above.wasm" 2>&1)" "$stage/above.wasm"
	printf 'i = 0\nwhile yes { i += 1 }\n' >"$stage/endless.warp"
	check "fuel, out of it" "$(native fuel 1_000_000 endless.warp | cut -c1-31)" "out of fuel after 1000000 steps"
	check "rebuild of no compiler" "$(native rebuild nonsense | cut -c1-34)" "warp rebuild: no compiler nonsense"
	check "sleep_ms and clock" "$(native 'c = clock(); sleep_ms(50); clock() - c >= 50')" yes
	check "random, the same after the same seed" "$(native 'random_seed(7); a = random(); random_seed(7); a == random() and a < 1')" yes
	check "regex replace with groups" "$(native 'use regex; replace_all("john smith", "(\\w+) (\\w+)", "$2 $1")')" "smith john"
	check "regex all" "$(native 'use regex; find_all("x12y345", "[0-9]+")')" '["12" "345"]'
	check "sha256" "$(native 'use hash; sha256("abc")')" ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
	check "crc32" "$(native 'use hash; crc32("abc")')" 891568578
	check "a system value of a library's" "$(native 'battery()')" "the system value battery is not in the native warp yet (warp-rust has it)"
	# an install as ~/dev/bin has it: bin/warp and bin/lib links to a build; what that warp builds keeps running after
	# the next install moves bin/lib to another build and removes the old one
	library=$(cd "${WARP_LIB:-$(dirname "$(command -v "$warp")")}" && pwd -P)
	mkdir -p "$stage/install/bin" "$stage/install/app" && ln -s "$library" "$stage/install/old" && ln -s ../old "$stage/install/bin/lib"
	ln -s "$stage/warp" "$stage/install/bin/warp" && cp "$checkout"/core/native/*.warp "$stage/install/app/"
	(cd "$stage/install/app" && env -u WARP_LIB ../bin/warp build native warp.warp >/dev/null 2>&1)
	ln -s "$library" "$stage/install/new" && ln -sfn ../new "$stage/install/bin/lib" && rm "$stage/install/old"
	played=$(native 'use sound; play(440Hz, 0.5s)')
	check "a sound, its line" "$(sed -n 1p <<<"$played" | cut -d: -f1)" "sound 0.50 s"
	wav=$(sed -n 1p <<<"$played" | cut -d' ' -f4-)
	check "a sound, its WAV" "$(head -c 4 "$wav"; wc -c <"$wav" | tr -d ' ')" RIFF22094
	check "a sound saved, loaded and saved with another" "$(native $'use sound\ntone(440Hz, 0.5s)\nrender_sound("a.wav")\nkick = sample("a.wav")\nplay_sample(kick, 2)\ntone(440Hz, 0.5s)\n[kick.rate, kick.channels, round(render_sound("b.wav") * 100)]' | tail -1)" "[22050 1 75]"
	check "a sound at its time in the save" "$(native $'use sound\ntone(440Hz, 0.5s)\nat 2s { tone(880, 0.5s) }\nround(render_sound("c.wav") * 100)' | tail -1)" 250
	printf "posix_spawnp(pid: out, file: text, actions: handle, attributes: handle, arguments: texts, environment: handle): i32 from 'c'\nposix_spawnp('echo', 0, 0, ['echo', 'eins', 'zwei'], 0)\n" >"$stage/spawning.warp"
	check "a list of texts as a C argv" "$(native spawning.warp | sort | tr '\n' ' ')" "0 eins zwei "
	# a recorder as the player: nothing sounds even if WARP_SOUND_PLAYER were ignored, WARP_NO_WINDOW mutes the others
	printf '#!/bin/sh\necho "$0 ${1##*-}" >>"%s/played.log"\n' "$stage" >"$stage/recorder.sh" && chmod +x "$stage/recorder.sh"
	paced=$(WARP_SOUND_PLAYER="$stage/recorder.sh" native $'use sound\nc = clock()\ntone(440Hz, 0.5s)\ntone(880Hz, 0.5s)\nwait_sound()\nclock() - c >= 1000' | tail -1)
	sleep 0.2
	check "sounds through the player, each when its voice reaches it" "$paced $(wc -l <"$stage/played.log" | tr -d ' ') $(sed -n 2p "$stage/played.log")" "yes 2 $stage/recorder.sh 2.wav"
	check "a signal, sample by sample" "$(native $'use sound\nhum(t: float) := 0.5 * sin(2 * pi * 220 * t)\nplay(hum, 0.5s)' 2>&1 | head -1 | cut -d: -f1)" "sound 0.50 s"
	check "a signal in blocks" "$(native $'use sound\nhum(block: samples, t) := for i in 1 to #block { block#i = 0.3 }\nplay(hum, 0.1s)' 2>&1 | head -1 | cut -d: -f1)" "sound 0.10 s"
	check "a sound file" "$(cd "$stage" && ./warp $'use sound\nplay "a.wav"' 2>&1 | head -1)" "sound file: a.wav"
	check "the view outside a window: the terminal's, else COLUMNS and LINES" "$(COLUMNS=133 LINES=41 native '[view_width(), view_height()]')" "[133 41]"
	check "the view of a terminal" "$(script -q /dev/null sh -c 'stty cols 101 rows 31; cd "$1" && ./warp "[view_width(), view_height()]"' sh "$stage" </dev/null | grep -o '\[[0-9]* [0-9]*\]')" "[101 31]"
	painting=$(native 'paint([0, 1, 0xFF00FF00, 0], 2, 2)')
	check "paint, a PNG" "${painting%%:*} $(file -b "${painting#*: }")" "painted 2×2 PNG image data, 2 x 2, 8-bit/color RGB, non-interlaced"
	check "paint of a shader" "$(native 'paint("fn x", 2, 2, ø)')" "paint of a shader: the native warp has no GPU yet (warp-rust renders it)"
	check "exit() ends the run well" "$(native $'print 1\nexit()\nprint 2'; echo $?)" "1
0"
	# never called, so no window even if the refusal failed
	printf 'import InitWindow from "raylib"\nprint "reached"\n' >"$stage/window.warp"
	check "a window opener under WARP_NO_WINDOW" "$(native window.warp)" "InitWindow from raylib opens a window: refused under WARP_NO_WINDOW or CI"
	# an installed lib/ this user can't write: the machine code goes to the user's cache folder, a file only read opens
	(cp -Rc "$library" "$stage/readonly" 2>/dev/null || cp -R "$library" "$stage/readonly") && rm -f "$stage"/readonly/*.cwasm* && chmod a-w "$stage/readonly"
	check "a read-only lib/, its machine code in the user's cache" "$(XDG_CACHE_HOME="$stage/cache" WARP_LIB="$stage/readonly" native '6 * 7'; ls "$stage/cache/warp" | grep -c 'cwasm$')" "42
1"
	chmod u+w "$stage/readonly"
	printf hi >"$stage/kept.txt" && chmod a-w "$stage/kept.txt"
	check "a read-only file read" "$(native "use file; read('$stage/kept.txt')")" hi
	check "window and display values set and read" "$(native $'window.width = 800\ndisplay.density = 300 dpi\n[window.width, window.height, 1inch * display.density == 300px, online == true or online == false]')" "[800 480 yes yes]"
	check "a channel's values sent in tasks" "$(native 'ch = channel(); go { for i in 1 to 3 { ch.send(i) }; ch.close() }; total = 0; for v in ch { total += v }; total')" 6
	check "json written and read" "$(native $'class P{x:int y:int}\n[to_json(P(1, 2)), parse_json("[1, null]")]')" '["{\"x\":1,\"y\":2}" [1 ø]]'
	check "a built executable after the next install" "$(cd "$stage/install/app" && WARP_LIB="$stage/install/bin/lib" ./warp '6 * 7' 2>&1)" 42
else
	echo "✗ warp: no executable"; head -5 "$stage/build.log"; failures=$((failures + 1))
fi
exit $((failures > 0))
