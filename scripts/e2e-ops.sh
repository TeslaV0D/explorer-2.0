#!/bin/bash
# E2E for M4 file operations (Linux, Xvfb, no window manager). Called by e2e.sh,
# runnable alone: xvfb-run -a -s "-screen 0 1280x800x24" scripts/e2e-ops.sh <exe> <workdir>
set -u
EXE=$1; W=$2; FAIL=0
pass() { echo "PASS  $1"; }
fail() { echo "FAIL  $1"; FAIL=1; }
check() { if eval "$2"; then pass "$1"; else fail "$1"; fi; }

# ---- layout (default 1200x720 window, see main.slint / e2e.sh) ----
TOP_Y=20; NEWFOLDER_X=185; PASTE_X=471; UNDO_X=718
NAV_Y=94; LIST_X=500; ROW0_Y=140
row_y() { echo $((ROW0_Y + 24 * $1 + 12)); }
JOB_Y=672; JOB_PAUSE_X=1060; JOB_CANCEL_X=1150   # operations panel row above the status bar

export EXPLORER2_TRACE=1 SLINT_BACKEND=winit-software EXPLORER2_CONFIG_DIR=$W/config-ops HOME=$W/home-ops
mkdir -p "$HOME"
shot() { command -v import >/dev/null && import -window root "$W/$1.png" || true; }
focus() { xdotool windowfocus --sync "$(xdotool search --name 'Explorer 2.0' | head -1)"; }
last() { grep "$1" $LOG | tail -1; }
navigated_to() { grep -qE "navigate tab=[0-9]+ gen=[0-9]+ User $1 at=" $LOG; }
goto() { xdotool key ctrl+l; sleep 0.3; xdotool key ctrl+a; xdotool type --delay 5 "$1"; xdotool key Return; sleep 1.2; }
sel_is() { last 'selection pane' | grep -q "$1"; }
TRASH=$HOME/.local/share/Trash/files

# Unique folder per run (no rm -rf of variables, see CLAUDE.md)
R=$W/m4-$$; SRC=$R/src; DST=$R/dst
mkdir -p "$SRC/Ordner" "$DST"
echo a > "$SRC/a.txt"; echo neu > "$SRC/b.txt"; echo c > "$SRC/c.txt"; echo i > "$SRC/Ordner/inner.txt"
echo alt > "$DST/b.txt"
# rows (name sort, folders first): 0 Ordner, 1 a.txt, 2 b.txt, 3 c.txt

LOG=$W/trace-38.log
echo "== [38-52] M4: rename, new folder, clipboard, conflicts, trash, undo/redo, DnD, jobs"
$EXE "$SRC" > $LOG 2>&1 & PID=$!
sleep 2.5; focus

xdotool mousemove $LIST_X $(row_y 1) click 1; sleep 0.4
xdotool key F2; sleep 0.5
check "[38] F2 opens inline rename on a.txt" "last 'rename start' | grep -q 'name=a.txt'"
xdotool type --delay 30 "umbenannt"; xdotool key Return; sleep 1.2
check "[38b] typing replaces only the stem (a.txt -> umbenannt.txt)" "[ -f '$SRC/umbenannt.txt' ] && [ ! -e '$SRC/a.txt' ]"
check "[38c] renamed item stays selected" "sel_is 'focus=umbenannt.txt'"
xdotool key ctrl+z; sleep 1
check "[39] Ctrl+Z undoes the rename" "[ -f '$SRC/a.txt' ] && [ ! -e '$SRC/umbenannt.txt' ]"
xdotool key ctrl+y; sleep 1
check "[39b] Ctrl+Y redoes it" "[ -f '$SRC/umbenannt.txt' ] && [ ! -e '$SRC/a.txt' ]"
xdotool key ctrl+z; sleep 1
check "[39c] undo again" "[ -f '$SRC/a.txt' ]"
xdotool mousemove $LIST_X $(row_y 1) click 1; sleep 0.3
xdotool key F2; sleep 0.4; xdotool type --delay 20 "x"; xdotool key Escape; sleep 0.6
check "[39d] Esc cancels the rename" "[ -f '$SRC/a.txt' ] && grep -q 'rename cancelled' $LOG"
xdotool key F2; sleep 0.4; xdotool key ctrl+a; xdotool type --delay 20 "b.txt"; xdotool key Return; sleep 1
check "[39e] rename onto an existing name is refused, editor stays open" "[ -f '$SRC/a.txt' ] && last 'rename failed' | grep -q 'bereits' && last 'rename start' | grep -q 'name=a.txt'"
xdotool key Escape; sleep 0.5

xdotool key ctrl+shift+n; sleep 1.2
check "[40] Ctrl+Shift+N creates 'Neuer Ordner' and starts renaming it" "[ -d '$SRC/Neuer Ordner' ] && last 'rename start' | grep -q 'name=Neuer Ordner'"
xdotool type --delay 30 "Projekt"; xdotool key Return; sleep 1.2
check "[40b] new folder named 'Projekt'" "[ -d '$SRC/Projekt' ] && [ ! -e '$SRC/Neuer Ordner' ]"
# rows now: 0 Ordner, 1 Projekt, 2 a.txt, 3 b.txt, 4 c.txt
xdotool mousemove $LIST_X $(row_y 2) click 1; sleep 0.3
xdotool key ctrl+Down ctrl+space; sleep 0.4
check "[41] a.txt + b.txt selected" "sel_is 'count=2 '"
xdotool key ctrl+c; sleep 0.4
check "[41b] Ctrl+C fills the clipboard" "last 'clipboard' | grep -q 'clipboard copy n=2'"
goto "$DST"
xdotool key ctrl+v; sleep 1.5; shot o42_conflict
check "[42] conflict dialog for b.txt" "last 'conflict dialog' | grep -q 'name=b.txt dirs=false'"
xdotool key b; sleep 1.5
check "[42b] 'Beide behalten' keeps the old file and adds 'b (2).txt'" "[ \"\$(cat '$DST/b.txt')\" = alt ] && [ \"\$(cat '$DST/b (2).txt')\" = neu ] && [ -f '$DST/a.txt' ]"
check "[42c] pasted items are selected" "sel_is 'count=2 '"
check "[42d] source untouched" "[ -f '$SRC/a.txt' ] && [ -f '$SRC/b.txt' ]"
xdotool key ctrl+z; sleep 1.5
check "[43] Ctrl+Z removes the pasted copies (to the trash)" "[ ! -e '$DST/a.txt' ] && [ ! -e '$DST/b (2).txt' ] && [ \"\$(cat '$DST/b.txt')\" = alt ] && [ -f '$TRASH/a.txt' ]"

xdotool key ctrl+v; sleep 1.2
xdotool key r; sleep 1.5
check "[44] 'Ersetzen' replaces b.txt, old version goes to the trash" "[ \"\$(cat '$DST/b.txt')\" = neu ] && grep -qx alt $TRASH/b*.txt"
xdotool key ctrl+z; sleep 1.5
check "[44b] undo of 'Ersetzen' brings the old b.txt back" "[ \"\$(cat '$DST/b.txt')\" = alt ] && [ ! -e '$DST/a.txt' ]"

xdotool key alt+Left; sleep 1.2
xdotool mousemove $LIST_X $(row_y 4) click 1; sleep 0.3     # c.txt
xdotool key ctrl+x; sleep 0.5
check "[45] Ctrl+X marks for move" "last 'clipboard' | grep -q 'clipboard cut n=1'"
xdotool key alt+Right; sleep 1.2
xdotool key ctrl+v; sleep 1.5
check "[45b] paste after cut moves the file" "[ -f '$DST/c.txt' ] && [ ! -e '$SRC/c.txt' ]"
xdotool key ctrl+v; sleep 1
check "[45c] a cut is pasted only once" "last 'paste' | grep -q 'paste: clipboard empty'"

# dst rows: 0 b.txt, 1 c.txt
xdotool mousemove $LIST_X $(row_y 1) click 1; sleep 0.3
xdotool key shift+Delete; sleep 0.8; shot o46_confirm
check "[46] Shift+Del asks for confirmation" "grep -q 'command delete-permanent' $LOG && [ -f '$DST/c.txt' ]"
xdotool key Escape; sleep 0.5
check "[46b] Esc keeps the file" "[ -f '$DST/c.txt' ] && last 'confirm' | grep -q 'confirm false'"
xdotool key shift+Delete; sleep 0.6; xdotool key Return; sleep 1.2
check "[46c] confirmed: deleted permanently (not in the trash)" "[ ! -e '$DST/c.txt' ] && [ ! -e '$TRASH/c.txt' ]"

xdotool mousemove $LIST_X $(row_y 0) click 1; sleep 0.3      # b.txt
xdotool key Delete; sleep 1.2
check "[47] Del moves to the trash" "[ ! -e '$DST/b.txt' ] && grep -lx alt $TRASH/b* >/dev/null"
xdotool key ctrl+z; sleep 1.5
check "[47b] Ctrl+Z restores it from the trash" "[ \"\$(cat '$DST/b.txt')\" = alt ]"

xdotool mousemove $NEWFOLDER_X $TOP_Y click 1; sleep 1.2
check "[48] toolbar 'Neuer Ordner' works too" "[ -d '$DST/Neuer Ordner' ] && last 'rename start' | grep -q 'name=Neuer Ordner'"
xdotool key Return; sleep 0.8
check "[48b] Enter keeps the default name" "[ -d '$DST/Neuer Ordner' ] && grep -q 'rename unchanged' $LOG"

echo "-- drag & drop"
goto "$SRC"
# rows: 0 Ordner, 1 Projekt, 2 a.txt, 3 b.txt
xdotool mousemove $LIST_X $(row_y 2) mousedown 1; sleep 0.2
for dy in 8 16 24 32 40 44; do xdotool mousemove $LIST_X $(( $(row_y 2) - dy )); sleep 0.05; done
xdotool mousemove $LIST_X $(row_y 0); sleep 0.4; shot o49_drag
xdotool mouseup 1; sleep 1.5
check "[49] drag a.txt onto folder 'Ordner' moves it" "[ -f '$SRC/Ordner/a.txt' ] && [ ! -e '$SRC/a.txt' ]"
# rows: 0 Ordner, 1 Projekt, 2 b.txt
xdotool mousemove $LIST_X $(row_y 2) mousedown 1; sleep 0.2
xdotool keydown ctrl
for dy in 8 16 24; do xdotool mousemove $LIST_X $(( $(row_y 2) - dy )); sleep 0.05; done
xdotool mousemove $LIST_X $(row_y 1); sleep 0.4
xdotool mouseup 1; xdotool keyup ctrl; sleep 1.5
check "[49b] Ctrl+drag onto 'Projekt' copies" "[ -f '$SRC/Projekt/b.txt' ] && [ -f '$SRC/b.txt' ]"
xdotool mousemove $LIST_X $(row_y 2) mousedown 1; sleep 0.2
for dy in 8 16; do xdotool mousemove $LIST_X $(( $(row_y 2) + dy )); sleep 0.05; done
xdotool mousemove $LIST_X 500; sleep 0.3; xdotool mouseup 1; sleep 1
check "[49c] drop in the same folder does nothing" "[ -f '$SRC/b.txt' ] && ! grep -q 'drop pane=0 row=-1 n=1 action=2' $LOG"

echo "-- context menu (Linux: no shell menu, the UI path is checked)"
# rows: 0 Ordner, 1 Projekt, 2 b.txt
xdotool mousemove $LIST_X $(row_y 0) click 1; sleep 0.3
xdotool mousemove $LIST_X $(row_y 2) click 3; sleep 0.6
check "[53] right-click on an unselected row selects it and opens the menu" "last 'context menu pane' | grep -q 'row=2 items=1' && sel_is 'count=1 focus=b.txt'"
xdotool mousemove $LIST_X 500 click 3; sleep 0.5
check "[53b] right-click on empty space = folder background menu" "last 'context menu pane' | grep -q 'row=-1 items=0'"
xdotool key shift+F10; sleep 0.5
check "[53c] Shift+F10 opens the menu at the focused row" "last 'context menu pane' | grep -q 'row=2 items=1'"
xdotool key ctrl+a; sleep 0.3; xdotool mousemove $LIST_X $(row_y 1) click 3; sleep 0.5
check "[53d] right-click inside a multi-selection keeps it" "last 'context menu pane' | grep -q 'row=1 items=3' && sel_is 'count=3 '"
xdotool mousemove $LIST_X $(row_y 2) click 1; sleep 0.3

echo "-- long job: operations panel, pause, cancel, undo"
python3 - "$R/many" <<'PY'
import os,sys
d=sys.argv[1]; os.mkdir(d)
for i in range(60000): open(os.path.join(d, f"f{i}.txt"),'w').write("x"*512)
PY
goto "$R"
# rows: 0 dst, 1 many, 2 src
xdotool mousemove $LIST_X $(row_y 1) click 1; sleep 0.3
xdotool key ctrl+c; sleep 0.3
xdotool mousemove $LIST_X $(row_y 0) click --repeat 2 --delay 80 1; sleep 1.2   # into dst
xdotool key ctrl+v; sleep 1.0; shot o50_panel
check "[50] operations panel appears for a long job" "grep -q 'job panel shown' $LOG"
xdotool mousemove $JOB_PAUSE_X $JOB_Y click 1; sleep 0.6
N1=$(ls "$DST/many" 2>/dev/null | wc -l); sleep 1; N2=$(ls "$DST/many" 2>/dev/null | wc -l)
check "[50b] pause stops progress ($N1 -> $N2)" "last 'job pause' | grep -q 'true' && [ $N1 = $N2 ]"
xdotool mousemove $JOB_CANCEL_X $JOB_Y click 1; sleep 1.5
check "[50c] cancel ends the job" "last 'job done' | grep -q 'cancelled=true'"
check "[50d] no partial files left" "[ -z \"\$(find '$DST' -name '*.e2part' | head -1)\" ]"
check "[50e] source complete" "[ \$(ls '$R/many' | wc -l) = 60000 ]"
xdotool key ctrl+z; sleep 2
check "[51] undo removes the partial copy" "[ ! -e '$DST/many' ]"
check "[52] process alive" "kill -0 $PID"
kill $PID; wait $PID 2>/dev/null
check "[52b] no panic" "! grep -qi panic $LOG"
exit $FAIL
