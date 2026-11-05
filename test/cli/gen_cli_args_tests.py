# python3 test/cli/gen_cli_args_tests.py > test/modules/test_cli_args.pairs.warp: the expected values from argparse on the same argv (card lib-cli-args)
import argparse, json, sys, io, contextlib
def P():
    p=argparse.ArgumentParser(prog="prog",allow_abbrev=False)
    p.add_argument("-v","--verbose",action=argparse.BooleanOptionalAction,default=False)
    p.add_argument("-q","--quiet",action=argparse.BooleanOptionalAction,default=False)
    p.add_argument("-n","--count",type=int,default=1)
    p.add_argument("-r","--ratio",type=float,default=0.5)
    p.add_argument("-t","--tag",action="append",default=[])
    p.add_argument("--out",required=True)
    p.add_argument("files",nargs="*")
    return p
SPEC='[{name: "verbose", short: "v", kind: "flag"}, {name: "quiet", short: "q", kind: "flag"}, {name: "count", short: "n", kind: "int", default: 1}, {name: "ratio", short: "r", kind: "float", default: 0.5}, {name: "tag", short: "t", many: yes}, {name: "out", required: yes}, {name: "files", positional: yes, many: yes}]'
cases=[
 ["--out=x"], ["--out","x"], ["--out=x","-v"], ["--out=x","--verbose","--no-verbose"], ["--out=x","-vq"],
 ["--out=x","-n","3"], ["--out=x","-n3"], ["--out=x","-vn3"], ["--out=x","-vn","4"], ["--out=x","--count=12"],
 ["--out=x","--count","-5"], ["--out=x","-r","2.25"], ["--out=x","-t","a","--tag=b","-tc"], ["a","--out=x","b"],
 ["--out=x","--","-v","--count=2"], ["--out=x","-"], ["--out=","a"], ["--out=x","--out=y"],
 ["--zz"], ["--out"], [], ["--out=x","-n","x"], ["--out=x","--verbose=1"], ["--out=x","-k"], ["--out=x","--no-out"],
 ["--out=x","--verb"], ["--out=x","--count"],
]
print("// lib/cli.warp parse_args against python's argparse (card lib-cli-args): each expected value is what argparse.parse_args")
print("// gives on the same argv (test/cli/gen_cli_args_tests.py runs python3 argparse, allow_abbrev=False, BooleanOptionalAction, append, nargs=*).")
print("// Where both refuse, warp's message is its own (cli: …), argparse's is not compared. Intentional differences:")
print("// positionals may come between options in warp (argparse refuses a b split by an option: parse_intermixed_args")
print("// accepts it), so that case's value is warp's own, marked below")
def w(v):
    if isinstance(v,bool): return "yes" if v else "no"
    if isinstance(v,str): return json.dumps(v)
    if isinstance(v,list): return "["+", ".join(w(x) for x in v)+"]"
    return repr(v)
INTERMIXED=["a","--out=x","b"]
for argv in cases:
    if argv==INTERMIXED:
        with contextlib.redirect_stderr(io.StringIO()):
            ns=vars(P().parse_intermixed_args(argv))
        print("// intentional difference: argparse.parse_args refuses this, parse_intermixed_args gives:")
        a="["+", ".join(json.dumps(x) for x in argv)+"]"
        print(f"'use cli; parse_args({SPEC}, {a})'\t"+"{help: no, "+", ".join(k+": "+w(v) for k,v in ns.items())+"}")
        continue
    a="["+", ".join(json.dumps(x) for x in argv)+"]"
    try:
        with contextlib.redirect_stderr(io.StringIO()):
            ns=vars(P().parse_args(argv))
        exp="{help: no, "+", ".join(k+": "+w(v) for k,v in ns.items())+"}"
        print(f"'use cli; parse_args({SPEC}, {a})'\t{exp}")
    except SystemExit:
        print(f"'use cli; try {{ parse_args({SPEC}, {a})[\"help\"] }} catch e {{ e.message.starts_with(\"cli: \") }}'\tyes")
