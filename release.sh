#!/usr/bin/env bash
# release.sh — create a new bruecke release
# Usage: ./release.sh v0.1.0
set -e
cd "$(dirname "$0")"

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
    echo ""
    echo "  Usage: ./release.sh v0.1.0"
    echo ""
    exit 1
fi

echo ""
echo "  bruecke release: $VERSION"
echo "  ──────────────────────────────────────"
echo ""

git tag "$VERSION"
git push origin "$VERSION"

echo "  ✓ Tag $VERSION pushed"
echo "  → GitHub Actions builds Mac / Linux / Windows automatically"
echo "  → Release appears at: https://github.com/LEVOGNE/bruecke/releases"
echo ""
