#!/usr/bin/env bash
# wap's modules: `use "file"` brings in a file's definitions and runs its statements first (notes/backbone.md "Stage
# languages", declared; core/analyze/declarations.wap). Run it with bash test/wap/test_modules.sh
source "$(dirname "$0")/compile.sh"

printf '%s\n' 'say(t: text) := print(t)' 'say("loaded")' >"$stage/speak.wap"
printf '%s\n' 'use "speak.wap"' 'greeting = "hi"' 'welcome() := say(greeting)' >"$stage/greet.wap"

check "a used module's function, its statements run first" "$(ran use_one 'use "speak.wap"
say("Hello")')" "loaded
Hello"
check "a module uses its own modules and variables" "$(ran use_nested 'use "greet.wap"
welcome()')" "loaded
hi"
check "a module used twice runs once" "$(ran use_twice 'use "greet.wap"
use "speak.wap"
welcome()
say("both")')" "loaded
hi
both"
printf '%s\n' 'use "pong.wap"' 'ping() := print("ping")' 'serve() := rally()' >"$stage/ping.wap"
printf '%s\n' 'use "ping.wap"' 'pong() := print("pong")' 'rally() := {' '	ping()' '	pong()' '}' >"$stage/pong.wap"
check "modules may use each other" "$(ran use_each_other 'use "ping.wap"
serve()')" "ping
pong"
exit $((failures > 0))
