#!/usr/bin/env bash
# PostToolUse hook: runs ModFoundry's check on the edited file and reports findings without blocking.
# Dependencies come from `mf deps`, saved in each mod's .modfoundry/dependencies.json (docs/modfoundry.md).
set -u
input=$(cat)

note() {
  local msg=$1
  if command -v jq >/dev/null; then
    jq -n --arg msg "$msg" '{hookSpecificOutput: {hookEventName: "PostToolUse", additionalContext: $msg}}'
  else
    msg=${msg//\\/\\\\}; msg=${msg//\"/\\\"}; msg=${msg//$'\n'/\\n}
    printf '{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"%s"}}\n' "$msg"
  fi
}

if ! command -v jq >/dev/null; then
  [[ "$input" =~ \"file_path\"[[:space:]]*:[[:space:]]*\"[^\"]*(config\.cpp|\.c|\.layout)\" ]] &&
    note "jq is not installed, so the edit hook could not read which file changed; nothing was checked (PREREQUISITES.md)."
  exit 0
fi
path=$(printf '%s' "$input" | jq -r '.tool_input.file_path // empty' 2>/dev/null) || exit 0
[ -n "$path" ] && [ -f "$path" ] || exit 0

case "$path" in
  *config.cpp) check=(mf validate); limit=5 ;;
  mods/*.c|*/mods/*.c|tests/*.c|*/tests/*.c) check=(mf lint); limit=15 ;;
  *.layout) check=(mf layout check); limit=5 ;;
  *) exit 0 ;;
esac

if ! { command -v mf >/dev/null && mf --version 2>/dev/null | grep -q '^mf [0-9]'; }; then
  note "ModFoundry (mf) is not installed, so $(basename "$path") was not checked. See docs/modfoundry.md."
  exit 0
fi

out=$(timeout "$limit" "${check[@]}" "$path" 2>&1)
[ $? -eq 124 ] && out="$out
note: mf ran past $limit s and was stopped; nothing was checked"

findings=$(printf '%s\n' "$out" | grep -E '^\[MF-|^game check:|^note: (game check skipped|mf ran past)' || true)
[ -z "$findings" ] && exit 0
note "mf check after editing $(basename "$path") (warning only; a 'game check: module refused' line can come from another file in the module):
$findings"
exit 0
