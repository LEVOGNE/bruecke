#!/usr/bin/env bash
# cleanreset.sh — löscht alle generierten Ordner und Dateien
set -e
cd "$(dirname "$0")"

echo ""
echo "  bruecke cleanreset"
echo "  ──────────────────────────────────────"

rm -rf target/   && echo "  ✓ target/"
rm -rf pkg/      && echo "  ✓ pkg/"
rm -rf dist/     && echo "  ✓ dist/"
rm -rf assets/   && echo "  ✓ assets/"
rm -f  bruecke_state.json && echo "  ✓ bruecke_state.json"
rm -rf history/  && echo "  ✓ history/"

echo ""
echo "  fertig. weiter mit: ./build.sh"
echo ""
