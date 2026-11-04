#!/usr/bin/env python3
"""Generate test/modules/test_money.pairs.warp, every expected amount computed with python's decimal module.
Rerun from the repository root: python3 test/money/gen_money_tests.py"""
import os
from decimal import Decimal as D, ROUND_HALF_EVEN, ROUND_HALF_UP, ROUND_DOWN, ROUND_UP
from fractions import Fraction as F
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
lit = lambda s: "'" + s.replace("\\", "\\\\").replace("'", "\\'") + "'"
W = lambda t: '"' + t.replace("\\", "\\\\").replace('"', '\\"') + '"'
MINOR = {"EUR": 2, "USD": 2, "JPY": 0, "BHD": 3, "KWD": 3, "TND": 3, "CLF": 4}
MODES = {"half_even": ROUND_HALF_EVEN, "half_up": ROUND_HALF_UP, "down": ROUND_DOWN, "up": ROUND_UP}
L = ["// lib/money.warp: exact Money in minor units, rounding modes, allocation, mixed currencies, parsing and formatting;",
     "// the expected amounts from python's decimal module (test/money/gen_money_tests.py, card lib-money)"]
def case(code, exp): L.append(lit("use money; " + code) + "\t" + exp)
def text(code, amount):
    q = D(amount).quantize(D(1).scaleb(-MINOR[code])) if MINOR[code] else D(amount).quantize(D(1))
    return "%s %s" % (code, q + 0 if q == 0 else q)
def rounded(x, code, mode): return D(x).quantize(D(1).scaleb(-MINOR[code]), rounding=MODES[mode])
L.append("// the four rounding modes on ties, near-ties and negatives (python's ROUND_HALF_EVEN, HALF_UP, DOWN, UP)")
for amount, code in [("0.125", "EUR"), ("0.135", "EUR"), ("-0.125", "EUR"), ("-0.135", "EUR"), ("2.675", "EUR"), ("-2.675", "EUR"), ("1.001", "EUR"), ("-1.001", "EUR"), ("0.005", "EUR"), ("-0.009", "EUR"), ("2.5", "JPY"), ("3.5", "JPY"), ("-2.5", "JPY"), ("1.0005", "BHD"), ("-1.0015", "KWD"), ("0.00005", "CLF")]:
    modes = list(MODES)
    got = [text(code, rounded(amount, code, m)) for m in modes]
    case("[money_of(%s, %s, m).text() for m in %s]" % (W(amount), W(code), "[" + ", ".join(W(m) for m in modes) + "]"), "[" + ", ".join(W(g) for g in got) + "]")
L.append("// exact amounts: ints, fractions and decimal texts; one finer than the minor unit raises without a mode")
case("money_of(12.34, \"EUR\").text()", W("EUR 12.34"))
case("money_of(5, \"JPY\").text()", W("JPY 5"))
case("money_of(\"1.234\", \"TND\").minor", "1234")
case("money_of(1/4, \"CLF\").text()", W("CLF 0.2500"))
case("try { money_of(\"0.125\", \"EUR\") } catch e { e.message }", W("money: 0.125 is finer than EUR's 2 decimals; pass a rounding mode"))
case("try { money_of(1, \"XYZ\") } catch e { e.message }", W("money: XYZ is no ISO 4217 currency of lib/money.warp"))
case("try { money_of(1, \"EUR\", \"sideways\") } catch e { e.message }", W("money: the rounding mode is half_even, half_up, down or up, not sideways"))
case("[currency_minor_units(c) for c in [\"JPY\", \"EUR\", \"BHD\", \"KWD\", \"TND\", \"CLF\"]]", "[0, 2, 3, 3, 3, 4]")
L.append("// + - and scalar *, and mixed currencies refused")
case("(money_of(10.05, \"EUR\") + money_of(0.95, \"EUR\") - money_of(3, \"EUR\")).text()", W(text("EUR", D("10.05") + D("0.95") - 3)))
case("(Money(1001, \"EUR\") * 3).text()", W(text("EUR", D("10.01") * 3)))
for k, mode in [(F(1, 2), "half_even"), (F(1, 2), "half_up"), (F(1, 3), "down"), (F(1, 3), "up"), (F(-1, 2), "half_even")]:
    exact = F(1001) * k
    expect = (D(exact.numerator) / D(exact.denominator)).quantize(D(1), rounding=MODES[mode]) / 100
    case("money_times(Money(1001, \"EUR\"), %d/%d, %s).text()" % (k.numerator, k.denominator, W(mode)), W(text("EUR", expect)))
case("(Money(1005, \"EUR\") * 0.5).text()", W(text("EUR", (D("10.05") * D("0.5")).quantize(D("0.01"), rounding=ROUND_HALF_EVEN))))
case("money_add(money_of(1, \"USD\"), money_of(2, \"USD\")) == money_of(3, \"USD\")", "true")
case("[money_of(1, \"EUR\") < money_of(2, \"EUR\"), money_of(-1, \"EUR\") > money_of(2, \"EUR\")]", "[true, false]")
case("try { money_of(1, \"EUR\") + money_of(1, \"USD\") } catch e { e.message }", W("money: EUR + USD mixes currencies; convert one first"))
case("try { money_sub(money_of(1, \"EUR\"), money_of(1, \"JPY\")) } catch e { e.message }", W("money: EUR - JPY mixes currencies; convert one first"))
case("try { money_of(1, \"EUR\") < money_of(1, \"GBP\") } catch e { e.message }", W("money: EUR compared with GBP mixes currencies; convert one first"))
case("try { money_of(1, \"EUR\") * 2.5e0 } catch e { e.message }", W("money: 2.5 is a float, not an exact number: write it as an int or a fraction (0.1, 1/3)"))
case("try { Money(4611686018427387903, \"EUR\") + Money(1, \"EUR\") } catch e { e.message }", W("money: 4611686018427387903 + 1 minor units is too large"))
L.append("// allocation: largest remainders, the parts always summing back (python's Fraction for the shares)")
def allocate(minor, ratios):
    sign = -1 if minor < 0 else 1; a = abs(minor); tot = sum(F(r) for r in ratios)
    ex = [F(a) * F(r) / tot for r in ratios]; parts = [int(e) for e in ex]; left = a - sum(parts)
    rest = [e - int(e) for e in ex]
    while left:
        b = max(range(len(rest)), key=lambda i: (rest[i], -i)); parts[b] += 1; rest[b] = -1; left -= 1
    return [p * sign for p in parts]
def wr(r): return str(r) if isinstance(r, int) else "%d/%d" % (r.numerator, r.denominator)
for minor, ratios in [(100, [1, 1, 1]), (5, [1, 1, 1]), (-100, [1, 1, 1]), (1001, [1, 2, 3]), (1000, [F(1, 3), F(2, 3)]), (99, [70, 20, 10]), (-1001, [1, 2, F(3, 2)]), (1, [1, 1]), (0, [3, 7])]:
    parts = allocate(minor, ratios); assert sum(parts) == minor
    case("[p.minor for p in money_allocate(Money(%d, \"EUR\"), [%s])]" % (minor, ", ".join(wr(r) for r in ratios)), str(parts))
for minor, n in [(100, 3), (1000, 7), (-7, 4), (10, 1)]:
    parts = allocate(minor, [1] * n)
    case("[p.minor for p in money_split(Money(%d, \"JPY\"), %d)]" % (minor, n), str(parts))
case("s = money_allocate(Money(123457, \"BHD\"), [1, 1, 1, 1, 1, 1, 1]); m = Money(0, \"BHD\"); for p in s { m = m + p }; m.text()", W("BHD 123.457"))
case("try { money_allocate(Money(5, \"EUR\"), [1, -1]) } catch e { e.message }", W("money: a negative ratio -1/1"))
L.append("// parsing en and de amounts, symbols or codes, negatives, JPY and BHD")
for t, code, amount in [("€1.234,56", "EUR", "1234.56"), ("$1,234.56", "USD", "1234.56"), ("1.234,56 €", "EUR", "1234.56"), ("-€0,50", "EUR", "-0.50"), ("($1,234.56)", "USD", "-1234.56"), ("USD 12", "USD", "12.00"), ("¥1,235", "JPY", "1235"), ("-1.235 ¥", "JPY", "-1235"), ("BHD 1,234.567", "BHD", "1234.567"), ("1.234,567 BHD", "BHD", "1234.567"), ("BHD 1.234", "BHD", "1.234"), ("€1.234.567", "EUR", "1234567.00"), ("CA$5.5", "CAD", "5.50"), ("£0.99", "GBP", "0.99"), ("1 234,50 EUR", "EUR", "1234.50")]:
    MINOR.setdefault(code, 2)
    case("money_parse(%s).text()" % W(t), W(text(code, amount)))
case("money_parse(\"12,345\", {style: \"en\", currency: \"EUR\"}).text()", W("EUR 12345.00"))
case("money_parse(\"12,345\", {style: \"de\", currency: \"BHD\"}).text()", W("BHD 12.345"))
case("try { money_parse(\"12.345\") } catch e { e.message }", W("money_parse: 12.345 names no currency; pass {currency: …}"))
case("try { money_parse(\"€1,234\", {style: \"en\", currency: \"EUR\"}).text() } catch e { e.message }", W("EUR 1234.00"))
case("try { money_parse(\"€1.2345\", {style: \"en\"}) } catch e { e.message }", W("money_parse: €1.2345 has 4 decimals, EUR has 2"))
L.append("// formatting en and de")
def fmt(minor, code, style, symbol):
    q = D(minor).scaleb(-MINOR[code]); s = "{:,.%df}" % MINOR[code]; n = s.format(abs(q))
    if style == "de": n = n.replace(",", "_").replace(".", ",").replace("_", ".")
    sign = "-" if minor < 0 else ""
    if style == "en": return sign + (symbol + " " if symbol.replace(" ", "").isalpha() and symbol.isupper() else symbol) + n
    return sign + n + " " + symbol
for minor, code, symbol in [(123456, "EUR", "€"), (-123456, "EUR", "€"), (5, "USD", "$"), (123456789, "USD", "$"), (1235, "JPY", "¥"), (-1235, "JPY", "¥"), (1234567, "BHD", "BHD"), (-1, "KWD", "KWD"), (12345, "CLF", "CLF"), (0, "EUR", "€")]:
    for style in ["en", "de"]:
        case("money_format(Money(%d, %s), %s)" % (minor, W(code), W(style)), W(fmt(minor, code, style, symbol)))
case("money_format(money_parse(\"€1.234,56\"))", W("€1,234.56"))
case("money_format(money_parse(\"$1,234.56\"), \"de\")", W("1.234,56 $"))
case("money_round(Money(123456, \"EUR\"), 0, \"half_even\").text()", W("EUR 1235.00"))
open(os.path.join(root, "test", "modules", "test_money.pairs.warp"), "w", encoding="utf-8").write("\n".join(L) + "\n")
print("test_money:", sum(1 for l in L if l.startswith("'")), "cases")
