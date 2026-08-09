#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  printf '%s\n' 'Usage: ./start-scholarpress.sh OPENWORK_WORKSPACE [PORT]'
  exit 0
fi

OPENWORK_WORKSPACE="${1:?OpenWork workspace path is required}"
PORT="${2:-${SCHOLARPRESS_PORT:-8765}}"
BUNDLE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCHOLARPRESS_ROOT="$OPENWORK_WORKSPACE/.scholarpress"
WORKSPACE_ROOT="$SCHOLARPRESS_ROOT/workspaces"
CATALOG_ROOT="$SCHOLARPRESS_ROOT/catalog"
TYPST_PATH="$BUNDLE_ROOT/bin/typst"
PANDOC_PATH="$BUNDLE_ROOT/bin/pandoc"

[[ -d "$BUNDLE_ROOT/catalog" ]] || { printf 'catalog directory not found: %s\n' "$BUNDLE_ROOT/catalog" >&2; exit 1; }
[[ -x "$BUNDLE_ROOT/sp-mcp" ]] || { printf 'sp-mcp not executable: %s\n' "$BUNDLE_ROOT/sp-mcp" >&2; exit 1; }

mkdir -p "$WORKSPACE_ROOT" "$CATALOG_ROOT"
if [[ -z "$(find "$CATALOG_ROOT" -mindepth 1 -maxdepth 1 -print -quit)" ]]; then
  cp -a "$BUNDLE_ROOT/catalog/." "$CATALOG_ROOT/"
fi

export SCHOLARPRESS_CATALOG_PATH="$CATALOG_ROOT"
export SCHOLARPRESS_WORKSPACE_ROOT="$WORKSPACE_ROOT"
export SCHOLARPRESS_TYPST_PATH="$TYPST_PATH"
export SCHOLARPRESS_PANDOC_PATH="$PANDOC_PATH"

printf 'ScholarPress MCP\n'
printf 'Transport: streamable HTTP\n'
printf 'Endpoint: http://127.0.0.1:%s/mcp\n' "$PORT"
printf 'Catalog: %s\n' "$CATALOG_ROOT"
printf 'Typst: %s\n' "$TYPST_PATH"
printf 'Pandoc: %s\n' "$PANDOC_PATH"

exec "$BUNDLE_ROOT/sp-mcp" --transport http --bind 127.0.0.1 --port "$PORT"
