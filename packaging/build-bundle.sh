#!/usr/bin/env bash
set -euo pipefail

OUTPUT_DIR="${1:-dist}"
if [[ "$OUTPUT_DIR" == "--help" || "$OUTPUT_DIR" == "-h" ]]; then
  printf '%s\n' 'Usage: ./packaging/build-bundle.sh [output-directory]'
  exit 0
fi
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUNDLE="$REPO_ROOT/$OUTPUT_DIR/scholarpress"
SP_MCP_PATH="${SP_MCP_PATH:-$REPO_ROOT/target/release/sp-mcp}"
TYPST_PATH="${TYPST_PATH:-$(command -v typst)}"
PANDOC_PATH="${PANDOC_PATH:-$(command -v pandoc)}"

rm -rf "$BUNDLE"
mkdir -p "$BUNDLE/bin" "$BUNDLE/catalog"
cp "$SP_MCP_PATH" "$BUNDLE/sp-mcp"
cp "$TYPST_PATH" "$BUNDLE/bin/typst"
cp "$PANDOC_PATH" "$BUNDLE/bin/pandoc"
cp -a "$REPO_ROOT/../scholarpress-catalog/institutions" "$BUNDLE/catalog/"
cp "$REPO_ROOT/packaging/start-scholarpress.sh" "$BUNDLE/"
cp "$REPO_ROOT/packaging/README-LINUX.md" "$BUNDLE/"
chmod +x "$BUNDLE/sp-mcp" "$BUNDLE/bin/typst" "$BUNDLE/bin/pandoc" "$BUNDLE/start-scholarpress.sh"

mkdir -p "$REPO_ROOT/$OUTPUT_DIR"
tar -czf "$REPO_ROOT/$OUTPUT_DIR/scholarpress-linux-x86_64.tar.gz" -C "$REPO_ROOT/$OUTPUT_DIR" scholarpress
printf '%s\n' "$REPO_ROOT/$OUTPUT_DIR/scholarpress-linux-x86_64.tar.gz"
