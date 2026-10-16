#!/bin/bash
# `todo` must parse and run under the system Python 3.9 and every newer Homebrew Python.
set -u
TODO=~/dev/bin/todo
status=0
for python in /usr/bin/python3 /opt/homebrew/bin/python3.1[0-9]; do
  [ -x "$python" ] || continue
  if "$python" -I -c "import ast,sys; ast.parse(open(sys.argv[1]).read())" "$TODO" \
     && "$python" "$TODO" list >/dev/null; then
    echo "ok   $python"
  else
    echo "FAIL $python"; status=1
  fi
done
exit $status
