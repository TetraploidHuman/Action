#!/usr/bin/env bash
# Register an additional self-hosted GitHub Actions runner instance (same machine).
#
# Usage:
#   TOKEN=$(gh api repos/TetraploidHuman/Action/actions/runners/registration-token --method POST -q .token)
#   ./scripts/setup-github-runner.sh 2 "$TOKEN"
#
# Instance 1 (nixos-x64-runner) lives at ~/桌面/Runner/runner and is left untouched.
set -euo pipefail

INSTANCE="${1:?runner instance id (2–9)}"
TOKEN="${2:?GitHub registration token (POST .../registration-token)}"

ROOT="${RUNNER_ROOT:-$HOME/桌面/Runner}"
TEMPLATE="${RUNNER_TEMPLATE:-$ROOT/runner}"
DEST="$ROOT/runner-${INSTANCE}"
NAME="nixos-x64-runner-${INSTANCE}"
REPO="${GITHUB_REPO:-https://github.com/TetraploidHuman/Action}"
RUNNER_VERSION="2.335.1"

if [[ ! -d "$TEMPLATE/bin.${RUNNER_VERSION}" ]]; then
    echo "error: template runner not found at $TEMPLATE" >&2
    exit 1
fi

if [[ -f "$DEST/.runner" ]]; then
    echo "already configured: $DEST ($(jq -r .agentName "$DEST/.runner"))"
    exit 0
fi

# Drop stale partial state from a failed registration attempt.
rm -f "$DEST/.path" "$DEST/.credentials" "$DEST/.credentials_rsaparams" "$DEST/.runner"

mkdir -p "$DEST"

for f in config.sh run.sh run-helper.sh run-helper.sh.template run-helper.cmd.template safe_sleep.sh env.sh; do
    cp "$TEMPLATE/$f" "$DEST/$f"
    chmod +x "$DEST/$f" 2>/dev/null || true
done

if [[ -f "$TEMPLATE/.env" ]]; then
    cp "$TEMPLATE/.env" "$DEST/.env"
fi

# Each instance needs its own bin/ copy: symlinking to ../runner makes
# Runner.Listener pick up the primary instance's .runner file.
cp -a "$TEMPLATE/bin.${RUNNER_VERSION}" "$DEST/bin.${RUNNER_VERSION}"
ln -sfn "bin.${RUNNER_VERSION}" "$DEST/bin"
cp -a "$TEMPLATE/externals.${RUNNER_VERSION}" "$DEST/externals.${RUNNER_VERSION}"
ln -sfn "externals.${RUNNER_VERSION}" "$DEST/externals"

cat > "$DEST/run-nixos.sh" << 'EOF'
#!/run/current-system/sw/bin/bash
set -a
[[ -f "$(dirname "$0")/.env" ]] && source "$(dirname "$0")/.env"
set +a
export DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1
cd "$(dirname "$0")"
exec /run/current-system/sw/bin/bash ./run-helper.sh "$@"
EOF
chmod +x "$DEST/run-nixos.sh"

(
    cd "$DEST"
    set -a
    [[ -f .env ]] && source .env
    set +a
    export DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1
    /run/current-system/sw/bin/bash ./config.sh \
        --url "$REPO" \
        --token "$TOKEN" \
        --name "$NAME" \
        --labels self-hosted,Linux,X64 \
        --unattended \
        --replace
)

echo "registered $NAME at $DEST"
echo "start with: $DEST/run-nixos.sh  (or: ./scripts/start-github-runners.sh)"
