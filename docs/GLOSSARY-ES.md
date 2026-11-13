# Glossary: Spanish (es)

Localisation stage L6 (D-029, `docs/PLAN-I18N.md`). Catalog: `lang/es/LC_MESSAGES/explorer2.po` (complete, offered as “Español”).
Variant: **Spanish (Spain), es-ES**, as Windows Explorer shows it; informal “tú” in instructions (“Elimina …”, “escribe > …”) like Windows 11.
**Native-speaker review still open**: please look over the CI artifact `ui-gallery-linux-l10n` (folder `es`).

## Core terms (Windows Explorer es-ES)
| English | Español | Note |
|---|---|---|
| File Explorer | Explorador de archivos | our app keeps its name “Explorer 2.0” |
| Recycle Bin | Papelera de reciclaje | |
| Empty Recycle Bin | Vaciar Papelera de reciclaje | |
| This PC | Este equipo | |
| Quick access | Acceso rápido | |
| Home (user folder) | Carpeta personal | not “Inicio” (that is Windows 11's start page) |
| Desktop / Documents / Downloads / Pictures / Music / Videos | Escritorio / Documentos / Descargas / Imágenes / Música / Vídeos | “Vídeos” with accent (es-ES) |
| Libraries | Bibliotecas | |
| Local Disk / USB Drive / DVD Drive / Network Drive | Disco local / Unidad USB / Unidad de DVD / Unidad de red | |
| File folder | Carpeta de archivos | type name |
| {} File | Archivo {} | “Archivo TXT” |
| Shortcut | Acceso directo | |
| Name / Date modified / Type / Size | Nombre / Fecha de modificación / Tipo / Tamaño | column headers |
| Date deleted / Original location | Fecha de eliminación / Ubicación original | Recycle Bin columns |
| Cut / Copy / Paste / Rename / Delete | Cortar / Copiar / Pegar / Cambiar nombre / Eliminar | |
| Delete permanently | Eliminar permanentemente | |
| Properties | Propiedades | |
| New folder / New tab | Nueva carpeta / Nueva pestaña | |
| Undo / Redo | Deshacer / Rehacer | |
| Refresh | Actualizar | |
| Select all | Seleccionar todo | |
| Show hidden items | Mostrar elementos ocultos | |
| File name extensions | Extensiones de nombre de archivo | |
| Details / Large icons / Medium icons / Extra large icons | Detalles / Iconos grandes / Iconos medianos / Iconos muy grandes | |
| Sort / Ascending / Descending | Ordenar / Ascendente / Descendente | |
| Search | Buscar | “Search X” → “Buscar en X” |
| Settings | Configuración | |
| Folder options | Opciones de carpeta | |
| Address bar | Barra de direcciones | |
| Command palette | Paleta de comandos | |
| Inspector | Inspector | own feature; “Toggle details pane” → “Mostrar u ocultar el panel de detalles” |
| Color scheme / Light / Dark / System default | Esquema de colores / Claro / Oscuro / Predeterminado del sistema | |
| Replace / Merge / Skip / Keep both / Cancel | Reemplazar / Combinar / Omitir / Conservar ambos / Cancelar | conflict dialog |
| OK / Close | Aceptar / Cerrar | |
| Restore | Restaurar | |
| Windows integration | Integración con Windows | |
| Reset | Restablecer | |
| item / items | elemento / elementos | “3 elementos seleccionados” |

## Keys
Key names as on a Spanish keyboard / in Windows es-ES menus: **Ctrl**, **Mayús** (Shift), **Alt**, **Supr** (Del), **Entrar** (Enter), arrows **Izquierda/Derecha/Arriba**. The keys themselves never change (Ctrl+A stays Ctrl+A).

Access keys in the conflict dialog (`core::mnemonic`, the `&` in the catalog): **R**eemplazar / Co**m**binar, **O**mitir, **C**onservar ambos, **t**odos (“Hacer esto con &todos los conflictos restantes”). Esc = Cancelar.

Recovery tool prompt: `[s/N]`, accepts “s”, “sí” (and always y/yes/j/ja).

## Numbers
`1.234,5` (point for thousands, comma for decimals), sizes “3,1 KB (3.170 bytes)”, `explorer_i18n::separators("es")`. Dates are the Windows short format `dd/MM/yyyy H:mm` (“15/01/2026 13:05”).

## Deliberately not translated
- The Start menu entry “Explorer 2.0 – Windows-Standard wiederherstellen” and the folder verb texts written to the registry stay German (R-001 journal compares them; TODO “R-001-safe relabel”). Texts that name this entry quote it unchanged.
- Technical names: `EXPLORER2_INDEX`, `SLINT_BACKEND`, `fs.inotify.max_user_watches`, `shell:`, `crash` folder.
