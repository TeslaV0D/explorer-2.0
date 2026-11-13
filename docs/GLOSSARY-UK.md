# Glossary: Ukrainian (uk)

Localisation stage L6 (D-029, `docs/PLAN-I18N.md`). Catalog: `lang/uk/LC_MESSAGES/explorer2.po` (complete, offered as “Українська”).
Variant: **Ukrainian (uk-UA)**, as Windows Explorer shows it; formal “ви” in instructions (“видаліть …”, “введіть > …”); apostrophe `'` like Windows (“Ім'я”).
**Native-speaker review still open**: please look over the CI artifact `ui-gallery-linux-l10n` (folder `uk`).

## Core terms (Windows Explorer uk-UA)
| English | Українська | Note |
|---|---|---|
| File Explorer | Провідник | our app keeps its name “Explorer 2.0” |
| Recycle Bin | Кошик | |
| Empty Recycle Bin | Очистити кошик | |
| This PC | Цей ПК | |
| Quick access | Швидкий доступ | |
| Home (user folder) | Особиста папка | not “Головна” (Windows 11 start page) |
| Desktop / Documents / Downloads / Pictures / Music / Videos | Робочий стіл / Документи / Завантаження / Зображення / Музика / Відео | |
| Libraries | Бібліотеки | |
| Local Disk / USB Drive / DVD Drive / Network Drive | Локальний диск / USB-накопичувач / DVD-дисковод / Мережний диск | |
| File folder | Папка файлів | type name |
| {} File | Файл {} | “Файл TXT” |
| Shortcut | Ярлик | |
| Name / Date modified / Type / Size | Ім'я / Дата змінення / Тип / Розмір | column headers |
| Date deleted / Original location | Дата видалення / Початкове розташування | Recycle Bin columns |
| Cut / Copy / Paste / Rename / Delete | Вирізати / Копіювати / Вставити / Перейменувати / Видалити | |
| Delete permanently | Видалити остаточно | |
| Properties | Властивості | |
| New folder / New tab | Нова папка / Нова вкладка | |
| Undo / Redo | Скасувати / Повторити | “Скасувати” is also Cancel, as in Windows |
| Refresh | Оновити | |
| Select all | Виділити все | |
| Show hidden items | Показувати приховані елементи | |
| File name extensions | Розширення імен файлів | |
| Details / Large icons / Medium icons / Extra large icons | Таблиця / Великі значки / Звичайні значки / Величезні значки | Explorer's view names |
| Sort / Ascending / Descending | Сортування / За зростанням / За спаданням | |
| Search | Пошук | “Search X” → “Пошук: X” |
| Settings | Параметри | |
| Folder options | Параметри папок | |
| Address bar | Адресний рядок | |
| Command palette | Палітра команд | |
| Inspector | Інспектор | “Toggle details pane” → “Показати або сховати область відомостей” |
| Install | Інсталювати | Windows uk uses “інсталювати” |
| Color scheme / Light / Dark / System default | Колірна схема / Світла / Темна / Як у системі | |
| Replace / Merge / Skip / Keep both / Cancel | Замінити / Об'єднати / Пропустити / Зберегти обидва / Скасувати | conflict dialog |
| OK / Close | OK / Закрити | |
| Restore / Reset | Відновити / Скинути | |
| Windows integration | Інтеграція з Windows | |
| item | елемент / елементи / елементів | three plural forms (1, 21 / 2–4, 22 / 0, 5–20, 11–14) |

## Typography
- Quotes «…» without spaces; dash “–” between parts of a sentence, “—” in copy names (`Ім'я — копія (2)`).
- Ellipsis without a space: `Завантаження…`.

## Keys
Key names stay Latin as on Ukrainian keyboards: **Ctrl**, **Shift**, **Alt**, **Del**, **Enter**; arrows **Вліво/Вправо/Вгору**.

Access keys in the conflict dialog (`core::mnemonic`, Cyrillic): **З**амінити / **О**б'єднати, **П**ропустити, Зберегти о**б**идва, **в**сіх (“Застосувати до &всіх решти конфліктів”). Esc = Скасувати.

Recovery tool prompt: `[т/Н]`, accepts “т”, “так” (and always y/yes/j/ja).

## Numbers and units
`1 234,5` (no-break space for thousands, comma for decimals); size units **Б, КБ, МБ, ГБ, ТБ** (“3,1 КБ (3 170 байт)”), milliseconds “мс”. Dates are the Windows short format `dd.MM.yyyy H:mm` (“15.01.2026 13:05”).

## Deliberately not translated
- The Start menu entry “Explorer 2.0 – Windows-Standard wiederherstellen” and the folder verb texts in the registry stay German (R-001; TODO “R-001-safe relabel”).
- Technical names: `EXPLORER2_INDEX`, `SLINT_BACKEND`, `fs.inotify.max_user_watches`, `shell:`, `crash` folder; Latin key names.
