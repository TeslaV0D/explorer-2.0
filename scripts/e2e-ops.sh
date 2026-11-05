#!/bin/bash
# E2E for M4 file operations (Linux, Xvfb, no window manager). Called by e2e.sh,
# runnable alone: xvfb-run -a -s "-screen 0 1280x800x24" scripts/e2e-ops.sh <exe> <workdir>
set -u
EXE=$1; W=$2; FAIL=0
pass() { echo "PASS  $1"; }
fail() { echo "FAIL  $1"; FAIL=1; }
check() { if eval "$2"; then pass "$1"; else fail "$1"; fi; }

# ---- layout (default 1200x720 window, see main.slint / e2e.sh) ----
TOP_Y=20; NEWFOLDER_X=185; PASTE_X=471; UNDO_X=718; SIDEBAR_X=80
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
echo "-- M5.6: recycle bin view, Dieser PC"
T=$R/t; mkdir -p "$T"; echo x > "$T/x.txt"; echo y > "$T/y.txt"
goto "$T"
# rows: 0 x.txt, 1 y.txt
xdotool mousemove $LIST_X $(row_y 0) click 1; sleep 0.3; xdotool key Delete; sleep 1.2
goto "Papierkorb"
check "[55] address alias 'Papierkorb' opens the recycle bin view" "last 'navigate tab' | grep -q ' ::papierkorb '"
check "[55b] trashed file listed with its original folder" "last 'kinds' | grep -qF '$T'"
xdotool type --delay 40 "x.txt"; sleep 0.5
check "[55c] type-ahead works in the trash" "sel_is 'focus=x.txt'"
xdotool mousemove 195 $TOP_Y click 1; sleep 1.5   # "Wiederherstellen"
check "[56] restore puts the file back" "[ \"\$(cat '$T/x.txt')\" = x ] && last 'trash Restore done' | grep -q 'ok=true'"
check "[56b] trash view refreshed (x.txt gone)" "! last 'kinds' | grep -q 'nothing' && ! ls $TRASH | grep -qx x.txt"
xdotool key ctrl+v; sleep 0.5
check "[56c] paste is refused in the trash" "last 'not available' | grep -q 'command paste not available in Papierkorb'"
goto "$T"
xdotool mousemove $LIST_X $(row_y 1) click 1; sleep 0.3; xdotool key Delete; sleep 1.2   # y.txt
xdotool mousemove $SIDEBAR_X 112 click 1; sleep 1.2   # sidebar "Papierkorb" (2nd quick-access item)
check "[57] sidebar 'Papierkorb' opens it" "[ \$(grep -c 'User ::papierkorb' $LOG) -ge 2 ]"
xdotool type --delay 40 "y.txt"; sleep 0.4; xdotool key Delete; sleep 0.6
xdotool key Return; sleep 1.5
check "[57b] Del in the trash = delete for good (after confirmation)" "last 'trash Purge done' | grep -q 'ok=true' && ! ls $TRASH | grep -qx y.txt && [ ! -e '$T/y.txt' ]"
xdotool mousemove 455 $TOP_Y click 1; sleep 0.6; xdotool key Return; sleep 2   # "Papierkorb leeren"
check "[57c] empty the trash" "[ -z \"\$(ls -A $TRASH 2>/dev/null)\" ]"
xdotool mousemove $SIDEBAR_X 150 click 1; sleep 1.5   # "Dieser PC" heading
check "[58] 'Dieser PC' lists the drives" "last 'navigate tab' | grep -q ' ::dieser-pc ' && last 'done tab' | grep -qE 'total=[1-9]'"
xdotool key Home Return; sleep 1.2
check "[58b] opening a drive navigates to its root" "last 'navigate tab' | grep -qE 'User / at='"
xdotool key BackSpace; sleep 1.2
check "[58c] up from the root goes to 'Dieser PC'" "last 'navigate tab' | grep -q ' ::dieser-pc '"
echo "-- M5.6c: network server path (Windows only, Linux: clear error)"
goto '\\localhost'
check "[59] server path gives a clear error, no crash" "last 'failed tab' | grep -qF 'localhost'"
check "[59b] the previous listing stays" "last 'address=' | grep -q 'address=::dieser-pc'"
echo "-- M5.6d: address bar ~, %VAR%, shell:"
goto '~'
check "[60] ~ opens the home folder" "last 'address=' | grep -qx '.*address=$HOME'"
goto '/' ; goto '%HOME%'
check "[60b] %HOME% is expanded" "last 'navigate tab' | grep -qF ' $HOME at='"
goto 'shell:gibtsnicht'
check "[60c] unknown shell: name gives a clear error" "last 'address error' | grep -q 'Unbekannter Shell-Ort: shell:gibtsnicht'"
echo "-- M5.6d2: libraries (Windows only, Linux: clear message)"
goto 'Bibliotheken'
check "[61] 'Bibliotheken' gives a clear message on Linux" "last 'failed tab' | grep -qF '::bibliotheken'"
goto 'shell:DocumentsLibrary'
check "[61b] shell:DocumentsLibrary resolves to the library place" "last 'navigate tab' | grep -qF '::bibliothek:Documents'"
echo "-- M5.7: icon view"
G=$R/g; mkdir -p "$G/Unterordner"; for i in $(seq 1 12); do echo x > "$G/f$i.txt"; done
goto "$G"
# items (natural sort, folders first): 0 Unterordner, i = f<i>.txt
xdotool key ctrl+shift+2; sleep 1
check "[62] Ctrl+Shift+2 switches to large icons" "last 'view mode' | grep -q 'mode=large'"
GEO=$(last 'geometry pane=0'); echo "   $GEO"
COLS=$(echo "$GEO" | sed -E 's/.* cols=([0-9]+).*/\1/'); GX=$(echo "$GEO" | sed -E 's/.* x=([0-9.]+).*/\1/'); GY=$(echo "$GEO" | sed -E 's/.* y=([0-9.]+).*/\1/')
GX=${GX%.*}; GY=${GY%.*}
check "[62a] the grid has several columns" "[ ${COLS:-0} -ge 3 ]"
tile_x() { echo $((GX + 128 * $1 + 64)); }
xdotool mousemove $(tile_x 2) $((GY + 50)) click 1; sleep 0.4
check "[62b] click on a tile selects it" "sel_is 'count=1 focus=f2.txt'"
xdotool key Right; sleep 0.3
check "[62c] → moves one item" "sel_is 'focus=f3.txt'"
xdotool key Down; sleep 0.3
check "[62d] ↓ moves one grid row" "sel_is 'focus=f$((3 + COLS)).txt'"
xdotool mousemove $(tile_x 0) $((GY + 50)) click --repeat 2 --delay 80 1; sleep 1.2
check "[62e] double-click on a folder tile opens it" "last 'navigate tab' | grep -qF '$G/Unterordner'"
xdotool key BackSpace; sleep 1.2
xdotool mousemove $(tile_x 1) $((GY + 50)) keydown ctrl click 5 keyup ctrl; sleep 0.8
check "[62f] Ctrl+wheel down = smaller icons" "last 'view mode' | grep -q 'mode=medium'"
check "[62g] the view is saved (for this folder)" "grep -qP '^medium\t$G$' $W/config-ops/views.ini"
xdotool key ctrl+shift+6; sleep 0.8
check "[62h] Ctrl+Shift+6 = details again" "last 'view mode' | grep -q 'mode=details' && last 'geometry pane=0' | grep -q 'cols=1 '"
echo "-- M6.1a: folder options (settings panel)"
F=$R/opt; mkdir -p "$F/Ordner/Tief"; echo x > "$F/x.txt"
goto "$F"
# rows: 0 Ordner, 1 x.txt
SETTINGS_BTN="1144 20"; OPT_EXT="835 205"; OPT_CLICK="835 257"; SETTINGS_CLOSE="1138 542"; VIEW_ALL="926 312"
xdotool mousemove $SETTINGS_BTN click 1; sleep 0.6
xdotool mousemove $OPT_EXT click 1; sleep 1.2
check "[63] 'Dateinamenerweiterungen anzeigen' off hides known extensions" "last 'option show_extensions' | grep -q '=false' && last 'shown' | grep -qF '\"x\"' && ! last 'shown' | grep -qF 'x.txt'"
xdotool mousemove $SETTINGS_CLOSE click 1; sleep 0.4
xdotool mousemove $LIST_X $(row_y 1) click 1; sleep 0.3; xdotool key F2; sleep 0.4
check "[63b] F2 edits the name without the hidden extension" "last 'rename start' | grep -q 'edit=x hidden_ext=txt'"
xdotool key ctrl+a; xdotool type --delay 30 "y"; xdotool key Return; sleep 1
check "[63c] … and keeps it: x.txt -> y.txt" "[ -f '$F/y.txt' ] && [ ! -e '$F/y' ] && [ ! -e '$F/x.txt' ]"
xdotool mousemove $SETTINGS_BTN click 1; sleep 0.6
xdotool mousemove $OPT_EXT click 1; sleep 1.2
check "[63d] extensions shown again" "last 'option show_extensions' | grep -q '=true' && last 'shown' | grep -qF 'y.txt'"
xdotool mousemove $OPT_CLICK click 1; sleep 0.4
xdotool mousemove $SETTINGS_CLOSE click 1; sleep 0.4
check "[64] single-click option saved" "grep -q 'single_click = true' $W/config-ops/settings.ini"
xdotool mousemove $LIST_X $(row_y 0) click 1; sleep 1.2
check "[64b] one click opens a folder" "last 'single-click open' | grep -q 'row=0' && last 'navigate tab' | grep -qF '$F/Ordner '"
xdotool key BackSpace; sleep 1.2
xdotool mousemove $LIST_X $(row_y 0) click --repeat 2 --delay 80 1; sleep 1.5
check "[64c] a double click opens only one level (Ordner, not Ordner/Tief)" "last 'navigate tab' | grep -qF '$F/Ordner ' && ! grep -qF '$F/Ordner/Tief' $LOG && grep -q 'single-click open ignored' $LOG"
xdotool key BackSpace; sleep 1.2
xdotool mousemove $LIST_X $(row_y 1) sleep 1.2
check "[64d] hovering selects (Markieren durch Zeigen)" "sel_is 'focus=y.txt'"
xdotool mousemove $SETTINGS_BTN click 1; sleep 0.6
xdotool mousemove $OPT_CLICK click 1; sleep 0.4
xdotool mousemove $SETTINGS_CLOSE click 1; sleep 0.4
check "[64e] single-click off again" "grep -q 'single_click = false' $W/config-ops/settings.ini"
echo "-- M6.1b: sort menu + rubber band"
# still in $F: rows 0 Ordner, 1 y.txt
SORT_BTN="1018 708"; SORT_NAME="1040 557"; SORT_SIZE="1040 629"; SORT_ASC="1040 654"; SORT_DESC="1040 678"
xdotool key ctrl+shift+2; sleep 1
check "[65] icon view hides the column header (list starts higher)" "last 'geometry pane=0' | grep -q ' y=114 '"
xdotool mousemove $SORT_BTN click 1; sleep 0.5; xdotool mousemove $SORT_SIZE click 1; sleep 0.8
check "[65b] sort menu: by size" "last 'sort key' | grep -q 'key=size desc=false'"
xdotool mousemove $SORT_BTN click 1; sleep 0.5; xdotool mousemove $SORT_DESC click 1; sleep 0.8
check "[65c] sort menu: descending keeps the column" "last 'sort key' | grep -q 'key=size desc=true'"
xdotool mousemove $SORT_BTN click 1; sleep 0.5; xdotool mousemove $SORT_NAME click 1; sleep 0.5
xdotool mousemove $SORT_BTN click 1; sleep 0.5; xdotool mousemove $SORT_ASC click 1; sleep 0.8
check "[65d] back to name ascending" "last 'sort key' | grep -q 'key=name desc=false'"
xdotool mousemove 1150 420 mousedown 1; sleep 0.2
for s in 1 2 3 4 5 6; do xdotool mousemove $((1150 - s * 140)) $((420 - s * 45)); sleep 0.08; done
xdotool mouseup 1; sleep 0.6
check "[66] rubber band in the icon view selects both tiles" "last 'band done' | grep -q 'hits=2' && sel_is 'count=2 '"
xdotool mousemove 1150 600 click 1; sleep 0.4
check "[66b] click on empty space clears it" "sel_is 'count=0 '"
xdotool key ctrl+shift+6; sleep 0.8
xdotool mousemove 600 420 mousedown 1; sleep 0.2
for s in 1 2 3 4 5; do xdotool mousemove 600 $((420 - s * 55)); sleep 0.08; done
xdotool mouseup 1; sleep 0.6
check "[66c] rubber band from below the rows in details" "last 'band done' | grep -q 'hits=2' && sel_is 'count=2 '"
xdotool mousemove $LIST_X $(row_y 0) click 1; sleep 0.3
xdotool mousemove 600 420 keydown ctrl mousedown 1; sleep 0.2
for s in 1 2 3 4 5; do xdotool mousemove 600 $((420 - s * 50)); sleep 0.08; done
xdotool mouseup 1 keyup ctrl; sleep 0.6
check "[66d] Ctrl + band adds to the selection" "sel_is 'count=2 '"
echo "-- M6.1c: view per folder"
VA=$R/va; VB=$R/vb; mkdir -p "$VA" "$VB"; echo a > "$VA/a.txt"; echo b > "$VB/b.txt"
goto "$VA"
xdotool key ctrl+shift+2; sleep 0.8
check "[67] the view is remembered for the folder" "grep -qP '^large\t$VA$' $W/config-ops/views.ini"
goto "$VB"
check "[67b] another folder keeps the default (details)" "last 'view follows folder' | grep -q 'mode=details' && last 'geometry pane=0' | grep -q 'cols=1 '"
xdotool key alt+Left; sleep 1.2
check "[67c] back in the first folder: large icons again" "last 'view follows folder' | grep -q 'mode=large'"
xdotool mousemove $SETTINGS_BTN click 1; sleep 0.6
xdotool mousemove $VIEW_ALL click 1; sleep 0.6
xdotool mousemove $SETTINGS_CLOSE click 1; sleep 0.4
check "[67d] 'Aktuelle Ansicht für alle Ordner' makes it the default" "last 'view applied to all' | grep -q 'mode=large' && grep -q 'view = large' $W/config-ops/settings.ini && [ ! -s $W/config-ops/views.ini ]"
goto "$VB"
check "[67e] … so the other folder now opens with large icons" "! last 'geometry pane=0' | grep -q 'cols=1 '"
xdotool key ctrl+shift+6; sleep 0.8
echo "-- M6.2: inspector"
I=$R/insp; mkdir -p "$I/Ordner/tief"; convert -size 640x480 xc:'#3070c0' "$I/bild.png"
printf 'Zeile eins\nZeile zwei\n' > "$I/notiz.txt"; head -c 3000 /dev/zero > "$I/Ordner/a.bin"; head -c 7000 /dev/zero > "$I/Ordner/tief/b.bin"
goto "$I"
xdotool key ctrl+shift+6; sleep 0.6   # [67d] made large icons the default
# rows: 0 Ordner, 1 bild.png, 2 notiz.txt
xdotool key alt+p; sleep 1.2
check "[68] Alt+P opens the inspector; nothing selected = the folder's direct content (no tree walk)" "last 'inspector open' | grep -q 'true' && last 'inspector rows' | grep -q 'Inhalt=2 Dateien, 1 Ordner'"
xdotool mousemove $LIST_X $(row_y 2) click 1; sleep 1
check "[68b] text file: preview + size" "last 'inspector done' | grep -q 'text=true' && last 'inspector done' | grep -q 'Größe=22 B'"
xdotool mousemove $LIST_X $(row_y 1) click 1; sleep 1
check "[68c] picture: dimensions from the file header" "last 'inspector done' | grep -q 'Abmessungen=640 × 480 Pixel'"
xdotool mousemove $LIST_X $(row_y 0) click 1; sleep 1.2
check "[68d] folder: files, subfolders and total size" "last 'inspector done' | grep -q 'Inhalt=2 Dateien, 1 Ordner' && last 'inspector done' | grep -q 'Gesamtgröße=9.8 KB'"
xdotool key ctrl+a; sleep 1.2
check "[68e] multi-selection: totals incl. the selected folder" "last 'inspector gen' | grep -q '3 Elemente' && last 'inspector done' | grep -q 'Gesamt=4 Dateien, 2 Ordner'"
check "[68f] the inspector state is saved" "grep -q 'inspector = true' $W/config-ops/settings.ini"
xdotool key alt+p; sleep 0.6
check "[68g] Alt+P closes it again" "last 'inspector open' | grep -q 'false'"
echo "-- M6.3: command palette"
PL=$R/pal; mkdir -p "$PL"; echo x > "$PL/x.txt"
goto "$PL"
xdotool key ctrl+shift+6; sleep 0.5   # [67d] made large icons the default
xdotool key ctrl+shift+p; sleep 0.5
check "[69] Ctrl+Shift+P opens the palette" "last 'palette open' >/dev/null && grep -q 'palette open' $LOG"
xdotool type --delay 40 "neord"; sleep 0.5
check "[69b] fuzzy: 'neord' finds 'Neuer Ordner' first" "last 'palette query' | grep -qF 'first=Some(\"Neuer Ordner\")'"
xdotool key Return; sleep 1.2
check "[69c] Enter runs it (same path as Ctrl+Shift+N: folder + inline rename)" "last 'palette run' | grep -q 'new-folder' && [ -d '$PL/Neuer Ordner' ] && last 'rename start' | grep -q 'Neuer Ordner'"
xdotool key Escape; sleep 0.4
xdotool key ctrl+shift+p; sleep 0.4; xdotool type --delay 40 "ansicht gro"; sleep 0.4; xdotool key Return; sleep 1
check "[69d] 'Ansicht: Große Symbole' from the palette" "last 'palette run' | grep -q 'view-large' && last 'view mode' | grep -q 'mode=large'"
xdotool key ctrl+shift+p; sleep 0.5
check "[69e] recently used commands come first" "last 'palette query' | grep -qF 'first=Some(\"Ansicht: Große Symbole\")' && grep -q 'recent_commands = view-large,new-folder' $W/config-ops/settings.ini"
xdotool key Escape; sleep 0.4
check "[69f] Esc closes without running anything" "last 'palette close' >/dev/null && last 'palette run' | grep -q 'view-large'"
xdotool key ctrl+shift+6; sleep 0.6
xdotool key ctrl+shift+p; sleep 0.4; xdotool type --delay 40 ">papier"; sleep 0.5; xdotool key Return; sleep 1.2
check "[69g] '>' jumps to places" "last 'palette go' | grep -q '::papierkorb' && last 'navigate tab' | grep -q '::papierkorb'"
xdotool key alt+Left; sleep 1
check "[52] process alive" "kill -0 $PID"
kill $PID; wait $PID 2>/dev/null
check "[52b] no panic" "! grep -qi panic $LOG"

echo "== [70] M6.4: colour schemes (light / dark / follow the system, accent colour)"
# measured by pixel (CLAUDE.md): list background, selection, a std-widget button
TW=$R/theme; mkdir -p "$TW"; echo a > "$TW/a.txt"; echo b > "$TW/b.txt"; echo c > "$TW/c.txt"
TCFG=$W/config-theme; mkdir -p "$TCFG"
px() { convert "$W/$1.png" -format "%[pixel:p{$2,$3}]" info: 2>/dev/null; }
red() { echo "$1" | sed -E 's/srgba?\(([0-9]+),.*/\1/'; }
green_wins() { echo "$1" | awk -F'[(),]' '{ exit !($3 > $2 + 20 && $3 > $4 + 20) }'; }
theme_run() { # <log> [env...]
  local log=$1; shift
  env EXPLORER2_CONFIG_DIR=$TCFG "$@" $EXE "$TW" > $log 2>&1 & PID=$!
  sleep 2.5; focus; LOG=$log
}
theme_run $W/trace-70.log
check "[70] first start follows the system (light here) with the original colours" "last 'theme why=init' | grep -q 'choice=system dark=false accent=#0067c0 selection=#cce8ff'"
xdotool mousemove 900 680; sleep 0.3; shot t70-light
P=$(px t70-light $LIST_X $(row_y 5)); B=$(px t70-light $NEWFOLDER_X 9)
check "[70b] light: list background #fafafa ($P), button light ($B)" "[ '$P' = 'srgb(250,250,250)' ] && [ \$(red '$B') -gt 200 ]"
T_DARK="1002 383"; T_ACCENT="835 414"   # settings panel: "Dunkel", "Windows-Akzentfarbe verwenden"
xdotool mousemove $SETTINGS_BTN click 1; sleep 0.6; xdotool mousemove $T_DARK click 1; sleep 0.8
check "[70c] 'Dunkel' in the settings switches live and is saved" "last 'theme why=choice' | grep -q 'choice=dark dark=true' && grep -qx 'theme = dark' $TCFG/settings.ini"
xdotool mousemove $SETTINGS_CLOSE click 1; sleep 0.4
xdotool mousemove $LIST_X $(row_y 0) click 1; sleep 0.4; xdotool mousemove 900 680; sleep 0.3; shot t70-dark
P=$(px t70-dark $LIST_X $(row_y 5)); S=$(px t70-dark $((LIST_X + 200)) $(row_y 0)); B=$(px t70-dark $NEWFOLDER_X 9); C=$(px t70-dark $SIDEBAR_X 600)
check "[70d] dark: list #191919 ($P), selection #1c3f63 ($S), sidebar #202020 ($C)" "[ '$P' = 'srgb(25,25,25)' ] && [ '$S' = 'srgb(28,63,99)' ] && [ '$C' = 'srgb(32,32,32)' ]"
check "[70e] the std widgets follow (dark button: $B)" "[ \$(red '$B') -lt 90 ]"
kill $PID; wait $PID 2>/dev/null
theme_run $W/trace-70f.log
check "[70f] restart starts dark right away (before the first frame)" "last 'theme why=init' | grep -q 'choice=dark dark=true' && ! grep -q 'theme why=init choice=[a-z]* dark=false' $LOG"
kill $PID; wait $PID 2>/dev/null
sed -i 's/^theme = .*/theme = system/' $TCFG/settings.ini
theme_run $W/trace-70g.log EXPLORER2_SYSTEM_THEME=dark EXPLORER2_ACCENT=#107c10
check "[70g] 'Wie Windows' follows a dark system with its accent" "last 'theme why=init' | grep -q 'choice=system dark=true .* system_dark=true system_accent=#107c10'"
xdotool mousemove $LIST_X $(row_y 1) click 1; sleep 0.4; xdotool mousemove 900 680; sleep 0.3; shot t70-green
S=$(px t70-green $((LIST_X + 200)) $(row_y 1))
check "[70h] the selection takes the accent's hue (green: $S)" "green_wins '$S'"
xdotool mousemove $SETTINGS_BTN click 1; sleep 0.6; xdotool mousemove $T_ACCENT click 1; sleep 0.8
check "[70h2] unticking 'Windows-Akzentfarbe' goes back to the blue" "last 'theme why=accent' | grep -q 'dark=true accent=#4cc2ff selection=#1c3f63' && grep -qx 'system_accent = false' $TCFG/settings.ini"
xdotool mousemove $T_ACCENT click 1; sleep 0.6; xdotool mousemove $SETTINGS_CLOSE click 1; sleep 0.4
xdotool key ctrl+shift+p; sleep 0.4; xdotool type --delay 40 "hell"; sleep 0.5; xdotool key Return; sleep 1
check "[70i] 'Hell' overrides the dark system" "last 'theme why=choice' | grep -q 'choice=light dark=false'"
xdotool key ctrl+shift+p; sleep 0.4; xdotool type --delay 40 "wie windows"; sleep 0.5; xdotool key Return; sleep 1
check "[70j] back to 'Wie Windows' = dark again" "last 'theme why=choice' | grep -q 'choice=system dark=true'"
check "[70k] process alive, no panic" "kill -0 $PID && ! grep -qi panic $LOG"
kill $PID; wait $PID 2>/dev/null

echo "== [71] R-001: Windows integration status + one-button reset (settings, palette)"
ICFG=$W/config-integration; mkdir -p "$ICFG"
T_RESET="956 502"   # settings panel: "Alle Windows-Integrationen zurücksetzen"
LOG=$W/trace-71.log
EXPLORER2_CONFIG_DIR=$ICFG $EXE "$TW" > $LOG 2>&1 & PID=$!
sleep 2.5; focus
xdotool mousemove $SETTINGS_BTN click 1; sleep 0.8
check "[71] opening the settings reads the integration status" "last 'integration status' | grep -q 'entries=0 active_points=0 journal=None'"
xdotool mousemove $SETTINGS_CLOSE click 1; sleep 0.4
xdotool key ctrl+shift+p; sleep 0.4; xdotool type --delay 40 "integrationen zur"; sleep 0.5; xdotool key Return; sleep 0.8
check "[71b] palette: nothing active -> no dialog, settings show the state" "last 'palette run' | grep -q 'integration-reset' && last 'integration reset: nothing active' >/dev/null && ! grep -q 'confirm integration reset' $LOG"
xdotool mousemove $SETTINGS_CLOSE click 1; sleep 0.4
# a damaged journal (e.g. after a crash): the reset must still work
printf 'v1\nvalue\tHKCU\tzz\n' > "$ICFG/integration-journal.txt"
xdotool mousemove $SETTINGS_BTN click 1; sleep 0.8
check "[71c] damaged journal is reported" "last 'integration status' | grep -q 'journal=Damaged'"
xdotool mousemove $T_RESET click 1; sleep 0.8
check "[71d] the button asks first" "last 'confirm integration reset' >/dev/null"
xdotool key Return; sleep 1
check "[71e] confirmed: reset ran, damaged journal removed" "last 'integration reset restored' | grep -q 'errors=0 journal=Damaged' && [ ! -e '$ICFG/integration-journal.txt' ]"
check "[71f] status afterwards: nothing active" "last 'integration status' | grep -q 'entries=0 active_points=0 journal=None'"
xdotool mousemove $SETTINGS_CLOSE click 1; sleep 0.4
# the same from the palette: the dialog must get the keyboard (Enter went nowhere, CI #60)
printf 'v1\nvalue\tHKCU\tzz\n' > "$ICFG/integration-journal.txt"
xdotool key ctrl+shift+p; sleep 0.4; xdotool type --delay 40 "integrationen zur"; sleep 0.5; xdotool key Return; sleep 0.8
xdotool key Return; sleep 1
check "[71f2] palette -> question -> Enter resets" "[ \$(grep -c 'integration reset restored' $LOG) -eq 2 ] && [ ! -e '$ICFG/integration-journal.txt' ]"
xdotool mousemove $SETTINGS_CLOSE click 1; sleep 0.4
check "[71g] process alive, no panic" "kill -0 $PID && ! grep -qi panic $LOG"
kill $PID; wait $PID 2>/dev/null

echo "== [72] M8: install from the app (palette -> question -> continue in the installed copy)"
IDIR=$R/inst/Programs/Explorer2; SDIR="$R/inst/Start/Explorer 2.0"
LOG=$W/trace-72.log
[ -x "$(dirname "$EXE")/explorer2-recover" ] || echo "NOTE  [72] explorer2-recover missing next to $EXE (build -p explorer-integrate --bins) - the installer refuses then"
EXPLORER2_CONFIG_DIR=$W/config-install EXPLORER2_INSTALL_DIR=$IDIR EXPLORER2_STARTMENU_DIR="$SDIR" \
  $EXE "$TW" > $LOG 2>&1 & PID=$!
sleep 2.5; focus
xdotool key ctrl+shift+p; sleep 0.4; xdotool type --delay 40 "installieren"; sleep 0.5; xdotool key Return; sleep 0.8
check "[72] palette 'installieren' asks first" "last 'confirm install' >/dev/null"
xdotool key Return; sleep 2.5
check "[72b] installed: program files + Start menu entries" "[ -x '$IDIR/explorer2' ] && [ -x '$IDIR/explorer2-recover' ] && [ -f '$SDIR/Explorer 2.0.lnk' ] && last 'install ok' | grep -q 'shortcuts=2'"
check "[72c] the old window closed, the installed copy runs" "! kill -0 $PID 2>/dev/null && pgrep -f '$IDIR/explorer2' >/dev/null"
pkill -f "$IDIR/explorer2" ; sleep 0.5
EXPLORER2_CONFIG_DIR=$W/config-install EXPLORER2_INSTALL_DIR=$IDIR EXPLORER2_STARTMENU_DIR="$SDIR" \
  "$IDIR/explorer2-recover" --uninstall --quiet > $W/uninstall-72.log 2>&1; sleep 2
check "[72d] uninstall removes program folder and Start menu entries" "[ ! -e '$IDIR' ] && [ ! -e '$SDIR' ]"
ls /tmp 2>/dev/null | grep explorer2-uninstall | sed 's|^|/tmp/|' | xargs -r rm -f

echo "== [73] M8.3: crash report + start guard (2 failed starts -> safe start)"
CC=$W/config-crash; CR=$CC/crash; LOG=$W/trace-73.log
mkdir -p "$CC" "$TW/safe-sub"
printf '[pane]\nactive_tab = 0\ntab = %s\n' "$TW/safe-sub" > "$CC/session.ini"
EXPLORER2_CONFIG_DIR=$CC EXPLORER2_TEST_CRASH=panic timeout 20 $EXE > $W/crash-73a.log 2>&1; A=$?
REP=$(ls $CR/crash-*.txt 2>/dev/null | head -1)
check "[73] a panic at start writes a report (exit $A)" "[ $A -ne 0 ] && [ -n '$REP' ] && grep -q 'Ursache: Panic' '$REP' && grep -q 'test crash requested' '$REP' && grep -q 'start guard' '$REP'"
EXPLORER2_CONFIG_DIR=$CC EXPLORER2_TEST_CRASH=native timeout 20 $EXE > $W/crash-73b.log 2>&1
check "[73b] two dead starts are counted" "grep -q 'failed=1' $CC/start-guard.txt"
EXPLORER2_CONFIG_DIR=$CC env -u SLINT_BACKEND $EXE > $LOG 2>&1 & PID=$!
sleep 2.5
check "[73c] third start is a safe one: software renderer, session not restored" "last 'start guard' | grep -q 'failed: 2 } safe=true' && last 'renderer choice' | grep -q 'safe start' && ! grep -q 'safe-sub' $LOG"
check "[73d] the notice names the crash and the safe start" "last 'notice ' | grep -q 'reports=1 safe=true' && ls $CR | grep -q 'seen.txt'"
shot crash-notice
sleep 4
check "[73e] after 5 s the start counts as stable (guard removed)" "grep -q 'start guard: stable' $LOG && [ ! -e $CC/start-guard.txt ]"
kill $PID; wait $PID 2>/dev/null
EXPLORER2_CONFIG_DIR=$CC env -u SLINT_BACKEND $EXE > $LOG 2>&1 & PID=$!
sleep 1.5
check "[73f] next start is normal again, no notice (report already shown)" "last 'start guard' | grep -q 'failed: 0 } safe=false' && ! last 'renderer choice' | grep -q 'safe start' && ! grep -q 'notice ' $LOG"
kill $PID; wait $PID 2>/dev/null
check "[73g] kill (SIGTERM) right after start is no crash" "[ ! -e $CC/start-guard.txt ]"
EXPLORER2_CONFIG_DIR=$CC EXPLORER2_TEST_CRASH=panic-later $EXE > $W/crash-73h.log 2>&1 & PID=$!
sleep 8
N=$(ls $CR | grep -c -v seen)
check "[73h] a panic in a worker thread during use: report with thread name, no start counted" "! kill -0 $PID 2>/dev/null && [ $N -eq 1 ] && grep -q 'Thread: test-crash' $CR/\$(ls $CR | grep -v seen) && [ ! -e $CC/start-guard.txt ]"
EXPLORER2_CONFIG_DIR=$CC $EXE > $LOG 2>&1 & PID=$!
sleep 1.5
check "[73i] next start: notice about the crash, but no safe start" "last 'notice ' | grep -q 'reports=1 safe=false' && last 'start guard' | grep -q 'safe=false'"
kill $PID; wait $PID 2>/dev/null
# a GPU probe that took its process down (marker left by a dead pid)
printf 'pid=999999\nfailed=0\n' > $CC/gpu-probe.txt
EXPLORER2_CONFIG_DIR=$CC $EXE > $LOG 2>&1 & PID=$!
sleep 1.5
check "[73j] a GPU probe that crashed the app marks the GPU unusable" "grep -q 'gpu probe crashed last time' $LOG && grep -q 'gpu_probe = unusable' $CC/settings.ini && [ ! -e $CC/gpu-probe.txt ]"
kill $PID; wait $PID 2>/dev/null
exit $FAIL
