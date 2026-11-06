# Nusantara Report Designer

A desktop designer for `.mrt` report templates. Lay out bands, tables, cross-tabs, and fields, preview with live data, and export to PDF, HTML, or text.

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-2024_edition-orange.svg)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)

![Nusantara Report Designer](docs/screenshot.png)

## Features

**Design**
- Page sizes A4, Letter, Legal, or custom. Portrait and landscape orientation. Each page can have its own size.
- Multiple columns per page (1 to 6), with a set gap between them. Body bands flow down one column, then into the next.
- Bands: Report Title, Page Header, Header, Data, Footer, Page Footer, Report Summary, and Group Header and Footer.
- Components: text, barcode, image, shape, line, checkbox, table, cross-tab, and subreport.
- Grid, snap to grid, rulers, guides (drag from a ruler, snap moved components to them), zoom, and fit page.
- Text formatting: font, size, bold, italic, underline, alignment, text and fill colour, and per-side borders.
- Watermark: a rotated, semi-transparent text drawn behind every sheet of a page.
- Undo and redo, copy, paste, duplicate, and multiple pages.
- Multi-select with Ctrl+click or a rubber-band drag; move the selection together. The Arrange menu aligns, distributes, and changes z-order.
- Outline view of bands and components.

**Barcodes**
- Code 128 (set B) and Code 39, drawn as real bars in the preview and in the PDF.

**Tables**
- Native table with a header row, a body row, and an optional footer row. Columns have their own widths, and body and footer cells can use expressions.
- With a data source, the body row prints once for every data row. The header repeats on each new sheet.

**Cross-tabs**
- A pivot table built from a data source: distinct values of one column as rows, another column as columns, and a value column aggregated as Sum, Count, or Average. Row totals, column totals, and a grand total are added.

**Subreports**
- Print another `.mrt` file inside a report, with its own data. Relative paths are taken from the folder of the report that contains it.

**Rich text (HTML tags)**
- Text components can enable *Allow HTML tags*.
- Inline tags: `b`, `strong`, `i`, `em`, `u`, `s`, `strike`, `del`, `mark` (highlight), `font` (color, size, face), `span` (style), `sup`, `sub`, `br`, and entities such as `&nbsp;`.
- Blocks: `p`, `div`, and headings `h1` to `h6`, with alignment set by `align` or `text-align` (left, centre, right). Lists with `ul` and `ol`, each `li` with a bullet or number.
- The HTML editor in the Properties panel has toolbar buttons for bold, italic, underline, strikethrough, highlight, colour, size, headings, lists, paragraph alignment, and line break, plus a source box and a live preview that matches the page.

**Conditional formatting**
- Per component, one or more rules: `>`, `>=`, `<`, `<=`, `=`, `<>`, or `contains`. The first matching rule sets the fill and text colour. Numbers compare as numbers, with either `.` or `,` as the decimal separator.

**Data and expressions**
- Load JSON data through the *Data* tab, or let the app pick up data files referenced by a `.mrt` (looked up in a local `assets/json` folder).
- Expressions: `{Source.Column}`, `{Variable}`, `{Sum(Band, Source.Column)}`, `{Convert.ToDateTime(...).ToString("dd MMMM yyyy")}`, and arithmetic on numbers.
- Running variables: a variable can be a running sum or running count over a data column, updated row by row as the report prints.
- Relations and master-detail: link two data sources by a key column. A detail band prints once for each child row under its parent row, and detail bands can nest.
- Group header bands group rows by a column. Groups can be kept together on one sheet when they fit.
- Expressions that are not supported are shown as written.

**Preview**
- All pages are shown with pagination. Data bands repeat per row and continue on the next page when needed.
- Page header and footer repeat on every page, with page number and total page count filled in.
- Export to PDF (vector output; Arial is embedded when found in the Windows font folder), HTML (absolute positioning per component), or plain text.

**Updates**
- The app checks GitHub for a newer release when it starts and every six hours while it runs. When one is found, a banner offers a link to the release page. *Help > Check for updates* checks on demand.
- The check only sends a plain request to the public GitHub releases API. It sends no report data.

**File formats**
- Reads `.mrt` files in JSON and XML formats.
- Units: centimetres, inches, millimetres, and hundredths of an inch. Everything is converted to millimetres internally.
- Saves `.mrt` in JSON format. Settings that Stimulsoft Designer does not define (for example conditional rules, cross-tab and table settings, subreport paths, relations, and guides) are stored under extra keys that other tools may ignore.

## Limitations

- Charts, RTF rich text, Excel and Word export, direct printing, report encryption, the Report Checker, and multi-language UI are not supported.
- Tables and cross-tabs are placed on the page, not inside a band. A table has one header row, one body row, and one footer row. Cells cannot be merged.
- A subreport is placed on the page, not inside a band. Its page header and footer are not applied inside it, and it is not split across pages of the parent report.
- Tables that came from a Stimulsoft `.mrt` file are kept as their individual text cells. They print as they did before, but they are not edited as one table.
- Guides and the grid are design aids. They do not appear in the preview or the PDF.
- Only Arial is embedded in the PDF. Other font families are substituted.
- Saving overwrites the file that was opened. Use *Save as* when working on files in a shared or production folder.

## Installation

### Requirements

- Windows 10 or 11.
- Rust, stable toolchain, with support for the 2024 edition.
- The GNU toolchain (`x86_64-pc-windows-gnu`) with MinGW-w64 tools. The project uses `gcc`, `ar`, and `windres`. Install them with [MSYS2](https://www.msys2.org/) (package `mingw-w64-x86_64-toolchain`), or adjust the paths in [`.cargo/config.toml`](.cargo/config.toml).

### Build

Run these commands from the project root (the folder that contains `Cargo.toml`):

```powershell
cargo build              # debug build, keeps the console for logs
cargo build --release    # release build, no console, icon and version info embedded
```

Output:
- Debug: `target\debug\nusantara_report_designer.exe`
- Release: `target\release\nusantara_report_designer.exe`

The build number is stamped automatically and increases with each compile. It is stored in `build_number.txt`. The build time is also stored in the program, and the update check uses it to decide whether a release is newer.

## Usage

```powershell
cargo run
```

Command-line options:

| Option | Description |
|---|---|
| `--open <file.mrt>` | Open a template on start |
| `--preview` | Start in Preview mode |
| `--about` | Open the About dialog on start |
| `--splash` | Show the splash screen even in screenshot mode |
| `--screenshot <file.png>` | Save a screenshot after about 120 frames and exit |

The splash screen appears for about two seconds on a normal start. Click it to close early.

### Keyboard shortcuts

| Shortcut | Action |
|---|---|
| Ctrl+N | New report |
| Ctrl+O | Open file |
| Ctrl+S | Save |
| Ctrl+Z / Ctrl+Y | Undo / redo |
| Ctrl+C / Ctrl+V | Copy / paste component |
| Ctrl+D | Duplicate component |
| Ctrl+click | Add or remove a component from the selection |
| Arrow keys | Move the selection (Shift for 5 mm steps) |
| Delete | Delete selected components or band |

## Development

### Tests

```powershell
cargo test
```

Some tests read sample files from disk. They run only when these environment variables are set:

```powershell
# Summary of every .mrt file in one folder
$env:NUSA_MRT_DIR = "C:\path\to\templates"
cargo test summarise_sample_folder -- --nocapture

# Summary of the data sources in one JSON file
$env:NUSA_DATA_FILE = "C:\path\to\data.json"
cargo test summarise_data_file -- --nocapture

# Summary of the pages of one .mrt file, with its data
$env:NUSA_PREVIEW_FILE = "C:\path\to\report.mrt"
cargo test summarise_preview_sheets -- --nocapture

# Live check against GitHub (needs internet access)
$env:NUSA_LIVE_UPDATE_TEST = "1"
cargo test latest_release_can_be_read -- --nocapture
```

Some tests also write PDF files to the temp folder (`nusa_sample_report.pdf`, and `nusa_demo.pdf` when `NUSA_DEMO_PDF` is set). Open them to check the layout.

### Project layout

```
.
├── Cargo.toml
├── build.rs              # build number, build time, Windows icon and version resource
├── build_number.txt      # daily build counter
├── assets/               # application icon and logo (WebP)
├── docs/                 # documentation assets
├── .cargo/config.toml    # linker and PATH for the GNU toolchain
└── src/
    ├── main.rs           # command-line options and window setup
    ├── update.rs         # release check against GitHub
    ├── model/            # report, bands, components, .mrt parser and writer,
    │                     # expressions, rich text, barcodes, cross-tabs,
    │                     # subreports, dictionary and relations
    └── ui/               # ribbon, side panel (Pages / Data / Outline),
                          # canvas, properties, HTML editor, preview,
                          # PDF export, dialogs, About, splash
```

## Contributing

Contributions are welcome. Before opening a pull request:

1. Fork the repository and create a branch for your change.
2. Run `cargo build` and `cargo test`. Both must pass without warnings.
3. Keep changes focused. One feature or fix per pull request.
4. Describe what changed and how you tested it.

Bug reports and feature requests are welcome through the issue tracker. For bugs, include the steps to reproduce, the build number shown in the title bar, and, if possible, a small `.mrt` file that shows the problem.

## License

Nusantara Report Designer is free software: you can redistribute it and/or modify it under the terms of the **GNU General Public License version 3** (or, at your option, any later version). It is distributed in the hope that it will be useful, but **without any warranty**. See [LICENSE](LICENSE) for the full text.

Copyright © 2026.

## Acknowledgements

- [egui](https://github.com/emilk/egui) and [eframe](https://github.com/emilk/egui/tree/master/crates/eframe), the immediate-mode GUI and window framework.
- [serde](https://serde.rs/) and [serde_json](https://github.com/serde-rs/json), for reading and writing report files.
- [image](https://github.com/image-rs/image), for decoding the icon, logo, and report pictures.
- [printpdf](https://github.com/fschutt/printpdf), for PDF export.
- [barcoders](https://github.com/buntine/barcoders), for Code 128 and Code 39 bars.
- [ttf-parser](https://github.com/RazrFalcon/ttf-parser), for measuring text in the PDF.
- [ureq](https://github.com/algesten/ureq), for the release check.
- [rfd](https://github.com/PolyMeilex/rfd), for native file dialogs.
- [uuid](https://github.com/uuid-rs/uuid) and [chrono](https://github.com/chronotope/chrono).
