#!/bin/bash
# FIDO-LABS - Script de build rapide
# Usage: ./build.sh [release|dev]

cd "$(dirname "$0")/fido-labs-complete/src-tauri"

echo "═══════════════════════════════════════"
echo "  FIDO-LABS v3.0 — Build Script"
echo "═══════════════════════════════════════"

if [ "$1" = "release" ]; then
    echo "[BUILD] Compilation release..."
    "C:/Users/digis/.cargo/bin/cargo.exe" build --release 2>&1 | tail -5
    if [ ${PIPESTATUS[0]} -ne 0 ]; then
        echo "[ERROR] Compilation échouée."
        exit 1
    fi
    echo "[BUILD] ✓ Compilation terminée"
    echo "[INFO]  Exécutable: target/release/fido-labs.exe"

    echo "[SIGN] Signing application..."
    "C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe" -File ../../sign-app.ps1
    if [ $? -ne 0 ]; then
        echo "[ERROR] Signing failed."
        exit 1
    fi
    echo "[SIGN] ✓ Signature terminée"

    echo "[INSTALLER] Compilation de l'installateur..."
    "C:/Program Files/Inno Setup 7/ISCC.exe" "../../installer/FIDO-LABS-setup.iss"
    if [ $? -ne 0 ]; then
        echo "[ERROR] Compilation installateur échouée."
        exit 1
    fi
    echo "[INSTALLER] ✓ Installateur compilé dans ../dist/"
else
    echo "[BUILD] Compilation dev..."
    "C:/Users/digis/.cargo/bin/cargo.exe" tauri dev 2>&1 | tail -20
    if [ $? -ne 0 ]; then
        echo "[ERROR] Compilation dev échouée."
        exit 1
    fi
fi
