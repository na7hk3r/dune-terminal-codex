# Dune Terminal Codex

An encyclopedia of the Dune universe that lives in your terminal.

I watched the movie, bought book six, and discovered I had no idea who anyone
was. I spent more time juggling wiki tabs than actually reading. So instead of
just keeping a browser tab open like a normal person, I built a SQLite
full-text index, a TUI, and an Oracle. Peak over-engineering. Now other fans
can profit from my complete lack of restraint.

![Dune Terminal Codex TUI](dune-screenshot.png)

## What it does

- Indexes your personal PDF collection of the saga for full-text search
  (FTS5 with bm25 relevance ranking).
- Populates a codex of characters, Great Houses, planets and glossary terms
  from a Dune wiki.
- Dispenses quotes and random wisdom via the Oracle.
- Works as an interactive TUI (`dune`) and as plain CLI commands.

Everything runs locally against your own files; the only step that touches the
network is the codex import.

Built with Rust, `ratatui` 0.30 and `rusqlite`.

## Requirements

- Rust stable (`rustup` recommended)
- `pdftotext` / `pdfinfo` (poppler-utils) for PDF indexing
- A PDF viewer for `dune open` (default: `okular`, configurable)

## Build

```sh
cargo build --release
# binary at target/release/dune
```

## First use

```sh
dune config        # create the default config (~/.config/dune/config.toml)
```

Edit `[library] paths` in `~/.config/dune/config.toml` to point at your PDFs
(it defaults to `~/Books/Dune`), and `[reader] command` to change the PDF
viewer. Then:

```sh
dune index         # scan library paths and index every PDF (full-text)
dune import-codex  # fetch characters, houses, planets and glossary
```

## Command reference

| Command                  | Description                                        |
|--------------------------|----------------------------------------------------|
| `dune` / `dune tui`      | Launch the interactive TUI                         |
| `dune search <query> [--book <name>]` | Full-text search, optionally filtered by book |
| `dune books`             | List indexed books                                 |
| `dune book <name>`       | Show details of one book                           |
| `dune character <name>`  | Codex entry for a character                        |
| `dune house <name>`      | Codex entry for a Great House                      |
| `dune planet <name>`     | Codex entry for a planet                           |
| `dune glossary [term]`   | Browse or look up glossary terms                   |
| `dune quote`             | Random quote                                       |
| `dune oracle`            | Random wisdom from the Oracle                      |
| `dune stats`             | Library statistics                                 |
| `dune index [--rebuild] [--verbose] [--file <pdf>...]` | Index/reindex PDFs |
| `dune import-codex`      | Import/refresh codex data (idempotent re-import)   |
| `dune config`            | Show configuration location and contents           |
| `dune open <path> [--page N]` | Open a PDF at a page                          |

## TUI keybindings

| Key            | Action                                            |
|----------------|---------------------------------------------------|
| `1`–`6` / Tab  | Switch tabs (Home, Library, Search, Codex, Oracle, Stats) |
| `j` / `k`      | Move down / up                                    |
| `PgUp` / `PgDn`| Scroll by page                                    |
| `g` / `G`      | Jump to start / end                               |
| `Enter`        | Open item (book, codex entry, next tab on Home)   |
| `c`            | Cycle Codex category                              |
| `r`            | New Oracle wisdom                                 |
| type           | Live search in the Search tab                     |
| `Esc` / `q`    | Back / close detail / quit                        |

In a Codex detail view: `j/k` scroll line by line, `PgUp/PgDn` by page,
`g/G` jump to top/bottom; the footer shows the current line position.

## Configuration

Config file: `~/.config/dune/config.toml` (created by `dune config`).

| Key                    | Default            | Purpose                          |
|------------------------|--------------------|----------------------------------|
| `[library] paths`      | `["~/Books/Dune"]` | Folders scanned by `dune index`  |
| `[reader] command`     | `"okular"`         | PDF viewer used by `dune open`   |
| `[reader] args`        | `[]`               | Extra arguments for the viewer   |
| `[codex] wiki_url`     | Dune wiki API      | Source used by `dune import-codex` |

The database lives at `~/.local/share/dune/dune.db`; delete it (or run
`dune index --rebuild`) to start over.

## License

MIT — see [LICENSE](LICENSE).
