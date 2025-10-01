#!/usr/bin/env bash
#
# generate_context.sh - Produce a prompt-ready project overview for ChatGPT.
#
# The script collects high-signal data (metadata, recent changes, tests, TODOs)
# about the current repository and writes it to a timestamped Markdown file in
# the `scripts/` directory. The file can be pasted directly into a chat prompt
# to provide context when brainstorming new features.
#
# Generated artifacts live under `scripts/` and are ignored by git (see
# `.gitignore`). The script itself remains versioned and documented for reuse.

set -euo pipefail

# --- Utility helpers -------------------------------------------------------

# Print usage instructions.
usage() {
    cat <<'USAGE'
Usage: scripts/generate_context.sh [--help]

Collect repository context and write it into scripts/context_YYYYMMDD_HHMMSS.md.
The script expects to be run from anywhere inside the repository.

Options:
  --help    Display this help message and exit.
USAGE
}

# Ensure an external command is available before we rely on it.
require_cmd() {
    local cmd="$1"
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "Error: required command '$cmd' not found in PATH" >&2
        exit 1
    fi
}

# Append a section with a title and body to the output file.
append_section() {
    local title="$1"
    shift
    {
        echo "## $title"
        echo
        "$@"
        echo
    } >>"$OUTPUT_FILE"
}

# --- Data collectors -------------------------------------------------------

collect_project_overview() {
    python3 <<'PY'
import json
import subprocess

meta = json.loads(subprocess.check_output([
    "cargo", "metadata", "--no-deps", "--format-version", "1"
]).decode())
pkg = meta["packages"][0]
print(f"- Name: {pkg['name']}")
print(f"- Version: {pkg['version']}")
print(f"- Edition: {pkg.get('edition', 'unknown')}")
print(f"- Description: {pkg.get('description', 'n/a')}")

features = pkg.get("features", {})
if features:
    print("- Features:")
    for name, deps in sorted(features.items()):
        joined = ", ".join(deps) if deps else "(no extra deps)"
        print(f"  - {name}: {joined}")
PY
}

collect_dependencies() {
    cargo metadata --format-version 1 --no-deps \
        | python3 - <<'PY'
import json, sys
meta = json.load(sys.stdin)
pkg = meta["packages"][0]
print("- Primary dependencies:")
for dep in sorted(pkg.get("dependencies", []), key=lambda d: d["name"]):
    kind = dep.get("kind")
    kind_str = f" ({kind})" if kind else ""
    print(f"  - {dep['name']} {dep['req']}{kind_str}")
PY
}

collect_recent_commits() {
    git log -5 --pretty='- %h %ad %s' --date=short || echo "(no git history)"
}

collect_status() {
    git status --short || echo "(not a git repository)"
}

collect_todos() {
    rg --no-heading --line-number "TODO|FIXME" || echo "(no TODO/FIXME found)"
}

collect_tests() {
    cargo test --all -- --list 2>/dev/null || echo "(unable to list tests)"
}

# --- Main ------------------------------------------------------------------

if [[ ${1:-} == "--help" ]]; then
    usage
    exit 0
fi

require_cmd python3
require_cmd cargo
require_cmd git
require_cmd rg

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="$SCRIPT_DIR"
TIMESTAMP="$(date +'%Y%m%d_%H%M%S')"
OUTPUT_FILE="$OUTPUT_DIR/context_${TIMESTAMP}.md"

{
    echo "# Project Context"
    echo
    echo "Generated: $(date --iso-8601=seconds)"
    echo "Repository: $(basename "$(git rev-parse --show-toplevel)")"
    echo
} >"$OUTPUT_FILE"

append_section "Project Overview" collect_project_overview
append_section "Dependencies" collect_dependencies
append_section "Recent Commits" collect_recent_commits
append_section "Working Tree Status" collect_status
append_section "TODO / FIXME" collect_todos
append_section "Test Matrix" collect_tests

cat <<EOM
Context written to: $OUTPUT_FILE
Share the generated file in your ChatGPT prompt.
EOM
