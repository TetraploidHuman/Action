#!/usr/bin/env bash
set -uo pipefail
RED='\033[0;31m';    GREEN='\033[0;32m'
YELLOW='\033[1;33m'; BLUE='\033[0;34m'
BOLD='\033[1m';       NC='\033[0m'
SRC_DIR="$(cd "$(dirname "$0")" && pwd)"
ITERATIONS="${ACTION_BENCH_ITER:-3}"
RESULTS_FILE="benchmark_results.txt"
LIST_ONLY=false; BUILD=false; PROFILE=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --iterations|-n) ITERATIONS="$2"; shift 2 ;;
        --iterations=*)  ITERATIONS="${1#*=}"; shift   ;;
        --results)       RESULTS_FILE="$2";    shift 2 ;;
        --results=*)     RESULTS_FILE="${1#*=}";shift   ;;
        --list|-l)       LIST_ONLY=true;       shift   ;;
        --build|-b)      BUILD=true;           shift   ;;
        --profile|-p)    PROFILE=true;         shift   ;;
        --help|-h)
            sed -n '4,14p' "$0" | sed 's/^# \?//'; exit 0 ;;
        *) echo -e "${RED}unknown: $1${NC}"; exit 1 ;;
    esac
done
find_action_bin() {
    local b="$SRC_DIR/target/release/action"; [[ -x "$b" ]] && { echo "$b"; return 0; }
    b="$SRC_DIR/target/debug/action"; [[ -x "$b" ]] && { echo "$b"; return 0; }
    echo ""; return 1
}
$BUILD && { echo -e "${BLUE}[build] release...${NC}"; (cd "$SRC_DIR" && cargo build --release); echo ""; }
ACTION_BIN="$(find_action_bin)"
if [[ -z "$ACTION_BIN" ]]; then
    echo -e "${YELLOW}[build] release...${NC}"; (cd "$SRC_DIR" && cargo build --release)
    ACTION_BIN="$(find_action_bin)"
fi
mapfile -t BENCH_FILES < <(ls "$SRC_DIR/examples"/bench_*.at 2>/dev/null | sort)
$LIST_ONLY && { echo -e "${BOLD}bench programs (${#BENCH_FILES[@]}):${NC}"; for f in "${BENCH_FILES[@]}"; do echo "  $(basename "$f")"; done; exit 0; }
echo -e "${BOLD}==============================================${NC}"
echo -e "${BOLD}  Action Language — 性能测试套件${NC}"
echo -e "${BOLD}==============================================${NC}"
echo -e "  binary:      ${ACTION_BIN}"
echo -e "  iterations:  ${ITERATIONS}"
echo -e "  count:       ${#BENCH_FILES[@]}"
echo ""
RESULTS_PATH="$SRC_DIR/$RESULTS_FILE"
cat > "$RESULTS_PATH" << FOE
# Action Language Benchmark Results
# $(date '+%Y-%m-%d %H:%M:%S') | binary: $ACTION_BIN | iterations: $ITERATIONS
# benchmark          min(ms)  avg(ms)  max(ms)  status
FOE
printf "${BOLD}%-38s %10s %10s %10s  %s${NC}\n" "Benchmark" "Min (ms)" "Avg (ms)" "Max (ms)" "Status"
printf "%s\n" "───────────────────────────────────────────────────────────────────────────"
PASS=0; FAIL=0; TOTAL=${#BENCH_FILES[@]}
for bench_file in "${BENCH_FILES[@]}"; do
    name="$(basename "$bench_file" .at)"; times=(); ok=true
    for ((i=0; i<ITERATIONS; i++)); do
        start=$(date +%s%N)
        if ("$ACTION_BIN" run "$bench_file" >/dev/null 2>&1) 2>/dev/null; then
            end=$(date +%s%N); times+=($(( (end - start) / 1000000 )))
        else ok=false; break; fi
    done
    if ! $ok; then
        printf "${RED}%-38s %s${NC}\n" "$name" "CRASH"; FAIL=$((FAIL+1))
        continue
    fi
    min=${times[0]}; max=${times[0]}; sum=0
    for t in "${times[@]}"; do (( t < min )) && min=$t; (( t > max )) && max=$t; (( sum += t )); done
    avg=$(( (sum + ITERATIONS/2) / ITERATIONS ))
    printf "%-38s %8d ms %8d ms %8d ms  ${GREEN}PASS${NC}\n" "$name" "$min" "$avg" "$max"
    PASS=$((PASS+1))
done
echo ""; echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "  TOTAL: ${TOTAL}  |  ${GREEN}PASS: ${PASS}${NC}  |  ${RED}FAIL: ${FAIL}${NC}"
echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""; echo -e "${GREEN}results: ${BOLD}$RESULTS_PATH${NC}"
