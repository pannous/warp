"""Doc comment blocks where a line starts a new summary right after a finished one: often a doc line left behind when
its function moved away, now stacked on top of another function's doc. Prints candidates to review by hand.
Usage: python3 probes/cleanup/stacked_docs.py <folders...>"""
import glob, re, sys

CONTINUES = (',', ':', '(', '—', '-', '/')

for folder in sys.argv[1:]:
	for path in sorted(glob.glob(folder.rstrip('/') + '/**/*.rs', recursive=True)):
		lines = open(path).read().split('\n')
		start = 0
		while start < len(lines):
			if not lines[start].strip().startswith('///'):
				start += 1
				continue
			end = start
			while end < len(lines) and lines[end].strip().startswith('///'):
				end += 1
			block = [line.strip()[3:].strip() for line in lines[start:end]]
			for index in range(1, len(block)):
				previous, current = block[index - 1], block[index]
				if previous and current and not previous.endswith(CONTINUES) and re.match(r'^[A-Z`]', current):
					print(f"{path}:{start + index + 1}: …{previous[-50:]!r} || {current[:60]!r}")
			start = end
