#!/usr/bin/env bash
set -euo pipefail

log() { printf "\033[1;32m[info]\033[0m %s\n" "$*"; }
warn() { printf "\033[1;33m[warn]\033[0m %s\n" "$*"; }
err() { printf "\033[1;31m[error]\033[0m %s\n" "$*" >&2; }

SUDO=""
if [ "$(id -u)" -ne 0 ]; then
  if command -v sudo >/dev/null 2>&1; then
    SUDO="sudo"
  else
    warn "Running without sudo. Some apt operations may fail if not root."
  fi
fi

if ! command -v curl >/dev/null 2>&1 || ! command -v git >/dev/null 2>&1; then
  if command -v apt-get >/dev/null 2>&1; then
    log "Installing prerequisites: curl, git, ca-certificates, build-essential"
    $SUDO apt-get update -y
    $SUDO apt-get install -y --no-install-recommends curl git ca-certificates build-essential
  else
    warn "apt-get not found. Please install curl and git manually."
  fi
fi

ensure_node() {
  if command -v node >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
    return
  fi
  if command -v apt-get >/dev/null 2>&1; then
    log "Installing Node.js (LTS) and npm via NodeSource"
    curl -fsSL https://deb.nodesource.com/setup_lts.x | $SUDO bash -
    $SUDO apt-get install -y nodejs
  else
    err "Node.js/npm not found and apt-get unavailable. Install Node.js/npm and rerun."
    exit 1
  fi
}

ensure_node

if ! command -v rustc >/dev/null 2>&1; then
  log "Installing Rust via rustup (non-interactive)"
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y --no-modify-path
  if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
  else
    warn "~/.cargo/env not found after install"
  fi
else
  log "Rust already installed: $(rustc --version)"
fi

if ! command -v rustup >/dev/null 2>&1; then
  if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
  fi
fi

if command -v rustup >/dev/null 2>&1; then
  if rustup target list --installed | grep -q '^wasm32-unknown-unknown'; then
    log "Rust target wasm32-unknown-unknown already installed"
  else
    log "Adding Rust target wasm32-unknown-unknown"
    rustup target add wasm32-unknown-unknown
  fi
else
  err "rustup not found on PATH. Please restart your shell or source ~/.cargo/env, then rerun."
  exit 1
fi


if ! command -v wasm-pack >/dev/null 2>&1; then
  log "Installing wasm-pack"
  if curl -fsSL https://rustwasm.github.io/wasm-pack/installer/init.sh | sh; then
    :
  else
    warn "Installer script failed. Falling back to cargo install"
    if command -v cargo >/dev/null 2>&1; then
      cargo install wasm-pack
    else
      err "cargo not found and wasm-pack installer failed."
      exit 1
    fi
  fi
else
  log "wasm-pack already installed: $(wasm-pack --version)"
fi

if ! command -v wasm-bindgen >/dev/null 2>&1; then
  log "Installing wasm-bindgen-cli (optional, safe to skip if not needed)"
  cargo install wasm-bindgen-cli || warn "wasm-bindgen-cli install failed; continuing"
fi

if [ -f package.json ]; then
  if [ ! -d node_modules ]; then
    if [ -f package-lock.json ] || [ -f npm-shrinkwrap.json ]; then
      log "Running npm ci"
      npm ci
    else
      log "Running npm install"
      npm install
    fi
  fi
  log "Running npm run build"
  npm run build
else
  warn "No package.json found in current directory. Skipping npm build."
fi

log "WebAssembly toolchain setup completed."
