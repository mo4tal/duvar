# Duvar Localization Guide (`locales/`)

Duvar defaults to **Turkish (`tr`)** and ships with **10 built-in languages**:

| Code | Native Name | English Name | Direction | File |
| :---: | :--- | :--- | :---: | :--- |
| `tr` | **Türkçe** *(Default)* | Turkish | LTR | [`tr.json`](tr.json) |
| `en` | **English** | English | LTR | [`en.json`](en.json) |
| `es` | **Español** | Spanish | LTR | [`es.json`](es.json) |
| `de` | **Deutsch** | German | LTR | [`de.json`](de.json) |
| `fr` | **Français** | French | LTR | [`fr.json`](fr.json) |
| `pt` | **Português (Brasil)** | Portuguese (Brazil) | LTR | [`pt.json`](pt.json) |
| `ru` | **Русский** | Russian | LTR | [`ru.json`](ru.json) |
| `ja` | **日本語** | Japanese | LTR | [`ja.json`](ja.json) |
| `zh` | **简体中文** | Chinese (Simplified) | LTR | [`zh.json`](zh.json) |
| `ar` | **العربية** | Arabic | RTL | [`ar.json`](ar.json) |

## How Localization Works

- **Single source of truth:** Both the Rust GTK 4 / Libadwaita layer (`src/i18n.rs`, compiled via `include_str!`) and the GJS GNOME Shell / Colors layer (`extension/i18n.js`, bundled into the extension ZIP) read the exact same `locales/*.json` files.
- **Active language:** Stored in GSettings key `org.gnome.shell.extensions.duvar` → `ui-language` (default `'tr'`), selectable under **Settings → Appearance & Privacy → Language**. Can also be overridden for testing via `DUVAR_LANG=en`.
- **Where every string appears:** See [`CONTEXT.md`](CONTEXT.md) for a complete key-by-key map of screen, file, widget, and placeholder variables.

## Rules for Translators & AI Assistants

1. **Preserve every `{placeholder}` verbatim** (e.g., `{count}`, `{width}`, `{height}`, `{version}`, `{title}`). Never translate placeholder names.
2. **Keep button labels concise** so they fit GNOME / Adwaita compact cards and 480 px narrow layouts without ellipsizing.
3. **Keep key structure identical to `tr.json`:** Do not add or remove keys in only one language file.
4. **Verify parity before committing:**
   ```sh
   TMPXDG=$(mktemp -d) && XDG_DATA_HOME="$TMPXDG" cargo test --locked --offline --lib i18n && rm -rf "$TMPXDG"
   ```
   The unit test verifies JSON syntax, 100% key parity across all 10 locales, non-empty values, and exact `{placeholder}` matching.
