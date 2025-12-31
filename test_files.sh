#!/bin/bash
cd "$(dirname "$0")"
for file in samples/*.wasp; do
  echo -n "Testing $file... "
  timeout 2 cargo run --quiet -- --parse "$file" > /dev/null 2>&1
  if [ $? -eq 124 ]; then
    echo "TIMEOUT - this file hangs!"
  elif [ $? -eq 0 ]; then
    echo "OK"
  else
    echo "ERROR"
  fi
done
