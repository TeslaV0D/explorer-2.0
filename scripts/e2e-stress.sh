#!/bin/bash
# M8.3c endurance (Linux, Xvfb, no window manager): many navigations, tabs
# opened/closed, view switches and file churn in the watched folder. Memory
# (RSS), open files and threads must stay flat after the warm-up.
# Called by e2e.sh, runnable alone:
#   xvfb-run -a -s "-screen 0 1280x800x24" scripts/e2e-stress.sh <exe> <workdir> [rounds]
set -u
EXE=$1; W=$2; N=${3:-150}; FAIL=0
pass() { echo "PASS  $1"; }
fail() { echo "FAIL  $1"; FAIL=1; }
check() { if eval "$2"; then pass "$1"; else fail "$1"; fi; }

export EXPLORER2_TRACE=1 SLINT_BACKEND=winit-software EXPLORER2_CONFIG_DIR=$W/config-stress HOME=$W/home-stress
mkdir -p "$HOME"
focus() { xdotool windowfocus --sync "$(xdotool search --name 'Explorer 2.0' | head -1)"; }

echo "== [75] M8.3c endurance: $N rounds (navigate, tab open/close, view switch, file churn)"
SD=$W/stress-$$; mkdir -p "$SD"
for i in $(seq -w 1 20); do
  mkdir -p "$SD/d$i"
  for j in $(seq 1 120); do : > "$SD/d$i/f$j.e$((j % 9))"; done
done
LOG=$W/trace-75.log
$EXE "$SD" > $LOG 2>&1 & PID=$!
sleep 2.5; focus
# RSS (KiB), open files, threads
stats() { echo "$(awk '/VmRSS/{print $2}' /proc/$PID/status) $(ls /proc/$PID/fd | wc -l) $(ls /proc/$PID/task | wc -l)"; }
cycle() {
  # 20 folders, then files: back to the top before Down passes the last folder
  [ $(( $1 % 19 )) -eq 0 ] && xdotool key Home
  xdotool key Down Return; sleep 0.12
  if [ $(( $1 % 5 )) -eq 0 ]; then xdotool key ctrl+shift+6; sleep 0.1; xdotool key ctrl+shift+1; fi
  if [ $(( $1 % 7 )) -eq 0 ]; then : > "$SD/churn-$1"; rm -f "$SD/churn-$(( $1 - 7 ))"; fi
  xdotool key BackSpace; sleep 0.12
  xdotool key ctrl+t; sleep 0.05; xdotool key ctrl+w; sleep 0.05
}
for i in $(seq 1 30); do cycle $i; done
sleep 1.5
read -r R1 F1 T1 <<< "$(stats)"
NAV1=$(grep -c 'navigate tab=' $LOG)
T0=$(date +%s)
for i in $(seq 31 $(( 30 + N ))); do cycle $i; done
sleep 2
read -r R2 F2 T2 <<< "$(stats)"
NAV2=$(grep -c 'navigate tab=' $LOG)
SECS=$(( $(date +%s) - T0 ))
echo "    after warm-up: rss=${R1} KiB fds=$F1 threads=$T1 | after $N more rounds (${SECS}s): rss=${R2} KiB fds=$F2 threads=$T2 | navigations $NAV1 -> $NAV2"
check "[75] the rounds really ran ($(( NAV2 - NAV1 )) navigations)" "[ $(( NAV2 - NAV1 )) -ge $N ]"
check "[75b] memory stays flat (+$(( (R2 - R1) / 1024 )) MiB, limit +24 MiB)" "[ $(( R2 - R1 )) -lt 24576 ]"
check "[75c] no leaked files/handles (+$(( F2 - F1 )), limit +3)" "[ $(( F2 - F1 )) -le 3 ]"
check "[75d] no leaked threads (+$(( T2 - T1 )), limit +2)" "[ $(( T2 - T1 )) -le 2 ]"
check "[75e] process alive, no panic" "kill -0 $PID 2>/dev/null && ! grep -qi 'panic' $LOG"
kill $PID; wait $PID 2>/dev/null

# D-028: a hidden window must not keep the folder listings. Linux CI has no
# tray, so hiding quits; EXPLORER2_TEST_PARK drives the same release/restore.
echo "== [75f] idle memory: drop the folder listing (4 park/resume cycles)"
BIG=$W/park-big
python3 -c "
import os
d = '$BIG'
os.makedirs(d, exist_ok=True)
for i in range(20000):
    fd = os.open(f'{d}/f{i:05d}.dat', os.O_CREAT | os.O_WRONLY, 0o644)
    os.close(fd)
"
CTL=$W/park-cmd
: > "$CTL"
LOG=$W/trace-75f.log
EXPLORER2_INDEX=off EXPLORER2_TEST_PARK=$CTL EXPLORER2_CONFIG_DIR=$W/config-park \
  $EXE "$BIG" > "$LOG" 2>&1 & PID=$!
wait_n() {
  local pat="$1" want="$2" i c=0
  for i in $(seq 1 80); do
    c=$(grep -c "$pat" "$LOG" || true)
    if [ "$c" -ge "$want" ]; then return 0; fi
    sleep 0.25
  done
  echo "    timeout: wanted $want x '$pat', have $c"
  return 1
}
count() { grep -c "$1" "$LOG" || true; }
rss() { awk '/VmRSS/{print $2}' /proc/$PID/status; }
fds() { ls /proc/$PID/fd 2>/dev/null | wc -l; }
threads() { ls /proc/$PID/task 2>/dev/null | wc -l; }
if wait_n "done tab=" 1; then
  sleep 0.4
  focus
  xdotool key Down
  sleep 0.3
else
  fail "[75f] the big folder loaded"
fi
declare -a HI LO FD TH
for n in 1 2 3 4; do
  echo park > "$CTL"
  if ! wait_n "parked entries=" "$n"; then fail "[75f] park $n"; break; fi
  sleep 0.3
  LO[$n]=$(rss)
  before=$(count "done tab=")
  echo resume > "$CTL"
  if ! wait_n "resumed" "$n"; then fail "[75f] resume $n"; break; fi
  ok=0
  for i in $(seq 1 40); do
    now=$(count "done tab=")
    if [ "$now" -gt "$before" ]; then ok=1; break; fi
    sleep 0.25
  done
  if [ "$ok" -ne 1 ]; then fail "[75f] reload $n"; break; fi
  sleep 0.3
  HI[$n]=$(rss)
  FD[$n]=$(fds)
  TH[$n]=$(threads)
done
echo "    parked KiB: ${LO[*]:-none} | shown KiB: ${HI[*]:-none} | fds ${FD[*]:-none} | threads ${TH[*]:-none}"
PARK=$(grep "parked entries=" "$LOG" | head -1 || true)
echo "    $PARK"
ENT=$(echo "$PARK" | sed -n 's/.*entries=\([0-9]*\).*/\1/p')
BYT=$(echo "$PARK" | sed -n 's/.*bytes=\([0-9]*\).*/\1/p')
SEL=$(echo "$PARK" | sed -n 's/.*selected=\([0-9]*\).*/\1/p')
check "[75f] park dropped the listing (${ENT:-0} entries, ${BYT:-0} bytes)" "[ \"${ENT:-0}\" -ge 15000 ] && [ \"${BYT:-0}\" -ge 1000000 ]"
check "[75g] the focused name is kept (selected=${SEL:-0})" "[ \"${SEL:-0}\" -ge 1 ]"
if [ -n "${HI[1]:-}" ] && [ -n "${LO[1]:-}" ] && [ -n "${HI[4]:-}" ] && [ -n "${LO[4]:-}" ]; then
  DROP=$(( HI[1] - LO[1] ))
  GROW=$(( HI[4] - HI[1] ))
  PGROW=$(( LO[4] - LO[1] ))
  echo "    drop on first park: $(( DROP / 1024 )) MiB | shown growth over 4 cycles: $(( GROW / 1024 )) MiB | parked growth: $(( PGROW / 1024 )) MiB"
  check "[75h] parking frees memory ($(( DROP / 1024 )) MiB, at least 2)" "[ $DROP -ge 2048 ]"
  check "[75i] reloading does not leak (shown +$(( GROW / 1024 )) MiB, parked +$(( PGROW / 1024 )) MiB, limit +16)" "[ $GROW -lt 16384 ] && [ $PGROW -lt 16384 ]"
  check "[75j] no leaked files or threads (fds +$(( FD[4] - FD[1] )), threads +$(( TH[4] - TH[1] )))" "[ $(( FD[4] - FD[1] )) -le 3 ] && [ $(( TH[4] - TH[1] )) -le 2 ]"
else
  fail "[75h] park/resume cycles did not finish"
fi
check "[75k] process alive, no panic" "kill -0 $PID 2>/dev/null && ! grep -qi 'panic' $LOG"
kill $PID; wait $PID 2>/dev/null
exit $FAIL
