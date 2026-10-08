#!/usr/bin/env bash
# The language is called warp everywhere (user 2026-10-08, card rename-warp): wasp → warp, Wasp → Warp, WASP → WARP
# in every tracked text file and path (*.wasp → *.warp with git mv), run from the repository root. Idempotent: a second
# run changes nothing, so it can run on main and again on every incoming branch.
#   probes/rename_to_warp.sh           rename
#   probes/rename_to_warp.sh --check   list what a run would still change (exit 1 if anything), then the kept mentions
# Kept as written:
#   - lines naming the original: C++, `implementation of wasp`, `Wasp Wisp and Warp`; lines marked legacy; lines about
#     this rename itself
#   - lines accepting both names: a file name or quoted word with wasp whose warp form is on the same line
#     (`".wasp" || ".warp"`, `["wasp", "warp"]`, `sin.wasp`/`sin.warp`)
#   - URLs, domains and paths of the original: github.com/pannous/wasp, wasp.pannous.com, ~/wasp, homebrew-wasp
#   - notes/OLD (history) and this script
# wisp is a different word and stays.
set -euo pipefail

MODE="${1:-rename}"
EXCLUDED=(':!notes/OLD' ':!probes/rename_to_warp.sh')

KEPT_LINES='C\+\+|\blegacy\b|implementation of \W*wasp|Wasp,? Wisp|wasp remains|rename[-_ ]?(?:to[-_ ])?warp|named warp everywhere'
# a file name `x.wasp` or a quoted word `"wasp_main"`: kept with its line when its warp form is there too
BOTH_NAMES='\w*\.(?:wasp|Wasp|WASP)\b|["'"'"'][\w.-]*(?:wasp|Wasp|WASP)[\w.-]*["'"'"']'
KEPT_PARTS='(?:https?://)?(?:www\.)?github\.com/pannous/wasp\b[\w./#-]*|[\w.-]*wasp\.pannous\.com[\w./#-]*|pannous\.github\.io/wasp\b|~/wasp\b[\w./-]*|/Users/me/wasp\b|apps/wasp\b|homebrew-wasp'

# perl: every line of the files named on stdin (NUL separated); MODE check prints the lines it would change
convert_files() {
	KEPT_LINES="$KEPT_LINES" BOTH_NAMES="$BOTH_NAMES" KEPT_PARTS="$KEPT_PARTS" MODE="$MODE" perl -0 -ne '
		BEGIN { $changed = 0 }
		sub renamed { my $text = shift; $text =~ s/wasp/warp/g; $text =~ s/Wasp/Warp/g; $text =~ s/WASP/WARP/g; $text }
		sub converted {
			my $line = shift;
			return $line if $line =~ /$ENV{KEPT_LINES}/i;
			for my $word ($line =~ /($ENV{BOTH_NAMES})/g) {
				my $other = renamed($word);
				return $line if index($line, $other) >= 0;
			}
			my @kept;
			$line =~ s/($ENV{KEPT_PARTS})/push @kept, $1; "\x00" . $#kept . "\x00"/ge;
			$line = renamed($line);
			$line =~ s/\b(warp|Warp)\/\1\b/$1/g; # "wasp/warp" says one name now
			$line =~ s/\x00(\d+)\x00/$kept[$1]/g;
			$line;
		}
		chomp(my $file = $_);
		open(my $in, "<", $file) or die "$file: $!";
		my @lines = do { local $/ = "\n"; <$in> };
		close $in;
		my @new = map { converted($_) } @lines;
		my $differs = 0;
		for my $index (0 .. $#lines) {
			next if $lines[$index] eq $new[$index];
			$differs = 1;
			print "$file:" . ($index + 1) . ": $lines[$index]" if $ENV{MODE} eq "--check";
		}
		next unless $differs;
		$changed++;
		if ($ENV{MODE} ne "--check") { open(my $out, ">", $file) or die "$file: $!"; print $out @new; close $out }
		END { exit($ENV{MODE} eq "--check" && $changed ? 1 : 0) }
	'
}

renamed_path() {
	perl -pe 's/wasp/warp/g; s/Wasp/Warp/g; s/WASP/WARP/g' <<<"$1"
}

text_files_with_wasp() {
	git grep -I -l -i -z wasp -- . "${EXCLUDED[@]}" || true
}

paths_with_wasp() {
	git ls-files -- . "${EXCLUDED[@]}" | grep -i wasp || true
}

if [ "$MODE" = "--check" ]; then
	status=0
	text_files_with_wasp | convert_files || status=1
	while read -r path; do
		[ -n "$path" ] || continue
		echo "rename: $path -> $(renamed_path "$path")"
		status=1
	done <<<"$(paths_with_wasp)"
	echo "kept mentions: $(git grep -I -i -c wasp -- . "${EXCLUDED[@]}" | awk -F: '{sum += $NF} END {print sum + 0}')"
	exit $status
fi

text_files_with_wasp | convert_files
while read -r path; do
	[ -n "$path" ] || continue
	target="$(renamed_path "$path")"
	if [ -e "$target" ]; then
		echo "rename_to_warp: $target exists, $path stays" >&2
		continue
	fi
	mkdir -p "$(dirname "$target")"
	git mv "$path" "$target"
done <<<"$(paths_with_wasp)"
