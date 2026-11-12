#!/usr/bin/env bash
# scripts/lsp.warp, warp's language server (card language-server): a scripted session over its standard input checks
# what an editor gets back, go to definition and hover. Run it with bash test/tools/test_lsp.sh; the warp under test
# is WARP (default the warp on PATH), its lib LIB (default its own; a tree's: LIB=<tree>/lib)
warp=${WARP:-warp}
lib=${LIB:-$(dirname "$(readlink -f "$(command -v "$warp")")")}
server=$(dirname "$0")/../../scripts/lsp.warp
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0
check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}
framed() { # framed <json>: the message with its header
	printf 'Content-Length: %d\r\n\r\n%s' "$(printf '%s' "$1" | wc -c)" "$1"
}
asked() { # asked <method> <id> <params>
	framed "{\"jsonrpc\":\"2.0\",\"id\":$2,\"method\":\"$1\",\"params\":$3}"
}
mkdir -p "$stage/my folder"
file="$stage/my folder/shapes.warp"
printf 'use helpers\nsquare(x) := x * x\n\ncubed(x) := square(x) * x\nprint twice(3)\n' >"$file"
printf '# helpers\nlimit = 10\ntwice(n) := n * 2\n' >"$stage/my folder/helpers.warp"
uri="file://${file// /%20}"
at() { echo "{\"textDocument\":{\"uri\":\"$uri\"},\"position\":{\"line\":$1,\"character\":$2}}"; }
replies=$({
	asked initialize 1 '{"rootUri":null,"capabilities":{}}'
	asked textDocument/definition 2 "$(at 3 13)"
	asked textDocument/hover 3 "$(at 3 13)"
	asked textDocument/definition 4 "$(at 4 7)"
	asked textDocument/definition 5 "$(at 4 1)"
	asked shutdown 6 null
	framed '{"jsonrpc":"2.0","method":"exit"}'
} | WARP_WINDOW=0 "$warp" --lib "$lib" "$server" 2>&1)
reply() { # the body of the reply to id $1
	printf '%s' "$replies" | tr -d '\r' | grep -o "{\"jsonrpc\":\"2.0\",\"id\":$1,.*" | head -1
}
check "initialize names its capabilities" "$(reply 1 | grep -o '"definitionProvider":true')" '"definitionProvider":true'
check "square used on line 4 is defined on line 2" "$(reply 2 | grep -o '"start":{[^}]*}')" '"start":{"line":1,"character":0}'
check "hover shows the definition's line" "$(reply 3 | grep -o 'square(x) := x \* x')" 'square(x) := x * x'
check "twice is found in the used module beside it" "$(reply 4 | grep -o 'helpers.warp.*"start":{[^}]*}')" 'helpers.warp","range":{"start":{"line":2,"character":0}'
check "print has no definition here" "$(reply 5 | grep -o '"result":null')" '"result":null'
[ "$failures" = 0 ] || { printf '%s\n' "$replies" | head -20; exit 1; }
