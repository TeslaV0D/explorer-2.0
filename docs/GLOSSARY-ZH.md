# Glossary: Chinese, Simplified (zh)

Localisation stage L6 (D-029, `docs/PLAN-I18N.md`). Catalog: `lang/zh/LC_MESSAGES/explorer2.po` (complete, offered as “中文（简体）”).
Variant: **Chinese (Simplified, China), zh-CN**, as Windows Explorer shows it. Traditional Chinese (`zh-Hant`, `zh-TW`, `zh-HK`, `zh-MO`) is **not** mapped to this catalog (`normalize` returns none); those users get the next preferred language.
**Native-speaker review still open**: please look over the CI artifact `ui-gallery-linux-l10n` (folder `zh`).

## Core terms (Windows Explorer zh-CN)
| English | 中文（简体） | Note |
|---|---|---|
| File Explorer | 文件资源管理器 | our app keeps its name “Explorer 2.0” |
| Recycle Bin | 回收站 | |
| Empty Recycle Bin | 清空回收站 | |
| This PC | 此电脑 | |
| Quick access | 快速访问 | |
| Home (user folder) | 个人文件夹 | not “主页” (that is the Windows 11 start page) |
| Desktop / Documents / Downloads / Pictures / Music / Videos | 桌面 / 文档 / 下载 / 图片 / 音乐 / 视频 | |
| Libraries | 库 | |
| Local Disk / USB Drive / DVD Drive / Network Drive | 本地磁盘 / USB 驱动器 / DVD 驱动器 / 网络驱动器 | |
| File folder / Folder | 文件夹 | both, as in the Chinese shell |
| {} File | {} 文件 | “TXT 文件” |
| Shortcut | 快捷方式 | “Shortcut or link” → “快捷方式或链接” |
| Name / Date modified / Type / Size | 名称 / 修改日期 / 类型 / 大小 | column headers as in Explorer |
| Date deleted / Original location | 删除日期 / 原始位置 | Recycle Bin columns |
| Cut / Copy / Paste / Rename / Delete | 剪切 / 复制 / 粘贴 / 重命名 / 删除 | |
| Delete permanently | 永久删除 | |
| Properties | 属性 | |
| New folder / New tab | 新建文件夹 / 新建标签页 | |
| Undo / Redo / Cancel | 撤消 / 重做 / 取消 | Cancel is not Undo |
| Refresh | 刷新 | |
| Select all | 全选 | |
| Show hidden items | 显示隐藏的项目 | |
| File name extensions | 文件扩展名 | “显示文件扩展名” |
| Details / Medium / Large / Extra large icons | 详细信息 / 中等图标 / 大图标 / 超大图标 | |
| Sort / Ascending / Descending | 排序 / 递增 / 递减 | |
| Search | 搜索 | “Search all drives” → “搜索所有驱动器” (short, the field is ~240 px) |
| Settings | 设置 | |
| Folder options | 文件夹选项 | |
| Address bar | 地址栏 | |
| Command palette | 命令面板 | |
| Inspector | 检查器 | |
| Color scheme / Light / Dark / System default | 颜色方案 / 浅色 / 深色 / 系统默认 | |
| Replace / Merge / Skip / Keep both / Cancel | 替换 / 合并 / 跳过 / 保留两者 / 取消 | conflict dialog |
| OK / Close | 确定 / 关闭 | |
| Restore / Reset | 还原 / 重置 | |
| Windows integration | Windows 集成 | |
| item | 个项目 | one form for 0, 1 and 2: “{n} 个项目” |

## Keys
Key names stay Latin, as on Chinese Windows: **Ctrl**, **Shift**, **Alt**, **Del**, **Enter**. The keys themselves never change (Ctrl+A stays Ctrl+A). Arrow tooltips use 左/右/上.

Access keys in the conflict dialog (`core::mnemonic`, the `&` in the catalog). The letter stays visible, Windows-style: 替换(&R) shows as 替换(R). Keys: **R**eplace, **M**erge (合并), **S**kip (跳过), **B**oth (保留两者), **A**ll (对剩余冲突全部执行). Esc = 取消.

Recovery tool prompt stays `[y/N]` (y/yes/j/ja are always accepted). The translated word for “yes” is “是”.

## Numbers and units
`1,234.5` (comma for thousands, point for decimals), `explorer_i18n::separators` default, which is what zh-CN uses. Size units stay **B, KB, MB, GB, TB**; the spelled unit is “字节” (“3.1 KB (3,170 字节)”). One plural form (`nplurals=1`).

## Deliberately not translated
- The Start menu entry “Explorer 2.0 – Windows-Standard wiederherstellen” and the folder verb texts written to the registry stay German (R-001 journal compares them; TODO “R-001-safe relabel”). Texts that name this entry quote it unchanged.
- Technical names: `EXPLORER2_INDEX`, `SLINT_BACKEND`, `fs.inotify.max_user_watches`, `shell:`, `crash` folder.
