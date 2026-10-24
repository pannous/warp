#!/usr/bin/env python3
"""The shortest published solutions (chars) of code.golf holes in Python, Ruby and JavaScript, from the leaderboards:
prints a markdown table for notes/code_golf.md and saves the hole descriptions to data/golf/holes.json"""
import re, sys, time, urllib.error, urllib.parse, urllib.request
from pathlib import Path

HOLES = ["fizz-buzz", "fibonacci", "prime-numbers", "99-bottles-of-beer", "pascals-triangle", "quine", "arabic-to-roman",
	"divisors", "happy-numbers", "leap-years", "catalan-numbers", "sierpiński-triangle", "diamonds", "collatz",
	"look-and-say", "triangular-numbers", "evil-numbers", "niven-numbers", "pernicious-numbers", "rot13"]
LANGUAGES = ["python", "ruby", "javascript"]
RANKINGS = "https://code.golf/rankings/holes/{hole}/{language}/chars"
PAUSE_SECONDS = 3  # code.golf answers 429 to faster crawling
DATA = Path(__file__).resolve().parent.parent / "data" / "golf"


def fetch(url, attempts=5):
	time.sleep(PAUSE_SECONDS)
	try:
		return urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": "warp-golf"}), timeout=30).read().decode()
	except urllib.error.HTTPError as error:
		if error.code != 429 or attempts == 1:
			raise
		time.sleep(30)
		return fetch(url, attempts - 1)


def best_chars(hole, language):
	page = fetch(RANKINGS.format(hole=urllib.parse.quote(hole), language=language))
	body = page[page.find("<tbody"):]
	cells = [cell.strip() for cell in re.sub(r"<[^>]+>", "|", body).split("|") if cell.strip()]
	# a row reads: rank, ordinal, name, [flag], points, chars, time, date: the first plain number after a "1,000" points cell
	for index, cell in enumerate(cells):
		if re.fullmatch(r"[\d,]+", cell) and index and cells[index - 1].replace(",", "").isdigit() and int(cells[index - 1].replace(",", "")) >= 1000:
			return int(cell.replace(",", ""))
	return None


def main():
	DATA.mkdir(parents=True, exist_ok=True)
	(DATA / "holes.json").write_text(fetch("https://code.golf/api/holes"))
	print("| hole | " + " | ".join(LANGUAGES) + " |")
	print("|---|" + "---|" * len(LANGUAGES))
	for hole in HOLES:
		print(f"| {hole} | " + " | ".join(str(best_chars(hole, language)) for language in LANGUAGES) + " |", flush=True)


if __name__ == "__main__":
	sys.exit(main())
