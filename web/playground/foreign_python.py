# The Python side of foreign_call (src/foreign.rs, notes/stdlib_connectors.md): one JSON request, one JSON answer.
# Natively a python3 child reads requests from stdin; the browser playground runs it in Pyodide with
# WARP_BRIDGE_LOOP = False and calls answer() itself.
import sys, json, importlib, numbers, operator, types
handles = {}
operators = types.SimpleNamespace(**vars(operator), len=len, list=list)
def handle(value):
    handles[len(handles) + 1] = value
    return {"$handle": len(handles), "type": type(value).__name__, "text": repr(value)[:200]}
def unplain(value):
    if isinstance(value, dict):
        return handles[value["$handle"]] if "$handle" in value else {key: unplain(item) for key, item in value.items()}
    if isinstance(value, list):
        return [unplain(item) for item in value]
    return value
def plain(value):
    if isinstance(value, int) and not isinstance(value, bool) and not -2**63 <= value < 2**63:
        return {"$int": str(value)}
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    if isinstance(value, (list, tuple, set, frozenset)):
        return [plain(item) for item in value]
    if isinstance(value, dict):
        return {str(key): plain(item) for key, item in value.items()}
    if isinstance(value, numbers.Integral):
        return plain(int(value))
    if isinstance(value, numbers.Real) and not isinstance(value, numbers.Rational):
        return float(value)
    return handle(value)
def answer(line):
    """One request line in, one answer line out (the native loop below; Pyodide in web/playground/host.js)"""
    request = json.loads(line)
    try:
        module = request["module"]
        value = unplain(module) if isinstance(module, dict) else operators if module == "operator" else importlib.import_module(module)
        for part in request["member"].split("."):
            value = getattr(value, part)
        if request["arguments"] is not None:
            value = value(*unplain(request["arguments"]))
        reply = {"value": plain(value)}
    except BaseException as failure:
        reply = {"error": type(failure).__name__ + ": " + str(failure)}
    return json.dumps(reply)
if globals().get("WARP_BRIDGE_LOOP", True):
    # the answers own stdout: what a module prints goes to stderr
    answers, sys.stdout = sys.stdout, sys.stderr
    for line in sys.stdin:
        print(answer(line), file=answers, flush=True)
