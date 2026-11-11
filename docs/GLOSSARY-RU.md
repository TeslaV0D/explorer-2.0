# Glossary: Russian (ru)

Localisation stage L6 (D-029, `docs/PLAN-I18N.md`). Catalog: `lang/ru/LC_MESSAGES/explorer2.po` (complete, offered as “Русский”).
Variant: **Russian (ru-RU)**, as Windows Explorer shows it; formal “вы” in instructions (“удалите …”, “введите > …”), **е instead of ё** like Windows (“Темная”, “еще”).
**Native-speaker review still open**: please look over the CI artifact `ui-gallery-linux-l10n` (folder `ru`).

## Core terms (Windows Explorer ru-RU)
| English | Русский | Note |
|---|---|---|
| File Explorer | Проводник | our app keeps its name “Explorer 2.0” |
| Recycle Bin | Корзина | |
| Empty Recycle Bin | Очистить корзину | |
| This PC | Этот компьютер | |
| Quick access | Быстрый доступ | |
| Home (user folder) | Личная папка | not “Главная” (Windows 11 start page) |
| Desktop / Documents / Downloads / Pictures / Music / Videos | Рабочий стол / Документы / Загрузки / Изображения / Музыка / Видео | |
| Libraries | Библиотеки | |
| Local Disk / USB Drive / DVD Drive / Network Drive | Локальный диск / USB-накопитель / DVD-дисковод / Сетевой диск | |
| File folder | Папка с файлами | type name |
| {} File | Файл {} | “Файл TXT” |
| Shortcut | Ярлык | |
| Name / Date modified / Type / Size | Имя / Дата изменения / Тип / Размер | column headers |
| Date deleted / Original location | Дата удаления / Исходное расположение | Recycle Bin columns |
| Cut / Copy / Paste / Rename / Delete | Вырезать / Копировать / Вставить / Переименовать / Удалить | |
| Delete permanently | Удалить безвозвратно | |
| Properties | Свойства | |
| New folder / New tab | Новая папка / Новая вкладка | |
| Undo / Redo | Отменить / Вернуть | |
| Refresh | Обновить | |
| Select all | Выделить все | |
| Show hidden items | Показывать скрытые элементы | |
| File name extensions | Расширения имен файлов | |
| Details / Large icons / Medium icons / Extra large icons | Таблица / Крупные значки / Обычные значки / Огромные значки | Explorer's view names (“Details” = “Таблица”) |
| Sort / Ascending / Descending | Сортировка / По возрастанию / По убыванию | |
| Search | Поиск | “Search X” → “Поиск: X” (like Explorer's search box) |
| Settings | Параметры | |
| Folder options | Параметры папок | |
| Address bar | Адресная строка | |
| Command palette | Палитра команд | |
| Inspector | Инспектор | “Toggle details pane” → “Показать или скрыть область сведений” |
| Color scheme / Light / Dark / System default | Цветовая схема / Светлая / Темная / Как в системе | |
| Replace / Merge / Skip / Keep both / Cancel | Заменить / Объединить / Пропустить / Сохранить оба / Отмена | conflict dialog |
| OK / Close | ОК / Закрыть | |
| Restore / Reset | Восстановить / Сбросить | |
| Windows integration | Интеграция с Windows | |
| item | элемент / элемента / элементов | three plural forms (1, 21 / 2–4, 22 / 0, 5–20, 11–14) |

## Typography
- Quotes «ёлочки» without spaces: `«notizen.txt»`; dash “–” between parts of a sentence, “—” in copy names like Windows (`Имя — копия (2)`).
- Ellipsis without a space: `Загрузка…`.

## Keys
Key names stay Latin as on Russian keyboards and in Windows ru-RU: **Ctrl**, **Shift**, **Alt**, **Del**, **Enter**; arrows **Влево/Вправо/Вверх**. The keys themselves never change.

Access keys in the conflict dialog (`core::mnemonic`, Cyrillic like Windows: they work with the Russian keyboard layout): **З**аменить / **О**бъединить, **П**ропустить, **С**охранить оба, **в**сем (“Применить ко &всем оставшимся конфликтам”). Esc = Отмена.

Recovery tool prompt: `[д/Н]`, accepts “д”, “да” (and always y/yes/j/ja).

## Numbers and units
`1 234,5` (no-break space for thousands, comma for decimals); size units **Б, КБ, МБ, ГБ, ТБ** like Russian Windows (“3,1 КБ (3 170 байт)”).

## Deliberately not translated
- The Start menu entry “Explorer 2.0 – Windows-Standard wiederherstellen” and the folder verb texts in the registry stay German (R-001; TODO “R-001-safe relabel”).
- Technical names: `EXPLORER2_INDEX`, `SLINT_BACKEND`, `fs.inotify.max_user_watches`, `shell:`, `crash` folder; Latin key names.
