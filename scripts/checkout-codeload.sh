#!/usr/bin/env bash
# Self-hosted checkout via codeload.github.com when github.com:443 git fetch times out.
set -euo pipefail

REPO="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY required}"
SHA="${GITHUB_SHA:?GITHUB_SHA required}"
WS="${GITHUB_WORKSPACE:-$PWD}"
OWNER="${REPO%%/*}"
NAME="${REPO##*/}"
URL="https://codeload.github.com/${OWNER}/${NAME}/tar.gz/${SHA}"

echo "checkout-codeload: ${URL} -> ${WS}"
mkdir -p "$WS"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

curl -fsSL --connect-timeout 30 --max-time 300 "$URL" | tar xz -C "$TMP" --strip-components=1
rsync -a --delete "$TMP/" "$WS/"

echo "checkout-codeload: done ($(find "$WS" -maxdepth 1 | wc -l) top-level entries)"
