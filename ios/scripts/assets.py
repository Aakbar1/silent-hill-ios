"""Original geometric placeholder; no game artwork. Python standard library only."""
import json
from pathlib import Path
import struct
import zlib


def png(size: int) -> bytes:
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    rows = bytearray()
    for y in range(size):
        rows.append(0)
        for x in range(size):
            nx, ny = x / size - .5, y / size - .5
            ring = .20 < (nx * nx + ny * ny) ** .5 < .26
            cross = (abs(nx) < .035 and abs(ny) < .33) or (abs(ny) < .035 and abs(nx) < .33)
            rows.extend((64, 255, 204) if ring or cross else (7, 9, 15))
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")


def generate(root: Path):
    icon = root / "AppIcon.appiconset"
    icon.mkdir(parents=True, exist_ok=True)
    (root / "Contents.json").write_text(json.dumps({"info": {"version": 1, "author": "xcode"}}))
    images = []
    for points, scale in [(20, 2), (20, 3), (29, 2), (29, 3), (40, 2), (40, 3), (60, 2), (60, 3), (1024, 1)]:
        filename = f"icon-{points}-{scale}.png"
        (icon / filename).write_bytes(png(points * scale))
        images.append({"idiom": "ios-marketing" if points == 1024 else "iphone", "size": f"{points}x{points}", "scale": f"{scale}x", "filename": filename})
    (icon / "Contents.json").write_text(json.dumps({"images": images, "info": {"version": 1, "author": "xcode"}}, indent=2))
