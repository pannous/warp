# The Python C API lib/native/python.warp declares, answered in the playground's worker by Pyodide (host-foreign.js
# pythonLibrary): natively warp calls these functions of libpython. A PyObject * is a handle, an int naming the object
# in `objects` (0 is NULL), counted as libpython counts references; a call that raises gives its error result (NULL,
# -1) and keeps the exception for PyErr_Occurred and PyErr_GetRaisedException, as libpython does.
import importlib

objects = {}
counts = {}
handles = iter(range(1, 2**62))
raised = None
# Py_GetConstant's constants, by number (Python 3.13 Py_CONSTANT_NONE …)
CONSTANTS = [None, False, True, Ellipsis, NotImplemented, 0, 1, "", b"", ()]


def held(value):
    handle = next(handles)
    objects[handle] = value
    counts[handle] = 1
    return handle


def released(handle):
    counts[handle] -= 1
    if counts[handle] == 0:
        del objects[handle], counts[handle]


def guarded(failed):
    def guarding(function):
        def call(*arguments):
            global raised
            try:
                return function(*arguments)
            except BaseException as exception:
                raised = exception
                return failed
        return call
    return guarding


def as_long_long(target):
    value = int(objects[target])
    if not -2**63 <= value < 2**63:
        raise OverflowError("Python int too large to convert to C long")
    return value


def as_utf8(target):
    if not isinstance(objects[target], str):
        raise TypeError("bad argument type for built-in operation")
    return objects[target]


def stealing(container, position, item):
    objects[container][position] = objects[item]
    released(item)
    return 0


capi = {
    "Py_InitializeEx": lambda signals: None,
    "Py_IsInitialized": lambda: 1,
    "dlopen": lambda file, flags: 1,
    "Py_GetConstant": lambda constant: held(CONSTANTS[constant]),
    "PyImport_ImportModule": guarded(0)(lambda name: held(importlib.import_module(name))),
    "PyObject_GetAttrString": guarded(0)(lambda target, name: held(getattr(objects[target], name))),
    "PyObject_Call": guarded(0)(lambda function, arguments, keywords: held(objects[function](*objects[arguments], **(objects[keywords] if keywords else {})))),
    "PyObject_Str": guarded(0)(lambda target: held(str(objects[target]))),
    "PyObject_Type": lambda target: held(type(objects[target])),
    "PyObject_IsTrue": guarded(-1)(lambda target: int(bool(objects[target]))),
    "PyObject_IsInstance": guarded(-1)(lambda target, kind: int(isinstance(objects[target], objects[kind]))),
    "PyObject_Size": guarded(-1)(lambda target: len(objects[target])),
    "PySequence_GetItem": guarded(0)(lambda sequence, index: held(objects[sequence][index])),
    # a tuple filled before its call, kept as a list until PyObject_Call unpacks it
    "PyTuple_New": lambda size: held([None] * size),
    "PyTuple_SetItem": stealing,
    "PyList_New": lambda size: held([None] * size),
    "PyList_Append": lambda sequence, item: objects[sequence].append(objects[item]) or 0,
    "PyDict_New": lambda: held({}),
    "PyDict_SetItem": guarded(-1)(lambda mapping, key, value: objects[mapping].__setitem__(objects[key], objects[value]) or 0),
    "PyDict_Items": lambda mapping: held(list(objects[mapping].items())),
    "PyLong_FromLongLong": lambda value: held(int(value)),
    "PyLong_AsLongLong": guarded(-1)(as_long_long),
    "PyFloat_FromDouble": lambda value: held(float(value)),
    "PyFloat_AsDouble": guarded(-1.0)(lambda target: float(objects[target])),
    "PyBool_FromLong": lambda value: held(bool(value)),
    "PyUnicode_FromString": lambda value: held(value),
    "PyUnicode_AsUTF8": guarded(None)(as_utf8),
    "PyErr_Occurred": lambda: 0 if raised is None else 1,
    "Py_IncRef": lambda target: counts.__setitem__(target, counts[target] + 1),
    "Py_DecRef": released,
}


def raised_exception():
    global raised
    exception, raised = raised, None
    return 0 if exception is None else held(exception)


capi["PyErr_GetRaisedException"] = raised_exception
