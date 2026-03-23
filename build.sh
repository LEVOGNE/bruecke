#!/usr/bin/env bash
# bruecke build script — auto-detects OS, asks for cross-compile targets
set -e
cd "$(dirname "$0")"

# ── detect current OS ─────────────────────────────────────────────────────────
case "$OSTYPE" in
    darwin*)  NATIVE="macOS" ;;
    linux*)   NATIVE="Linux" ;;
    msys*|cygwin*|win32*) NATIVE="Windows" ;;
    *)        NATIVE="unknown ($OSTYPE)" ;;
esac

echo ""
echo "  bruecke build"
echo "  ─────────────────────────────────────────"
echo "  Detected OS: $NATIVE"
echo ""
echo "  Build targets:"
echo "    [1] Native only ($NATIVE)"
echo "    [2] Native + Linux x86_64"
echo "    [3] Native + Windows x86_64"
echo "    [4] All platforms (native + Linux + Windows)"
echo ""
read -rp "  Choose [1-4]: " CHOICE
echo ""

BUILD_LINUX=false
BUILD_WINDOWS=false
case "$CHOICE" in
    2) BUILD_LINUX=true ;;
    3) BUILD_WINDOWS=true ;;
    4) BUILD_LINUX=true; BUILD_WINDOWS=true ;;
esac

# ── check cross if needed ─────────────────────────────────────────────────────
if ($BUILD_LINUX || $BUILD_WINDOWS) && ! command -v cross &>/dev/null; then
    echo "  'cross' not found — install it first:"
    echo "    cargo install cross"
    echo ""
    echo "  Continuing with native only."
    BUILD_LINUX=false
    BUILD_WINDOWS=false
fi

# ── build ─────────────────────────────────────────────────────────────────────
echo "▶ 1/3  Building WASM..."
wasm-pack build --target web --release

echo "▶ 2/3  Building server (native)..."
cargo build --bin server --release

echo "▶ 3/4  Copying WASM into assets/ (for cargo install / crates.io)..."
mkdir -p assets
cp pkg/bruecke_bg.wasm assets/bruecke_bg.wasm

echo "▶ 4/4  Assembling dist/..."
mkdir -p dist
cp pkg/bruecke_bg.wasm dist/bruecke_bg.wasm
cp app.py dist/app.py

# native binary
if [[ "$OSTYPE" == "msys"* || "$OSTYPE" == "cygwin"* ]]; then
    cp target/release/server.exe dist/server.exe
    echo "    ✓ server.exe ($NATIVE)"
else
    cp target/release/server dist/server
    echo "    ✓ server ($NATIVE)"
fi

# cross-compile Linux
if $BUILD_LINUX; then
    echo "    cross → Linux x86_64..."
    cross build --bin server --release --target x86_64-unknown-linux-musl
    cp target/x86_64-unknown-linux-musl/release/server dist/server-linux-x86_64
    echo "    ✓ server-linux-x86_64"
fi

# cross-compile Windows
if $BUILD_WINDOWS; then
    echo "    cross → Windows x86_64..."
    cross build --bin server --release --target x86_64-pc-windows-gnu
    cp target/x86_64-pc-windows-gnu/release/server.exe dist/server-windows-x86_64.exe
    echo "    ✓ server-windows-x86_64.exe"
fi

# ── README ────────────────────────────────────────────────────────────────────
cat > dist/README.txt << 'EOF'
bruecke
=======

Start the server for your platform:
  macOS / Linux:   ./server
  Linux (x86_64):  ./server-linux-x86_64
  Windows:         server.exe  or  server-windows-x86_64.exe

Then open: http://127.0.0.1:7777

Edit app.py — browser updates instantly on save.
EOF

echo ""
echo "✓ dist/ ready:"
ls -lh dist/
echo ""
echo "  Run:  cd dist && ./server"
echo "  Open: http://127.0.0.1:7777"
echo ""
