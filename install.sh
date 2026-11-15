#!/usr/bin/env bash
# ==============================================================================
# Open Business Application Framework (OBAF) - 1-Line Universal Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/dhirajnavghare/Open-Business-Application-Framework/main/install.sh | bash
# ==============================================================================
set -e

OBAF_DIR="${HOME}/.obaf"
OBAF_BIN="${OBAF_DIR}/bin"

echo ""
echo "  ___  ____    _    _____ "
echo " / _ \| __ )  / \  |  ___|"
echo "| | | |  _ \ / _ \ | |_   "
echo "| |_| | |_) / ___ \|  _|  "
echo " \___/|____/_/   \_\_|    "
echo ""
echo "🚀 Installing Open Business Application Framework (OBAF)..."

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        PLATFORM="linux"
        ;;
    Darwin)
        PLATFORM="macos"
        ;;
    *)
        echo "❌ Unsupported operating system: $OS"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)
        TARGET_ARCH="x86_64"
        ;;
    arm64|aarch64)
        TARGET_ARCH="aarch64"
        ;;
    *)
        echo "❌ Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

echo "Detected Platform: ${PLATFORM} (${TARGET_ARCH})"
mkdir -p "${OBAF_BIN}"

# Check for Cargo if local source available or install from repository
if [ -f "./Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
    echo "📦 Building release binaries from local repository workspace..."
    cargo build --release -p obaf-server -p obaf-cli -p obaf-worker
    cp ./target/release/obaf-server "${OBAF_BIN}/"
    cp ./target/release/obaf-cli "${OBAF_BIN}/obaf"
    cp ./target/release/obaf-worker "${OBAF_BIN}/"
else
    echo "📦 Compiling and installing OBAF via Cargo..."
    if command -v cargo >/dev/null 2>&1; then
        cargo install --git https://github.com/dhirajnavghare/Open-Business-Application-Framework.git obaf-cli --bin obaf-cli
        cargo install --git https://github.com/dhirajnavghare/Open-Business-Application-Framework.git obaf-server --bin obaf-server
        cargo install --git https://github.com/dhirajnavghare/Open-Business-Application-Framework.git obaf-worker --bin obaf-worker
        cp "$(which obaf-cli)" "${OBAF_BIN}/obaf"
    else
        echo "❌ Cargo not found. Please install Rust from https://rustup.rs to compile OBAF."
        exit 1
    fi
fi

chmod +x "${OBAF_BIN}/obaf" "${OBAF_BIN}/obaf-server" "${OBAF_BIN}/obaf-worker"

echo ""
echo "✅ OBAF successfully installed to: ${OBAF_BIN}"
echo ""

# Shell configuration hint
SHELL_NAME="$(basename "${SHELL:-bash}")"
PROFILE_FILE=""
if [ "$SHELL_NAME" = "zsh" ]; then
    PROFILE_FILE="${HOME}/.zshrc"
elif [ "$SHELL_NAME" = "bash" ]; then
    PROFILE_FILE="${HOME}/.bashrc"
fi

if [[ ":$PATH:" != *":${OBAF_BIN}:"* ]]; then
    echo "To add 'obaf' to your PATH, run:"
    echo "  export PATH=\"${OBAF_BIN}:\$PATH\""
    if [ -n "$PROFILE_FILE" ]; then
        echo ""
        echo "Or add it permanently to your profile:"
        echo "  echo 'export PATH=\"${OBAF_BIN}:\$PATH\"' >> ${PROFILE_FILE}"
    fi
fi

echo ""
echo "✨ Get started with OBAF:"
echo "  obaf doctor                          # Check database connectivity and presets"
echo "  obaf presets                         # Explore all 22 domain business presets"
echo "  obaf init my-store --preset pharmacy # Scaffold a turnkey business project"
echo "  obaf-server                          # Start high-performance API server"
echo ""
