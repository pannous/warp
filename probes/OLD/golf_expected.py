#!/usr/bin/env python3
"""The expected output of each code.golf hole of samples/golf/, written as samples/golf/<hole>.txt (reference solutions)"""
from math import comb
from pathlib import Path

GOLF = Path(__file__).resolve().parent.parent / "samples" / "golf"


def lines(values):
	return "\n".join(map(str, values)) + "\n"


def fizz_buzz():
	return lines("Fizz" * (n % 3 == 0) + "Buzz" * (n % 5 == 0) or n for n in range(1, 101))


def fibonacci():
	numbers = [0, 1]
	while len(numbers) < 31:
		numbers.append(numbers[-1] + numbers[-2])
	return lines(numbers)


def is_prime(n):
	return n > 1 and all(n % d for d in range(2, n))


def prime_numbers():
	return lines(n for n in range(1, 101) if is_prime(n))


def bottles(n):
	return f"{n or 'no more'} bottle{'s' * (n != 1)} of beer"


def ninety_nine_bottles_of_beer():
	verses = [f"{bottles(n)} on the wall, {bottles(n)}.\nTake one down and pass it around, {bottles(n - 1)} on the wall."
		for n in range(99, 0, -1)]
	verses.append("No more bottles of beer on the wall, no more bottles of beer.\n"
		"Go to the store and buy some more, 99 bottles of beer on the wall.")
	return "\n\n".join(verses) + "\n"


def pascals_triangle():
	return lines(" ".join(str(comb(row, k)) for k in range(row + 1)) for row in range(20))


def divisors():
	return lines(" ".join(str(d) for d in range(1, n + 1) if n % d == 0) for n in range(1, 101))


def is_happy(n):
	seen = set()
	while n != 1 and n not in seen:
		seen.add(n)
		n = sum(int(digit) ** 2 for digit in str(n))
	return n == 1


def happy_numbers():
	return lines(n for n in range(1, 201) if is_happy(n))


def leap_years():
	return lines(year for year in range(1800, 2401) if year % 4 == 0 and (year % 100 or year % 400 == 0))


def catalan_numbers():
	return lines(comb(2 * n, n) // (n + 1) for n in range(100))


def sierpinski_triangle():
	rows = ["▲"]
	for _ in range(4):
		rows = [" " * len(rows) + row + " " * len(rows) for row in rows] + [row + " " + row for row in rows]
	return lines(row.rstrip() for row in rows)


LARGEST_DIAMOND = 9


def diamond(size):
	row = lambda width: " " * (LARGEST_DIAMOND - width) + "".join(map(str, range(1, width + 1))) + "".join(map(str, range(width - 1, 0, -1)))
	return "\n".join(row(width) for width in [*range(1, size + 1), *range(size - 1, 0, -1)])


def diamonds():
	return "\n\n".join(diamond(size) for size in range(1, LARGEST_DIAMOND + 1)) + "\n"


def stopping_time(n):
	steps = 0
	while n != 1:
		n = n // 2 if n % 2 == 0 else 3 * n + 1
		steps += 1
	return steps


def collatz():
	return lines(stopping_time(n) for n in range(1, 1001))


def look_and_say():
	terms = ["1"]
	while len(terms) < 20:
		term, said, start = terms[-1], "", 0
		while start < len(term):
			end = start
			while end < len(term) and term[end] == term[start]:
				end += 1
			said += f"{end - start}{term[start]}"
			start = end
		terms.append(said)
	return lines(terms)


def ones(n):
	return bin(n).count("1")


def evil_numbers():
	return lines(n for n in range(51) if ones(n) % 2 == 0)


def niven_numbers():
	return lines(n for n in range(1, 101) if n % sum(map(int, str(n))) == 0)


def pernicious_numbers():
	return lines(n for n in range(51) if is_prime(ones(n)))


ROMAN_ARGUMENTS = ["1", "4", "9", "14", "40", "90", "400", "1994", "2024", "3999"]
ROT13_ARGUMENTS = ["Hello, World!", "The Quick Brown Fox Jumps Over The Lazy Dog.", "warp 123"]
ROMAN_SYMBOLS = [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"),
	(10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")]


def roman(n):
	numeral = ""
	for value, symbol in ROMAN_SYMBOLS:
		numeral += symbol * (n // value)
		n %= value
	return numeral


def arabic_to_roman():
	return lines(roman(int(argument)) for argument in ROMAN_ARGUMENTS)


def rot13_letter(c):
	for first in "aA":
		if first <= c <= chr(ord(first) + 25):
			return chr(ord(first) + (ord(c) - ord(first) + 13) % 26)
	return c


def rot13():
	return lines("".join(map(rot13_letter, argument)) for argument in ROT13_ARGUMENTS)


ARGUMENTS = {"arabic-to-roman": ROMAN_ARGUMENTS, "rot13": ROT13_ARGUMENTS}

HOLES = {"arabic-to-roman": arabic_to_roman, "rot13": rot13, "fizz-buzz": fizz_buzz, "fibonacci": fibonacci, "prime-numbers": prime_numbers,
	"99-bottles-of-beer": ninety_nine_bottles_of_beer, "pascals-triangle": pascals_triangle, "divisors": divisors,
	"happy-numbers": happy_numbers, "leap-years": leap_years, "catalan-numbers": catalan_numbers,
	"sierpiński-triangle": sierpinski_triangle, "diamonds": diamonds, "collatz": collatz, "look-and-say": look_and_say,
	"evil-numbers": evil_numbers, "niven-numbers": niven_numbers, "pernicious-numbers": pernicious_numbers}

if __name__ == "__main__":
	GOLF.mkdir(parents=True, exist_ok=True)
	for hole, output in HOLES.items():
		(GOLF / f"{hole}.txt").write_text(output())
	for hole, arguments in ARGUMENTS.items():
		(GOLF / f"{hole}.args").write_text("\n".join(arguments) + "\n")
