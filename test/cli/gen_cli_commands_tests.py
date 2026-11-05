# python3 test/cli/gen_cli_commands_tests.py > test/modules/test_cli_commands.pairs.warp: lib/cli.warp's choices and
# parse_commands against argparse subparsers on the same argv (card lib-cli-args-2)
import argparse, json, io, contextlib
def P():
    p = argparse.ArgumentParser(prog="todo", allow_abbrev=False)
    p.add_argument("-v", "--verbose", action=argparse.BooleanOptionalAction, default=False)
    p.add_argument("-f", "--file", default="todo.json")
    sub = p.add_subparsers(dest="command", required=True)
    a = sub.add_parser("add", allow_abbrev=False)
    a.add_argument("-p", "--priority", choices=["low", "mid", "high"], default="low")
    a.add_argument("-n", "--times", type=int, choices=[1, 2, 3], default=1)
    a.add_argument("title", nargs="*")
    d = sub.add_parser("done", allow_abbrev=False)
    d.add_argument("id", type=int)
    l = sub.add_parser("list", allow_abbrev=False)
    l.add_argument("-a", "--all", action=argparse.BooleanOptionalAction, default=False)
    l.add_argument("--sort", choices=["id", "title"], default="id")
    return p
G = '[{name: "verbose", short: "v", kind: "flag"}, {name: "file", short: "f", default: "todo.json"}]'
C = ('{add: [{name: "priority", short: "p", choices: ["low", "mid", "high"], default: "low"}, {name: "times", short: "n", kind: "int", choices: [1, 2, 3], default: 1}, {name: "title", positional: yes, many: yes}], '
     'done: [{name: "id", kind: "int", positional: yes}], '
     'list: [{name: "all", short: "a", kind: "flag"}, {name: "sort", choices: ["id", "title"], default: "id"}]}')
cases = [
    ["add", "buy", "milk"], ["-v", "add", "-p", "high", "buy", "milk"], ["add", "--priority=mid", "x"], ["add", "-n", "3", "x"],
    ["-f", "a.json", "done", "3"], ["--file=b.json", "-v", "done", "12"], ["list"], ["list", "-a", "--sort", "title"],
    ["list", "--no-all"], ["--no-verbose", "list"], ["add", "--", "-v", "x"], ["add"],
    [], ["zap"], ["add", "-p", "urgent", "x"], ["add", "-n", "4", "x"], ["add", "-n", "x", "y"], ["done"], ["done", "x"],
    ["list", "-v"], ["list", "--sort=date"], ["list", "extra"], ["-f"], ["--zz", "list"],
]
def w(v):
    if isinstance(v, bool): return "yes" if v else "no"
    if isinstance(v, str): return json.dumps(v)
    if isinstance(v, list): return "[" + ", ".join(w(x) for x in v) + "]"
    return repr(v)
print("// lib/cli.warp choices and parse_commands against python's argparse subparsers (card lib-cli-args-2): each expected")
print("// value is what argparse gives on the same argv (test/cli/gen_cli_commands_tests.py; dest=\"command\", required,")
print("// allow_abbrev=False). Where both refuse, warp's message is its own (cli: …), argparse's is not compared.")
for argv in cases:
    a = "[" + ", ".join(json.dumps(x) for x in argv) + "]"
    call = f"parse_commands({G}, {C}, {a})"
    try:
        with contextlib.redirect_stderr(io.StringIO()):
            ns = vars(P().parse_args(argv))
        print(f"'use cli; {call}'\t" + "{help: no, " + ", ".join(k + ": " + w(v) for k, v in ns.items()) + "}")
    except SystemExit:
        print(f"'use cli; try {{ {call}[\"help\"] }} catch e {{ e.message.starts_with(\"cli: \") }}'\tyes")
