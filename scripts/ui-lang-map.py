#!/usr/bin/env python3
"""German UI name -> the same name in another language (D-029), for the UI
scripts: they click German names (`ui_click "Einstellungen"`); with
EXPLORER2_LANG=<code> `ui-lib.sh` looks each name up here first.

  python3 -I scripts/ui-lang-map.py <repo> <code>   -> TSV "german<TAB>target" on stdout

German -> English source (lang/de), then English -> <code> (lang/<code>, missing
or English = the source text). Only single-line, placeholder-free texts.
"""
import sys
from pathlib import Path


def unquote(s):
    s = s.strip()[1:-1]
    out, i = [], 0
    while i < len(s):
        if s[i] == '\\':
            out.append({'n': '\n', 't': '\t', 'r': '\r', '"': '"', '\\': '\\'}[s[i + 1]])
            i += 2
        else:
            out.append(s[i])
            i += 1
    return ''.join(out)


def catalog(path):
    """msgid -> msgstr (singular entries only, translated only)."""
    out, cur, field = {}, {}, None
    def flush():
        if cur.get('msgid') and cur.get('msgstr') and 'msgid_plural' not in cur:
            out[cur['msgid']] = cur['msgstr']
    if not path.exists():
        return out
    for line in path.read_text(encoding='utf-8').splitlines() + ['']:
        t = line.strip()
        if not t:
            flush(); cur, field = {}, None
        elif t.startswith('#'):
            continue
        elif t.startswith('"') and field:
            cur[field] += unquote(t)
        else:
            kw, _, rest = t.partition(' ')
            field = kw
            cur[kw] = unquote(rest)
    return out


def main():
    repo, code = Path(sys.argv[1]), sys.argv[2]
    de = catalog(repo / 'lang/de/LC_MESSAGES/explorer2.po')
    target = {} if code == 'en' else catalog(repo / f'lang/{code}/LC_MESSAGES/explorer2.po')
    seen = set()
    for en, ge in de.items():
        if '\n' in ge or '{' in ge or ge in seen:
            continue
        seen.add(ge)
        tr = target.get(en, en)
        if tr != ge:
            sys.stdout.write(f"{ge}\t{tr}\n")


main()
