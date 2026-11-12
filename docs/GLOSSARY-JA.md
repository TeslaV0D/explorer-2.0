# Glossary: Japanese (ja)

Localisation stage L6 (D-029, `docs/PLAN-I18N.md`). Catalog: `lang/ja/LC_MESSAGES/explorer2.po` (complete, offered as “日本語”).
Variant: **Japanese (Japan), ja-JP**, as Windows 11 Explorer shows it (Microsoft’s spaced compounds: “ネットワーク ドライブ”, “クイック アクセス”).
**Native-speaker review still open**: please look over the CI artifact `ui-gallery-linux-l10n` (folder `ja`).

## Core terms (Windows Explorer ja-JP)
| English | 日本語 | Note |
|---|---|---|
| File Explorer | エクスプローラー | our app keeps its name “Explorer 2.0” |
| Recycle Bin | ごみ箱 | |
| Empty Recycle Bin | ごみ箱を空にする | |
| This PC | PC | Windows 11; Windows 10 said “この PC” |
| Quick access | クイック アクセス | |
| Home (user folder) | 個人用フォルダー | not “ホーム” (that is the Windows 11 start page) |
| Desktop / Documents / Downloads / Pictures / Music / Videos | デスクトップ / ドキュメント / ダウンロード / ピクチャ / ミュージック / ビデオ | |
| Libraries | ライブラリ | |
| Local Disk / USB Drive / DVD Drive / Network Drive | ローカル ディスク / USB ドライブ / DVD ドライブ / ネットワーク ドライブ | |
| File folder / Folder | ファイル フォルダー / フォルダー | type name vs the shorter word |
| {} File | {} ファイル | “TXT ファイル” |
| Shortcut | ショートカット | “Shortcut or link” → “ショートカットまたはリンク” |
| Name / Date modified / Type / Size | 名前 / 更新日時 / 種類 / サイズ | column headers as in Explorer |
| Date deleted / Original location | 削除日 / 元の場所 | Recycle Bin columns |
| Cut / Copy / Paste / Rename / Delete | 切り取り / コピー / 貼り付け / 名前の変更 / 削除 | |
| Delete permanently | 完全に削除 | |
| Properties | プロパティ | |
| New folder / New tab | 新しいフォルダー / 新しいタブ | |
| Undo / Redo / Cancel | 元に戻す / やり直し / キャンセル | three different words |
| Refresh | 最新の情報に更新 | |
| Select all | すべて選択 | |
| Show hidden items | 隠し項目を表示する | |
| File name extensions | ファイルの拡張子 | |
| Details / Medium / Large / Extra large icons | 詳細 / 中アイコン / 大アイコン / 特大アイコン | |
| Sort / Ascending / Descending | 並べ替え / 昇順 / 降順 | |
| Search | 検索 | “Search all drives” → “すべてのドライブを検索” |
| Settings | 設定 | |
| Folder options | フォルダー オプション | |
| Address bar | アドレス バー | |
| Command palette | コマンド パレット | |
| Inspector | インスペクター | |
| Color scheme / Light / Dark / System default | 配色 / ライト / ダーク / システムの既定 | |
| Replace / Merge / Skip / Keep both / Cancel | 置き換え / 結合 / スキップ / 両方保持 / キャンセル | conflict dialog |
| OK / Close | OK / 閉じる | |
| Restore / Reset | 復元 / リセット | |
| Windows integration | Windows 統合 | |
| item | 個の項目 | one form for 0, 1 and 2: “{n} 個の項目”; drives use “{n} 台” |

## Keys
Key names stay Latin, as on Japanese Windows: **Ctrl**, **Shift**, **Alt**, **Del**, **Enter**. The keys themselves never change (Ctrl+A stays Ctrl+A). Arrow tooltips use 左/右/上.

Access keys in the conflict dialog (`core::mnemonic`, the `&` in the catalog). The letter stays visible, Windows-style: 置き換え(&R) shows as 置き換え(R). Keys: **R**eplace, **M**erge (結合), **S**kip (スキップ), **B**oth (両方保持), **A**ll (残りの競合すべてに適用). Esc = キャンセル.

Recovery tool prompt stays `[y/N]` (y/yes/j/ja are always accepted). The translated word for “yes” is “はい”.

## Numbers and units
`1,234.5` (comma for thousands, point for decimals), `explorer_i18n::separators("ja")`. Size units stay **B, KB, MB, GB, TB**; the spelled unit is “バイト” (“3.1 KB (3,170 バイト)”). One plural form (`nplurals=1`). Quotes use 「」.

## Deliberately not translated
- The Start menu entry “Explorer 2.0 – Windows-Standard wiederherstellen” and the folder verb texts written to the registry stay German (R-001 journal compares them; TODO “R-001-safe relabel”). Texts that name this entry quote it unchanged.
- Technical names: `EXPLORER2_INDEX`, `SLINT_BACKEND`, `fs.inotify.max_user_watches`, `shell:`, `crash` folder.
