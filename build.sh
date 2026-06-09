#!/usr/bin/env bash
# Build helper for Action language compiler on NixOS
# Works around Chinese-character path issues and LLVM split-package layout
set -euo pipefail

SRC_DIR="/tmp/action-src"
TARGET_DIR="/tmp/action-target"
LLVM_PREFIX="/tmp/llvm-21-combined"

LIBFFI_DIR="/nix/store/2zs4bbi72plfm8j6zxf1js4f3yc4yzwy-libffi-3.5.2/lib"
ZLIB_DIR="/nix/store/61a1nwx3w6rqyaisj5rn1sal1981apm7-zlib-1.3.2/lib"
LIBXML2_DIR="/nix/store/rxwgr4whg32qlkd8fx7skjyd0mrm0zls-libxml2-2.15.2/lib"
GCC_LIB_DIR="/nix/store/ybp235ps7m4yd85v0pgvqkhd4xmxf6jq-gcc-14.3.0-lib/lib"

export CARGO_TARGET_DIR="$TARGET_DIR"
export LLVM_SYS_211_PREFIX="$LLVM_PREFIX"
export RUSTFLAGS="-L $LIBFFI_DIR -L $ZLIB_DIR -L $LIBXML2_DIR"
export LD_LIBRARY_PATH="${LLVM_PREFIX}/lib:${LIBFFI_DIR}:${ZLIB_DIR}:${LIBXML2_DIR}:${GCC_LIB_DIR}"

cd "$SRC_DIR"
exec cargo "$@"
