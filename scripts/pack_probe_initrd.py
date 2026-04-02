#!/usr/bin/env python3
"""Pack gzip-compressed newc cpio initramfs (no cpio(1) required).

Modes:
  3 args: one ELF becomes /init (legacy).
  4 args: first ELF is /init, second is /probe (console stub + real probe).
"""
from __future__ import annotations

import gzip
import stat
import sys
from pathlib import Path


def _pad4(data: bytes) -> bytes:
    pad = (-len(data)) % 4
    return data + (b"\0" * pad)


def _newc_header(namesize: int, filesize: int, mode: int) -> bytes:
    """SVR4 newc cpio header: magic + thirteen 8-digit uppercase hex fields + newline."""
    parts = [
        "070701",
        f"{1:08X}",  # ino
        f"{mode:08X}",
        f"{0:08X}",  # uid
        f"{0:08X}",  # gid
        f"{1:08X}",  # nlink
        f"{0:08X}",  # mtime
        f"{filesize:08X}",
        f"{0:08X}",  # devmajor
        f"{0:08X}",  # devminor
        f"{0:08X}",  # rdevmajor
        f"{0:08X}",  # rdevminor
        f"{namesize:08X}",
        f"{0:08X}",  # check
    ]
    return "".join(parts).encode("ascii") + b"\n"


def _append_file(chunk: bytearray, path_in_cpio: str, elf_path: Path) -> None:
    body = elf_path.read_bytes()
    name = path_in_cpio.encode("ascii") + b"\0"
    namesize = len(name)
    fmode = stat.S_IFREG | 0o755
    hdr = _newc_header(namesize, len(body), fmode)
    chunk.extend(hdr + _pad4(name) + _pad4(body))


def pack_elf_as_init(elf_path: Path, out_gz: Path) -> None:
    chunk = bytearray()
    _append_file(chunk, "init", elf_path)
    trailer_name = b"TRAILER!!!\0"
    th = _newc_header(len(trailer_name), 0, 0)
    chunk.extend(th + _pad4(trailer_name))
    out_gz.write_bytes(gzip.compress(bytes(chunk), compresslevel=9))


def pack_init_and_probe(init_elf: Path, probe_elf: Path, out_gz: Path) -> None:
    chunk = bytearray()
    _append_file(chunk, "init", init_elf)
    _append_file(chunk, "probe", probe_elf)
    trailer_name = b"TRAILER!!!\0"
    th = _newc_header(len(trailer_name), 0, 0)
    chunk.extend(th + _pad4(trailer_name))
    out_gz.write_bytes(gzip.compress(bytes(chunk), compresslevel=9))


def main() -> int:
    if len(sys.argv) == 3:
        elf = Path(sys.argv[1])
        out = Path(sys.argv[2])
        if not elf.is_file():
            print(f"not a file: {elf}", file=sys.stderr)
            return 1
        pack_elf_as_init(elf, out)
        return 0
    if len(sys.argv) == 4:
        init_elf = Path(sys.argv[1])
        probe_elf = Path(sys.argv[2])
        out = Path(sys.argv[3])
        if not init_elf.is_file():
            print(f"not a file: {init_elf}", file=sys.stderr)
            return 1
        if not probe_elf.is_file():
            print(f"not a file: {probe_elf}", file=sys.stderr)
            return 1
        pack_init_and_probe(init_elf, probe_elf, out)
        return 0
    print(
        "usage: pack_probe_initrd.py <riscv64-static-elf> <out-initrd.cpio.gz>\n"
        "       pack_probe_initrd.py <init-elf> <probe-elf> <out-initrd.cpio.gz>",
        file=sys.stderr,
    )
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
