#!/usr/bin/env bash
# Install user systemd units for all local GitHub Actions runners (boot + linger).
#
# Runners managed:
#   ~/桌面/Runner/runner      -> actions-runner.service      (ci)
#   ~/桌面/Runner/runner-2..5 -> actions-runner-{2..5}.service (ci)
#   ~/桌面/Runner/runner-6    -> actions-runner-6.service    (benchmark)
#   ~/actions-runner          -> actions-runner-atomic-lang.service (atomic-lang repo)
#
# Usage:
#   ./scripts/install-github-runner-services.sh
#   ./scripts/install-github-runner-services.sh --enable-now
set -euo pipefail

ENABLE_NOW=0
[[ "${1:-}" == "--enable-now" ]] && ENABLE_NOW=1

RUNNER_ROOT="${RUNNER_ROOT:-$HOME/桌面/Runner}"
LOG_DIR="$RUNNER_ROOT/logs"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
BASH_BIN="/run/current-system/sw/bin/bash"

mkdir -p "$LOG_DIR" "$UNIT_DIR"

write_service() {
    local unit_name="$1"
    local workdir="$2"
    local description="$3"
    local logfile="$4"
    local wanted_by="${5:-actions-runners.target}"

    if [[ ! -f "$workdir/.runner" ]]; then
        echo "skip $unit_name (no .runner in $workdir)" >&2
        return 0
    fi
    if [[ ! -x "$workdir/run-nixos.sh" && ! -x "$workdir/run-helper.sh" ]]; then
        echo "skip $unit_name (no launcher in $workdir)" >&2
        return 0
    fi

  local launcher="run-nixos.sh"
  [[ -x "$workdir/run-nixos.sh" ]] || launcher="run-helper.sh"

    cat >"$UNIT_DIR/${unit_name}.service" <<EOF
[Unit]
Description=${description}
After=network-online.target
Wants=network-online.target
PartOf=actions-runners.target

[Service]
Type=simple
WorkingDirectory=${workdir}
ExecStart=${BASH_BIN} ${workdir}/${launcher}
Restart=on-failure
RestartSec=10
StandardOutput=append:${logfile}
StandardError=append:${logfile}

[Install]
WantedBy=${wanted_by}
EOF
    echo "wrote ${unit_name}.service -> ${workdir}"
}

# Target: all Action repo runners
cat >"$UNIT_DIR/actions-runners.target" <<'EOF'
[Unit]
Description=GitHub Actions runners (TetraploidHuman/Action)
Wants=actions-runner.service
Wants=actions-runner-2.service
Wants=actions-runner-3.service
Wants=actions-runner-4.service
Wants=actions-runner-5.service
Wants=actions-runner-6.service

[Install]
WantedBy=default.target
EOF
echo "wrote actions-runners.target"

write_service "actions-runner" "$RUNNER_ROOT/runner" \
    "GitHub Actions Runner (nixos-x64-runner)" "$LOG_DIR/runner.log"

for i in 2 3 4 5 6; do
    write_service "actions-runner-${i}" "$RUNNER_ROOT/runner-${i}" \
        "GitHub Actions Runner (nixos-x64-runner-${i})" "$LOG_DIR/runner-${i}.log"
done

# Legacy atomic-lang runner (separate repo)
if [[ -f "$HOME/actions-runner/.runner" ]]; then
    write_service "actions-runner-atomic-lang" "$HOME/actions-runner" \
        "GitHub Actions Runner (atomic-lang / nixos-runner)" \
        "$LOG_DIR/atomic-lang.log" "default.target"
fi

systemctl --user daemon-reload

if [[ "$ENABLE_NOW" -eq 1 ]]; then
    # Stop manual/nohup listeners so systemd can bind cleanly.
    for dir in "$RUNNER_ROOT/runner" "$RUNNER_ROOT"/runner-[2-6] "$HOME/actions-runner"; do
        [[ -d "$dir" ]] || continue
        if pgrep -f "$dir/bin/Runner.Listener run" >/dev/null 2>&1; then
            echo "stopping existing listener in $dir"
            pkill -f "$dir/bin/Runner.Listener run" || true
        fi
    done
    sleep 2

    systemctl --user enable actions-runners.target
    if [[ -f "$UNIT_DIR/actions-runner-atomic-lang.service" ]]; then
        systemctl --user enable actions-runner-atomic-lang.service
    fi
    systemctl --user restart actions-runners.target
    if [[ -f "$UNIT_DIR/actions-runner-atomic-lang.service" ]]; then
        systemctl --user restart actions-runner-atomic-lang.service
    fi
    sleep 2
    systemctl --user --no-pager status actions-runners.target || true
    echo ""
    gh api repos/TetraploidHuman/Action/actions/runners \
        --jq '.runners[] | "\(.name) status=\(.status) busy=\(.busy)"' 2>/dev/null \
        || echo "tip: gh api repos/TetraploidHuman/Action/actions/runners"
else
    echo ""
    echo "Units installed. Enable on boot:"
    echo "  systemctl --user enable --now actions-runners.target"
    echo "  systemctl --user enable --now actions-runner-atomic-lang.service  # if present"
    echo "Ensure linger: loginctl enable-linger \$USER"
fi
