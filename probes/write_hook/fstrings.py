# card write-hook: a Write of this content was reported to be denied by the claudeignore hook ("No stderr output")
import re

NUMBER = re.compile(r"(?P<int>\d+)(?:\.(?P<frac>\d*))?\s*$")
QUOTED = re.compile(r'"(?:[^"\\]|\\.)*"|\'[^\']*\'')

def show(name, value):
	print(f"{name!r:>12} = {value:08.3f}\t{{braces}} \\n \c {QUOTED.pattern}")
	return f'{name}: {"yes" if NUMBER.match(str(value)) else "no"}'

print(show("pi", 3.14159))
