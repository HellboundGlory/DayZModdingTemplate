#!/usr/bin/env bash
# One-time setup after "Use this template", and the agent-folder sync afterwards.
#   ./setup.sh                 ask the family prefix and core, the author, the folders, the frameworks and
#                              the reference mod folder, fill the placeholders, write tools/family.conf,
#                              copy the agent folders, optionally set up trail
#   ./setup.sh --sync-agents   copy .agents/skills and .agents/agents into .claude/ again
#   ./setup.sh --check-agents  exit 1 if a .claude/ copy differs from its .agents/ source
set -euo pipefail
shopt -u patsub_replacement 2>/dev/null || true
cd "$(dirname "$0")"

PLACEHOLDERS='\{\{(FAMILY|CORE|AUTHOR|DAYZ_BUILD|DAYZ_REFERENCE|WORKSHOP|FRAMEWORKS|REFERENCE_MODS)\}\}'
TEMPLATE_REPO=HellboundGlory/DayZModdingTemplate

# Copies the sources over the copies; files of your own in .claude/skills or .claude/agents are left alone.
sync_agents() {
  local d
  for d in skills agents; do
    [ -d ".agents/$d" ] || { echo ".agents/$d is missing; nothing copied." >&2; return 1; }
    mkdir -p ".claude/$d"
    cp -R ".agents/$d/." ".claude/$d/"
  done
  echo "Copied .agents/skills and .agents/agents into .claude/."
}

check_agents() {
  local bad=0 f
  while IFS= read -r -d '' f; do
    if ! cmp -s "$f" ".claude/${f#.agents/}"; then
      echo ".claude/${f#.agents/} differs from $f: run ./setup.sh --sync-agents"
      bad=1
    fi
  done < <(find .agents/skills .agents/agents -type f -print0)
  return $bad
}

case "${1:-}" in
  --sync-agents) sync_agents; exit $? ;;
  --check-agents) check_agents; exit $? ;;
  "") ;;
  *) echo "usage: $0 [--sync-agents | --check-agents]" >&2; exit 2 ;;
esac

is_modfoundry() { command -v mf >/dev/null && mf --version 2>/dev/null | grep -q '^mf [0-9]'; }

# Prints the answer, or the default when it is empty. Stops setup if input ends.
ask() {
  local prompt=$1 default=${2:-} answer
  if [ -n "$default" ]; then prompt="$prompt [$default]"; fi
  if ! read -r -p "$prompt: " answer && [ -z "$answer" ]; then
    echo >&2
    echo "Input ended before setup finished; nothing was changed. Run ./setup.sh in a terminal." >&2
    exit 1
  fi
  answer=${answer%$'\r'}
  answer=${answer:-$default}
  if [[ "$answer" == *"{{"* ]]; then
    echo "Answers cannot contain '{{'." >&2
    exit 1
  fi
  printf '%s' "$answer"
}

ask_required() {
  local answer
  while :; do
    answer=$(ask "$@") || exit 1
    [ -n "$answer" ] && { printf '%s' "$answer"; return; }
    echo "  This one is needed." >&2
  done
}

# ask_match <regex> <rule> <prompt> [default]: asks again until the answer matches the regex.
ask_match() {
  local pattern=$1 rule=$2 prompt=$3 default=${4:-} answer LC_ALL=C
  while :; do
    answer=$(ask "$prompt" "$default") || exit 1
    [[ "$answer" =~ $pattern ]] && { printf '%s' "$answer"; return; }
    echo "  '$answer' must be $rule." >&2
  done
}

# ask_path <yes|no> <prompt> [default]: a path, required or not, made absolute; ~name is refused.
ask_path() {
  local need=$1 answer
  shift
  while :; do
    if [ "$need" = yes ]; then answer=$(ask_required "$@") || exit 1; else answer=$(ask "$@") || exit 1; fi
    if [[ "$answer" == "~"[!/]* ]]; then
      echo "  Start with ~/ or give a full path; ~name is not supported." >&2
      continue
    fi
    expand_path "$answer"
    return
  done
}

yes_no() {
  local answer
  read -r -p "$1 [y/N]: " answer || return 1
  [[ "$answer" =~ ^[Yy] ]]
}

# A path as typed (~, Windows or POSIX) to an absolute POSIX path.
expand_path() {
  local p=$1
  [[ "$p" == "~" || "$p" == "~/"* ]] && p="${HOME%/}${p:1}"
  if [[ "$p" =~ ^[A-Za-z]:[\\/] ]] && command -v cygpath >/dev/null; then p=$(cygpath -u "$p"); fi
  printf '%s' "$p"
}

# A path for the repo's files: from ~ when it is under the home folder, so no account name is written down.
tilde() {
  local home=${HOME%/}
  [ -n "$home" ] || { printf '%s' "$1"; return; }
  case "$1" in
    "$home") printf '~' ;;
    "$home"/*) printf '~/%s' "${1#"$home"/}" ;;
    *) printf '%s' "$1" ;;
  esac
}

valid_id() { local LC_ALL=C; [[ "$1" =~ ^[A-Za-z][A-Za-z0-9]*$ ]]; }

placeholder_files() { grep -rlIE --exclude-dir=.git --exclude=setup.sh "$PLACEHOLDERS" . || true; }

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "This folder is not a git repository. Clone your repo (or run 'git init') and run ./setup.sh again." >&2
  exit 1
fi

if [ -z "$(placeholder_files)" ]; then
  echo "The placeholders are already filled. Syncing the agent folders only."
  [ -f tools/family.conf ] || echo "tools/family.conf is missing: copy the shape from USAGE.md and fill it in." >&2
  sync_agents
  exit 0
fi

echo "Setting up this DayZ mod family. Press Enter to take the value in brackets."
echo

family=$(ask_required "Mod family name (used in titles)" "$(basename "$PWD")")
prefix=$(ask_match '^[A-Z][A-Za-z0-9]*$' "letters and digits, starting with a capital" \
  "Family prefix, the start of every mod id (for example Acme)")
core=$(ask_match "^$prefix[A-Za-z0-9]+$" "the prefix $prefix followed by letters and digits" \
  "Core mod id" "${prefix}Core")
author=$(ask_required "Author name" "$(git config user.name 2>/dev/null || true)")
git_name=$(ask_required "Git commit name for this repo" "$author")
git_email=$(ask_required "Git commit email for this repo (a GitHub noreply address keeps yours private)" \
  "$(git config user.email 2>/dev/null || true)")

case "${OSTYPE:-}" in
  msys*|cygwin*) steam="/c/Program Files (x86)/Steam/steamapps" ;;
  *) steam="$HOME/.local/share/Steam/steamapps" ;;
esac
workshop=$(ask_path yes "Workshop folder for DayZ" "$steam/workshop/content/221100")
build=$(ask_required "Target DayZ build (the main menu shows it, for example 1.30.164289)")
reference=$(ask_path yes "Unpacked vanilla game folder (docs/modfoundry.md §Unpacking the game)" \
  "$HOME/DayZ-reference/dayz-$build")

echo
echo "Frameworks the family depends on. Common ones:"
echo "  Community Framework: name CF, Workshop 1559212036, CfgPatches JM_CF_Scripts"
echo "  Dabs Framework: name Dabs, Workshop 2545327648, CfgPatches DF_GUI DF_Scripts"
echo "Press Enter at the first name for none; at a later one, when there are no more."
frameworks=""
framework_conf=""
framework_names=" "
while :; do
  name=$(ask "Framework name, letters and digits (empty when done)") || exit 1
  [ -z "$name" ] && break
  if ! valid_id "$name"; then
    echo "  '$name' is not letters and digits." >&2
    continue
  fi
  if [ "$name" = "$prefix" ] || [ "$name" = "$core" ]; then
    echo "  $name is the family's own prefix or core." >&2
    continue
  fi
  if [[ "$framework_names" == *" $name "* ]]; then
    echo "  $name is already listed." >&2
    continue
  fi
  framework_id=$(ask_match '^[0-9]+$' "digits only" "Workshop id for $name") || exit 1
  addons=$(ask_match '^[A-Za-z0-9_]+( +[A-Za-z0-9_]+)*$' \
    "CfgPatches names of letters, digits and underscores, space separated" "CfgPatches names for $name") || exit 1
  read -r -a addon_words <<<"$addons"
  framework_names+="$name "
  frameworks+="${frameworks:+, }$name (Workshop $framework_id)"
  framework_conf+="framework = $name ${addon_words[*]}"$'\n'
done
[ -n "$frameworks" ] || frameworks="none"
reference_mods=$(ask "Reference mods you read for behaviour only, comma separated" "none")
reference_mod=$(ask_path no "Reference mod folder for check-reference (empty for none)")

reference_out=$(tilde "$reference")
workshop_out=$(tilde "$workshop")
reference_mod_out=""
if [ -n "$reference_mod" ]; then reference_mod_out=$(tilde "$reference_mod"); fi

# replace_in <file> <old> <new> [<old> <new>]...: literal replacements, the file's bytes otherwise kept.
replace_in() {
  local f=$1 text
  shift
  IFS= read -r -d '' text <"$f" || true
  while [ $# -ge 2 ]; do
    text=${text//"$1"/"$2"}
    shift 2
  done
  printf '%s' "$text" >"$f"
}

git config user.name "$git_name"
git config user.email "$git_email"

while IFS= read -r f; do
  replace_in "$f" \
    "{{FAMILY}}" "$family" "{{CORE}}" "$core" "{{AUTHOR}}" "$author" "{{DAYZ_BUILD}}" "$build" \
    "{{DAYZ_REFERENCE}}" "$reference_out" "{{WORKSHOP}}" "$workshop_out" \
    "{{FRAMEWORKS}}" "$frameworks" "{{REFERENCE_MODS}}" "$reference_mods"
done < <(placeholder_files)

replace_in README.md \
  "# DayZModdingTemplate" "# $family" \
  "A starting point for a DayZ mod family that coding agents work on." \
  "A DayZ mod family, started from [DayZModdingTemplate](https://github.com/$TEMPLATE_REPO)."
replace_in LICENSE "Copyright (c) 2026 Hellbound" "Copyright (c) $(date +%Y) $author"

mkdir -p tools
{
  cat <<'EOF'
# The family's values the checks in tools/ read. The `family` check shows them (USAGE.md).
# Comments take a whole line. A relative path is under the repo root, and ~ is HOME.
EOF
  printf 'prefix = %s\ncore = %s\n' "$prefix" "$core"
  printf '%s' "$framework_conf"
  if [ -n "$reference_mod_out" ]; then printf 'reference = %s\n' "$reference_mod_out"; fi
  printf 'vanilla = %s\n' "$reference_out"
} >tools/family.conf

sync_agents

echo
if [ -d "$reference/scripts" ]; then
  echo "Found the unpacked game at $reference."
else
  echo "No unpacked game at $reference yet. docs/modfoundry.md §Unpacking the game says how to make it."
fi
if is_modfoundry; then
  echo "ModFoundry: $(mf --version). After adding a mod, save its dependencies with mf deps add (USAGE.md)."
else
  echo "ModFoundry (mf) not found. It is optional; PREREQUISITES.md links it."
fi

trail_note="trail init also changes your agents' user-wide settings (for Claude Code, ~/.claude/settings.json)."
if command -v trail >/dev/null; then
  echo "$trail_note"
  if yes_no "Wire trail into your agents here (trail init)?"; then
    trail init . || echo "trail init failed; run it again later." >&2
  fi
elif command -v npm >/dev/null; then
  echo "$trail_note"
  if yes_no "trail is not installed. Install it with npm (npm install -g @trailhq/trail-code) and run trail init?"; then
    if npm install -g @trailhq/trail-code; then
      trail init . || echo "trail init failed; run it again later." >&2
    else
      echo "npm could not install trail; PREREQUISITES.md links it." >&2
    fi
  fi
fi

origin=$(git remote get-url origin 2>/dev/null || true)
if command -v gh >/dev/null && [ -n "$origin" ] && [[ "$origin" != *"$TEMPLATE_REPO"* ]]; then
  slug=$(printf '%s' "$family" | tr -c 'A-Za-z0-9._-' '-' | sed -e 's/^-*//' -e 's/-*$//')
  if [ -n "$slug" ] && yes_no "Rename the GitHub repo at $origin to '$slug' (gh repo rename)?"; then
    gh repo rename "$slug" --yes || echo "gh repo rename failed; rename it on GitHub instead." >&2
  fi
fi

echo
echo "Done. Review the changes with 'git status' and 'git diff' (tools/family.conf is new) and commit them."
echo "Then add your core mod: USAGE.md §Adding a mod."
