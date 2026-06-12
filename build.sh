#!/bin/bash
# Build script for Action compiler
# Works around NixOS Unicode path issue by building from ASCII-only path
set -e
SRC_DIR="/home/miaox99/桌面/Action"
BUILD_DIR="/home/miaox99/action-build"
LLVM_PREFIX="/home/miaox99/llvm-combined"
LIB_PATHS=(
    /nix/store/2zs4bbi72plfm8j6zxf1js4f3yc4yzwy-libffi-3.5.2/lib
    /nix/store/61a1nwx3w6rqyaisj5rn1sal1981apm7-zlib-1.3.2/lib
    /nix/store/rxwgr4whg32qlkd8fx7skjyd0mrm0zls-libxml2-2.15.2/lib
    /nix/store/d3fl1d6ny4yy3y96nr9waqm7p36js4v8-llvm-21.1.8-lib/lib
    /nix/store/ybp235ps7m4yd85v0pgvqkhd4xmxf6jq-gcc-14.3.0-lib/lib
)
export LLVM_SYS_211_PREFIX="$LLVM_PREFIX"
export RUSTFLAGS="-L ${LIB_PATHS[0]} -L ${LIB_PATHS[1]} -L ${LIB_PATHS[2]}"
export LD_LIBRARY_PATH=$(IFS=:; echo "${LIB_PATHS[*]}")
rsync -a --delete "$SRC_DIR/" "$BUILD_DIR/" --exclude target
cd "$BUILD_DIR"
exec cargo "$@"
