#!/usr/bin/env python3
import gzip
import struct
import sys
from pathlib import Path


MAGIC = b"RSRD"
HEADER_SIZE = 64
FORMAT_CPIO_NEWC = 1
FORMAT_TAR_USTAR = 2
UIMAGE_MAGIC = 0x27051956


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: scripts/prepare_initrd.py <input-rootfs> <output-image>", file=sys.stderr)
        return 2

    src = Path(sys.argv[1])
    dst = Path(sys.argv[2])
    raw = src.read_bytes()
    payload = normalize_payload(raw)
    fmt = detect_format(payload)
    if fmt == 0:
        print(f"unsupported initrd format: {src}", file=sys.stderr)
        return 1

    dst.parent.mkdir(parents=True, exist_ok=True)
    header = bytearray(HEADER_SIZE)
    header[0:4] = MAGIC
    struct.pack_into("<I", header, 4, 1)
    struct.pack_into("<I", header, 8, fmt)
    struct.pack_into("<I", header, 12, HEADER_SIZE)
    struct.pack_into("<I", header, 16, len(payload))
    struct.pack_into("<I", header, 20, checksum(payload))
    dst.write_bytes(header + payload)
    print(f"initrd: {src} -> {dst} format={format_name(fmt)} size={len(payload)}")
    return 0


def normalize_payload(raw: bytes) -> bytes:
    if len(raw) >= 64 and struct.unpack_from(">I", raw, 0)[0] == UIMAGE_MAGIC:
        raw = raw[64:]
    if len(raw) >= 2 and raw[0:2] == b"\x1f\x8b":
        raw = gzip.decompress(raw)
    return raw


def detect_format(payload: bytes) -> int:
    if payload.startswith(b"070701") or payload.startswith(b"070702"):
        return FORMAT_CPIO_NEWC
    if len(payload) >= 512 and payload[257:263] in (b"ustar\x00", b"ustar "):
        return FORMAT_TAR_USTAR
    return 0


def checksum(payload: bytes) -> int:
    value = 0
    for byte in payload:
        value = (value + byte) & 0xFFFFFFFF
    return value


def format_name(fmt: int) -> str:
    if fmt == FORMAT_CPIO_NEWC:
        return "cpio-newc"
    if fmt == FORMAT_TAR_USTAR:
        return "tar-ustar"
    return "unknown"


if __name__ == "__main__":
    raise SystemExit(main())
