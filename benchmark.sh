#!/usr/bin/env bash
set -uo pipefail
RED='\033[0;31m';    GREEN='\033[0;32m'
YELLOW='\033[1;33m'; BLUE='\033[0;34m'
BOLD='\033[1m';       NC='\033[0m'
SRC_DIR="$(cd "$(dirname "$0")" && pwd)"
ITERATIONS="${ACTION_BENCH_ITER:-3}"
WARMUP="${ACTION_BENCH_WARMUP:-1}"
MODE="${ACTION_BENCH_MODE:-run}"   # run | aot
OPT="${ACTION_BENCH_OPT:-0}"
RESULTS_FILE="benchmark_results.txt"
BENCH_ONLY="${ACTION_BENCH_ONLY:-}"
LIST_ONLY=false; BUILD=false; PROFILE=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --iterations|-n) ITERATIONS="$2"; shift 2 ;;
        --iterations=*)  ITERATIONS="${1#*=}"; shift   ;;
        --results)       RESULTS_FILE="$2";    shift 2 ;;
        --results=*)     RESULTS_FILE="${1#*=}";shift   ;;
        --mode)          MODE="$2";            shift 2 ;;
        --mode=*)        MODE="${1#*=}";        shift   ;;
        --opt|-O)        OPT="$2";             shift 2 ;;
        --opt=*)         OPT="${1#*=}";         shift   ;;
        --warmup)        WARMUP=1;             shift   ;;
        --no-warmup)     WARMUP=0;             shift   ;;
        --list|-l)       LIST_ONLY=true;       shift   ;;
        --build|-b)      BUILD=true;           shift   ;;
        --profile|-p)    PROFILE=true;         shift   ;;
        --only)          BENCH_ONLY="$2";      shift 2 ;;
        --only=*)        BENCH_ONLY="${1#*=}"; shift   ;;
        --help|-h)
            cat <<'EOF'
Action Language benchmark suite

Usage: ./benchmark.sh [options]

Options:
  -n, --iterations N   Timed iterations per benchmark (default: 3)
  --mode run|aot       run: action run (JIT, default)
                       aot: compile exe once, time pure execution
  -O, --opt N          LLVM opt level 0-3 (default: 0)
  --warmup             Run one discarded warmup per benchmark (default)
  --no-warmup          Disable warmup
  --results FILE       Output file (default: benchmark_results.txt)
  -b, --build          cargo build --release before benchmarking
  --only LIST            Comma-separated bench names (e.g. bench_cow,bench_all)
  -l, --list           List benchmark programs
  -p, --profile        Pass --profile to action run (run mode only)
  -h, --help           Show this help

Environment:
  ACTION_BENCH_ITER, ACTION_BENCH_MODE, ACTION_BENCH_OPT, ACTION_BENCH_WARMUP
EOF
            exit 0 ;;
        *) echo -e "${RED}unknown: $1${NC}"; exit 1 ;;
    esac
done
find_action_bin() {
    local root="$SRC_DIR/target"
    if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
        root="$CARGO_TARGET_DIR"
        if [[ -n "${TARGET:-}" ]]; then
            root="$root/$TARGET"
        fi
    fi
    local b="$root/release/action"; [[ -x "$b" ]] && { echo "$b"; return 0; }
    b="$root/debug/action"; [[ -x "$b" ]] && { echo "$b"; return 0; }
    echo ""; return 1
}
bench_exe_path() {
    local f="$1"
    echo "${f%.ac}"
}
# AOT: emit+link once per benchmark (not timed).
compile_aot() {
    local bench_file="$1"
    local exe; exe="$(bench_exe_path "$bench_file")"
    rm -f "$exe"
    if ! "$ACTION_BIN" run "-O$OPT" --emit exe "$bench_file" >/dev/null 2>&1; then
        return 1
    fi
    [[ -x "$exe" ]]
}
run_bench() {
    local bench_file="$1"
    if [[ "$MODE" == "aot" ]]; then
        "$(bench_exe_path "$bench_file")" >/dev/null 2>&1
        return $?
    fi
    local profile_arg=()
    $PROFILE && profile_arg=(--profile)
    "$ACTION_BIN" run "-O$OPT" "${profile_arg[@]}" "$bench_file" >/dev/null 2>&1
}
$BUILD && { echo -e "${BLUE}[build] release...${NC}"; (cd "$SRC_DIR" && cargo build --release); echo ""; }
ACTION_BIN="$(find_action_bin)"
if [[ -z "$ACTION_BIN" ]]; then
    echo -e "${YELLOW}[build] release...${NC}"; (cd "$SRC_DIR" && cargo build --release)
    ACTION_BIN="$(find_action_bin)"
fi
mapfile -t BENCH_FILES < <(ls "$SRC_DIR/examples"/bench_*.ac 2>/dev/null | sort)
if [[ -n "$BENCH_ONLY" ]]; then
    IFS=',' read -ra ONLY_NAMES <<< "$BENCH_ONLY"
    filtered=()
    for bench_file in "${BENCH_FILES[@]}"; do
        name="$(basename "$bench_file" .ac)"
        for want in "${ONLY_NAMES[@]}"; do
            if [[ "$name" == "$want" ]]; then
                filtered+=("$bench_file")
                break
            fi
        done
    done
    BENCH_FILES=("${filtered[@]}")
fi
$LIST_ONLY && { echo -e "${BOLD}bench programs (${#BENCH_FILES[@]}):${NC}"; for f in "${BENCH_FILES[@]}"; do echo "  $(basename "$f")"; done; exit 0; }
echo -e "${BOLD}==============================================${NC}"
echo -e "${BOLD}  Action Language — 性能测试套件${NC}"
echo -e "${BOLD}==============================================${NC}"
echo -e "  binary:      ${ACTION_BIN}"
echo -e "  mode:        ${MODE}$([[ "$MODE" == "aot" ]] && echo ' (timed: exe only)')"
echo -e "  opt:         -O${OPT}"
echo -e "  warmup:      ${WARMUP}"
echo -e "  iterations:  ${ITERATIONS}"
echo -e "  count:       ${#BENCH_FILES[@]}"
echo ""
RESULTS_PATH="$SRC_DIR/$RESULTS_FILE"
AOT_NOTE=""
[[ "$MODE" == "aot" ]] && AOT_NOTE=" | timed: exe-only (compile excluded)"
cat > "$RESULTS_PATH" << FOE
# Action Language Benchmark Results
# $(date '+%Y-%m-%d %H:%M:%S') | binary: $ACTION_BIN | mode: $MODE | opt: -O$OPT | warmup: $WARMUP | iterations: $ITERATIONS${AOT_NOTE}
# benchmark          min(ms)  avg(ms)  max(ms)  status
FOE
printf "${BOLD}%-38s %10s %10s %10s  %s${NC}\n" "Benchmark" "Min (ms)" "Avg (ms)" "Max (ms)" "Status"
printf "%s\n" "───────────────────────────────────────────────────────────────────────────"
PASS=0; FAIL=0; TOTAL=${#BENCH_FILES[@]}
for bench_file in "${BENCH_FILES[@]}"; do
    name="$(basename "$bench_file" .ac)"; times=(); ok=true
    if [[ "$MODE" == "aot" ]]; then
        compile_aot "$bench_file" || ok=false
    fi
    if $ok && [[ "$WARMUP" -gt 0 ]]; then
        run_bench "$bench_file" || { ok=false; }
    fi
    if $ok; then
        for ((i=0; i<ITERATIONS; i++)); do
            start=$(date +%s%N)
            if run_bench "$bench_file"; then
                end=$(date +%s%N); times+=($(( (end - start) / 1000000 )))
            else ok=false; break; fi
        done
    fi
    if ! $ok; then
        printf "${RED}%-38s %s${NC}\n" "$name" "CRASH"
        printf "%-38s %8s %8s %8s  FAIL\n" "$name" "-" "-" "-" >> "$RESULTS_PATH"
        FAIL=$((FAIL+1))
        continue
    fi
    min=${times[0]}; max=${times[0]}; sum=0; n=${#times[@]}
    for t in "${times[@]}"; do (( t < min )) && min=$t; (( t > max )) && max=$t; (( sum += t )); done
    avg=$(( (sum + n/2) / n ))
    printf "%-38s %8d ms %8d ms %8d ms  ${GREEN}PASS${NC}\n" "$name" "$min" "$avg" "$max"
    printf "%-38s %8d %8d %8d  PASS\n" "$name" "$min" "$avg" "$max" >> "$RESULTS_PATH"
    PASS=$((PASS+1))
    if [[ "$MODE" == "aot" ]]; then
        rm -f "$(bench_exe_path "$bench_file")" "${bench_file%.ac}.o"
    fi
done
echo ""; echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "  TOTAL: ${TOTAL}  |  ${GREEN}PASS: ${PASS}${NC}  |  ${RED}FAIL: ${FAIL}${NC}"
echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""; echo -e "${GREEN}results: ${BOLD}$RESULTS_PATH${NC}"
[[ "$FAIL" -gt 0 ]] && exit 1
exit 0
