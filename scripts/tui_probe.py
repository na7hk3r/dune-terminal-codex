#!/usr/bin/env python3
"""Optional manual smoke probe for the dune TUI (NOT part of cargo test).

Drives the interactive TUI inside a pseudo-terminal, feeds keystrokes, and
prints the rendered screen as plain text so the fuzzy search can be verified
end-to-end without a real terminal.

Usage:
    cargo build
    DUNE_DB_PATH=/tmp/probe.db ASSERT=Atreides KEYS='3,atelides' \
        python3 scripts/tui_probe.py -- ./target/debug/dune tui

Env:
  KEYS    comma-separated keystrokes between captures. Supports escapes like
          \r \x03 \t and literal chars; e.g. KEYS='3,s,p,i,c,e,\r'
  ROWS    terminal rows (default 35)
  COLS    terminal cols (default 120)
  ASSERT  optional substring that must appear in the final rendered screen;
          exit code 1 and the final frame are printed when it is missing.
          E.g. ASSERT=Atreides after KEYS='3,atelides' proves the TUI search
          surfaces the typo'd query through the trigram fuzzy index.
  PROBE_CWD  working directory for the spawned command (default: cwd)

Requires: pyte (pip install pyte). The venv used for this project is
~/.venvs/tui: `python3 -m venv ~/.venvs/tui --system-site-packages &&
~/.venvs/tui/bin/pip install pyte`.

The probe deliberately stays OUT of `cargo test`: it needs a TTY, a Python
venv with pyte, and a seeded database.
"""
import fcntl
import os
import pty
import select
import struct
import sys
import termios
import time

import pyte

ROWS = int(os.environ.get("ROWS", "35"))
COLS = int(os.environ.get("COLS", "120"))
ASSERT = os.environ.get("ASSERT", "")


def spawn(cmd, cwd):
    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(cwd)
        os.environ["TERM"] = "xterm-256color"
        os.execvp(cmd[0], cmd)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
    return pid, fd


def main():
    args = sys.argv[1:]
    if args and args[0] == "--":
        args = args[1:]
    cmd = args if args else ["./target/debug/dune", "tui"]
    cwd = os.environ.get("PROBE_CWD", os.getcwd())
    pid, fd = spawn(cmd, cwd)
    screen = pyte.Screen(COLS, ROWS)
    stream = pyte.ByteStream(screen)

    def pump(dur=0.6):
        end = time.time() + dur
        while time.time() < end:
            r, _, _ = select.select([fd], [], [], 0.1)
            if r:
                try:
                    data = os.read(fd, 65536)
                except OSError:
                    return False
                if not data:
                    return False
                stream.feed(data)
        return True

    pump(2.0)  # initial render

    for key in [k for k in os.environ.get("KEYS", "").split(",") if k]:
        os.write(fd, key.encode().decode("unicode_escape").encode())
        pump(0.8)

    frame = "\n".join(screen.display)
    print(frame)
    if ASSERT and ASSERT not in frame:
        print(f"\nASSERT FAILED: {ASSERT!r} not found in rendered screen",
              file=sys.stderr)
        sys.exit(1)

    try:
        os.write(fd, b"\x03")  # ctrl-c
        pump(0.5)
        os.write(fd, b"q")
        pump(0.3)
    except OSError:
        pass
    try:
        os.close(fd)
    except OSError:
        pass
    try:
        os.waitpid(pid, 0)
    except ChildProcessError:
        pass


if __name__ == "__main__":
    main()