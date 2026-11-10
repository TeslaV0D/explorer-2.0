# Plan: localisation (i18n)

As of 2026-11-10 · Boss: “mach die Lokalisation weiter” · supersedes D-007 (UI German hard-coded) once decided.
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
| **L0 Basics** | crate `explorer-i18n`, `core::locale`, setting `language` (“” = Windows language via `GetUserDefaultUILanguage`, Linux `LANG`), override `EXPLORER2_LANG`, build step, CI completeness test, pseudo-locale | app unchanged, tests green |
| **L1 Slint texts** | all `.slint` texts to `@tr`, `.pot` extracted | app looks the same in the source language |
| **L2 Rust texts** | all user-visible Rust texts to `tr!`/`tr_n!` (commands, status, dialogs, errors, Inspector, search, recovery, setup), formatting via `core::locale` | same |
| **L3 First translation** | glossary (Windows Explorer terms per language from Microsoft's terminology), complete `.po` | E2E in the new language green |
| **L4 Language setting** | Settings → “Sprache/Language” (list in each language's own name), live switch, palette command | E2E: switch live, restart keeps it |
| **L5 Layout robustness** | pseudo-locale + overflow check over all screens, fixes (shorten, wrap, dynamic widths), gallery per language as a CI artifact | no overflow in any language |
| **L6 More languages** | one language per step, each with glossary + gallery | per language: complete + green |

## 4. Decisions for Boss
1. **Which languages, in which order?** Recommendation: English first (biggest reach, also the README language), then e.g. French/Spanish/Russian/Polish – depends on who should use it.
2. **Source language in the code:** *Recommendation: English source texts, German becomes a translation (`de.po`).* That's the standard (every further language is translated from English, matches D-004 “code in English”, translation tools expect it). Cost: every text is touched once and tests pin German. Alternative: keep German as the source and add `en.po` – less churn now, but every further translator needs German.
3. **Language switch:** *Recommendation: live* (Slint supports it; Rust texts are re-sent). Alternative: “takes effect after a restart” (simpler, less test effort).
4. **Translation quality:** I translate with a glossary based on the Windows Explorer terms of each language; German and English I can review myself, for other languages a native speaker should look over the gallery (screenshots per language).

## 5. Risks
- **Exe size/start:** strings ×N languages are small (~50 KB per language); `@tr` costs a lookup per text – measure start and first-start gates.
- **Tests:** if German stops being the source, any forgotten test that matches a text breaks → `EXPLORER2_LANG=de` everywhere in CI/E2E first (L0), then convert.
- **Texts in traces:** traces stay English/neutral, never translated (tests rely on them).
- **Placeholders:** `{0}` order differs between languages → Slint supports positional `{0}`/`{1}`; Rust `tr!` the same.
