# Plan: localisation (i18n)

As of 2026-11-10 · Boss: “mach die Lokalisation weiter” · decisions in D-029 (supersedes D-007) · **L0 ✅ (CI #141), L1 ✅ (CI #144), L2 ✅ (CI #146), L3 ✅ (CI #147), L4 ✅ (CI #149), L5 ✅ (CI #152), L6: es ✅ (CI #153), fr ✅ (CI #154), ru ✅ (CI #157), uk ✅ (CI #157), pl ✅ (CI #157), it ✅**, next zh.
Wish list from 2026-11-08 (`docs/TODO.md`): exact meanings like Windows Explorer in each language, layout robustness, language switchable in the settings (default: Windows language), numbers/dates per locale, mind font coverage.

## 1. Current state (inventory)
| Where | Amount | Notes |
|---|---|---|
| `crates/app/ui/*.slint` | ~400 string literals (not all are UI text: ids, icon names, places) | labels, tooltips, menus, dialogs, settings panel, `accessible-label`s |
| Rust (`app`, `core`, `ops`, `search`, `index`, `integrate`, `setup`, `fs`) | ~300–500 user-visible strings in ~40 files | status line, errors, dialogs, command names (`core::commands`), Inspector facts, type labels, search status, recovery tool, setup dialogs; many built with `format!` and German plurals (“1 Element” / “2 Elemente”) |
| Number/date formatting | `core::format_size`, `format_count`, `format_local` | German style hard-coded (`1.234.567`, `3,4 s`, `2026-11-10 06:44`) |
| Texts from Windows | shell type names, known folder names, Properties, native menu | already in the Windows language – nothing to do |
| Tests | 36 names in Linux E2E (`ui_click "Einstellungen"` …), 5 in Windows E2E, many trace/assert texts | tests must pin a language |

## 2. Technique (recommendation)
- **Slint:** every UI text as `@tr("…")` (plurals: `@tr("{n} item" | "{n} items" % n)`), **bundled translations** (`slint_build::CompilerConfiguration::with_bundled_translations("lang")`, gettext `.po` files compiled into the exe – no gettext runtime, no extra files). Switching: `slint::select_bundled_translation(lang)` re-evaluates all `@tr` **live**.
- **Rust:** the same `.po` files, read at build time by a small `build.rs` step into a static table; `tr!("…")` / `tr_n!(n, "…", "…")` macro in a new tiny crate `explorer-i18n` (lookup by source string + optional context, current language in an atomic). No new runtime dependency. Rust-built texts (status line, models) are re-sent on a language switch (`sync_window` + model refresh).
- **Extraction/check:** `slint-tr-extractor` for `.slint`, an own scanner for `tr!` in Rust → one `.pot`; CI test: every `.po` complete (no missing/fuzzy entry), placeholders `{0}`/`{n}` identical, no unused entries.
- **Formatting:** `core::locale` (number grouping/decimal separator, date order, size units) chosen by language; plural rules per language from the `.po` header.
- **Fonts:** no new font. Latin/Cyrillic/Greek are covered by Segoe UI; CJK would need Windows' own fonts (Yu Gothic/Microsoft YaHei, fallback cost at the first frame → measure in the first-start gate, gotcha from M6.1b).
- **Tests:** E2E pin German (`EXPLORER2_LANG=de`), so the existing 270 checks stay valid; new E2E per language: every screen of the gallery in every language + an **overflow check** (UI dump: no text box wider than its parent / clipped) + pseudo-localisation (`EXPLORER2_LANG=qps`: every text +40 % length with accents) to find layout breaks before real translations exist.

## 3. Stages
| Stage | Content | Done when |
|---|---|---|
| **L0 Basics** | crate `explorer-i18n` (`tr!`/`tr_n!`, own `.po` reader + build-time table, plural rules, catalog check, `extract` tool), setting `language` (“” = Windows UI language via `GetUserPreferredUILanguages`, Linux `LANGUAGE`/`LC_ALL`/`LC_MESSAGES`/`LANG`), override `EXPLORER2_LANG`, Slint bundled translations wired, tests pin `de` | app unchanged, tests green |
| **L1 Slint texts** | all `.slint` texts to `@tr` with English source, `de.po` filled with today's German texts | German UI pixel-identical, English visible with `EXPLORER2_LANG=en` |
| **L2 Rust texts** | all user-visible Rust texts to `tr!`/`tr_n!` (commands, status, dialogs, errors, Inspector, search, recovery, setup), formatting via `core::locale`, `de.po` complete | same |
| **L3 English review** | English source texts reviewed as a product language (Windows Explorer terms), gallery in English (test targets translated via the catalogs) | English gallery complete (CI artifact), German unchanged, E2E (German) green |
| **L4 Language setting** | Settings → “Sprache/Language” (list in each language's own name), live switch, palette command | E2E: switch live, restart keeps it |
| **L5 Layout robustness** | pseudo-locale (`qps`, generated at build time) + overflow check over all screens, fixes (shorten, wrap, dynamic widths), gallery per language as a CI artifact | no overflow in any language |
| **L6 More languages** | one language per step, each with glossary + gallery | per language: complete + green |

## 4. Decisions (Boss, 2026-11-10 → D-029)
1. **Languages:** the most widely used – German, English, Spanish, French, Russian, Ukrainian, Polish, Italian, Chinese (Simplified), Japanese; one after another (L6), each with glossary + gallery.
2. **Source language:** **English** (“nur Englisch”); German becomes `de.po` and must be complete before L1/L2 land, so the app stays German for German users.
3. **Language switch:** **live**.
4. **Translation quality:** I translate with a glossary based on the Windows Explorer terms of each language; German and English I review myself, for the other languages a native speaker should look over the gallery.

## 5. Risks
- **Exe size/start:** strings ×N languages are small (~50 KB per language); `@tr` costs a lookup per text – measure start and first-start gates.
- **Tests:** if German stops being the source, any forgotten test that matches a text breaks → `EXPLORER2_LANG=de` everywhere in CI/E2E first (L0), then convert.
- **Texts in traces:** traces stay English/neutral, never translated (tests rely on them).
- **Placeholders:** `{0}` order differs between languages → Slint supports positional `{0}`/`{1}`; Rust `tr!` the same.
