#!/bin/bash
# FIDO-LABS - Script de build rapide
# Usage: ./build.sh [release|dev]
set -e

cd "$(dirname "$0")/src-tauri"

echo "═══════════════════════════════════════"
echo "  FIDO-LABS v3.0 — Build Script"
echo "═══════════════════════════════════════"

if [ "$1" = "release" ]; then
    echo "[BUILD] Compilation release..."
    cargo build --release 2>&1 | tail -5
    echo "[BUILD] ✓ Compilation terminée"
    echo "[INFO]  Exécutable: target/release/fido-labs.exe"
    echo "[INFO]  Installeur Inno: ../installer/FIDO-LABS-setup.iss"
else
    echo "[BUILD] Compilation dev..."
    cargo tauri dev 2>&1 | tail -20
fi
