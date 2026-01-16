#!/bin/bash
# PreToolUse hook to enforce .claudeignore patterns
# Blocks Read, Edit, Write, Glob, Grep from accessing ignored paths

set -e

# Read JSON input from stdin
INPUT=$(cat)

TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
TOOL_INPUT=$(echo "$INPUT" | jq -r '.tool_input // empty')

# Find .claudeignore (walk up from cwd)
find_claudeignore() {
    local dir="$PWD"
    while [[ "$dir" != "/" ]]; do
        if [[ -f "$dir/.claudeignore" ]]; then
            echo "$dir/.claudeignore"
            return 0
        fi
        dir=$(dirname "$dir")
    done
    return 1
}

IGNORE_FILE=$(find_claudeignore 2>/dev/null) || exit 0

# Extract path from tool input based on tool type
case "$TOOL_NAME" in
    Read|Edit|Write)
        TARGET_PATH=$(echo "$TOOL_INPUT" | jq -r '.file_path // empty')
        ;;
    Glob)
        TARGET_PATH=$(echo "$TOOL_INPUT" | jq -r '.path // empty')
        PATTERN=$(echo "$TOOL_INPUT" | jq -r '.pattern // empty')
        ;;
    Grep)
        TARGET_PATH=$(echo "$TOOL_INPUT" | jq -r '.path // empty')
        ;;
    *)
        exit 0
        ;;
esac

# If no target path, allow (can't check)
[[ -z "$TARGET_PATH" && -z "$PATTERN" ]] && exit 0

# Read ignore patterns (skip comments and empty lines)
while IFS= read -r pattern || [[ -n "$pattern" ]]; do
    # Skip comments and empty lines
    [[ -z "$pattern" || "$pattern" =~ ^[[:space:]]*# ]] && continue
    # Trim whitespace
    pattern=$(echo "$pattern" | xargs)
    [[ -z "$pattern" ]] && continue

    # Check if target path contains the ignored pattern
    if [[ -n "$TARGET_PATH" && "$TARGET_PATH" == *"$pattern"* ]]; then
        echo "Blocked: path matches .claudeignore pattern '$pattern'"
        exit 2
    fi

    # For Glob, also check if pattern would match ignored dirs
    if [[ "$TOOL_NAME" == "Glob" && -n "$PATTERN" && "$PATTERN" == *"$pattern"* ]]; then
        echo "Blocked: glob pattern matches .claudeignore pattern '$pattern'"
        exit 2
    fi
done < "$IGNORE_FILE"

exit 0
