#!/usr/bin/env bash
#
# generate_context.sh - Produce a prompt-ready project overview for ChatGPT.
#
# The script collects high-signal data (metadata, recent changes, tests, TODOs,
# source snapshot) about the current repository and writes it to a timestamped
# Markdown file in the `scripts/` directory. The output can be pasted directly
# into a chat prompt to bootstrap discussions about new features.
#
# Generated artifacts live under `scripts/` and are ignored by git. This script
# itself is versioned so teams can adapt it easily.

set -euo pipefail

# --- Utility helpers -------------------------------------------------------

usage() {
    cat <<'USAGE'
Usage: scripts/generate_context.sh [--help]

Collect repository context and write it into scripts/context_YYYYMMDD_HHMMSS.md.
The script can be launched from anywhere inside the workspace.

Options:
  --help    Display this help message and exit.
USAGE
}

require_cmd() {
    local cmd="$1"
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "Error: required command '$cmd' not found in PATH" >&2
        exit 1
    fi
}

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
    python3 <<'PY'
import json
import subprocess

meta = json.loads(subprocess.check_output([
    "cargo", "metadata", "--no-deps", "--format-version", "1"
]).decode())
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
    grep -R --line-number --exclude-dir=target -E "TODO|FIXME" src tests 2>/dev/null || echo "(no TODO/FIXME found)"
}

collect_tests() {
    cargo test --all -- --list 2>/dev/null || echo "(unable to list tests)"
}

collect_rs_sources() {
    while IFS= read -r file; do
        local local_rel="${file#./}"
        echo "### ${local_rel}"
        echo
        echo '```rust'
        cat "$file"
        echo '```'
        echo
    done < <(find src -type f -name '*.rs' -print | sort)
}

# --- Main ------------------------------------------------------------------

if [[ ${1:-} == "--help" ]]; then
    usage
    exit 0
fi

require_cmd python3
require_cmd cargo
require_cmd git
require_cmd grep

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git rev-parse --show-toplevel)"
OUTPUT_DIR="$SCRIPT_DIR"
TIMESTAMP="$(date +'%Y%m%d_%H%M%S')"
OUTPUT_FILE="$OUTPUT_DIR/context_${TIMESTAMP}.md"

cd "$REPO_ROOT"

{
    echo "# Project Context"
    echo
    echo "Generated: $(date --iso-8601=seconds)"
    echo "Repository: $(basename "$REPO_ROOT")"
    echo
} >"$OUTPUT_FILE"

append_section "Project Overview" collect_project_overview
append_section "Dependencies" collect_dependencies
append_section "Recent Commits" collect_recent_commits
append_section "Working Tree Status" collect_status
append_section "TODO / FIXME" collect_todos
append_section "Test Matrix" collect_tests
append_section "Rust Source Snapshot" collect_rs_sources

cat <<EOM
Context written to: $OUTPUT_FILE
Share the generated file in your ChatGPT prompt.
EOM
