# Dune Terminal Codex

An offline encyclopedia of the Dune universe for your terminal. Index your
personal PDF collection of the saga, search it full-text (FTS5 with bm25
relevance ranking), browse a codex of characters / houses / planets /
glossary imported from a wiki, and receive wisdom from the Oracle.

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
dune index         # scan library paths and index every PDF (full-text)
dune import-codex  # fetch characters, houses, planets and glossary
```

Edit `[library] paths` in the config file to point at your PDFs, and
`[reader] command` to change the PDF viewer.

## Command reference

| Command                  | Description                                        |
|--------------------------|----------------------------------------------------|
| `dune` / `dune tui`      | Launch the interactive TUI                         |
| `dune search <query>`    | Full-text search across indexed books              |
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
| `dune open <path\|id> [--page N]` | Open a PDF at a page (by path or book id) |

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
