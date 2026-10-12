from __future__ import annotations

import hashlib
import json
import struct
import sys
import tempfile
import unittest
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
from render_static_fallback import (  # noqa: E402
    PNG_SIGNATURE,
    PreviewError,
    encode_png_rgba,
    main,
    render_static_fallback_rgba,
)

EXAMPLE_PATH = ROOT / "contracts/examples/first-germination.scene.json"


def decode_png(png: bytes) -> tuple[int, int, bytes, list[bytes]]:
    if not png.startswith(PNG_SIGNATURE):
        raise AssertionError("bad PNG signature")
    offset = len(PNG_SIGNATURE)
    chunks: list[tuple[bytes, bytes]] = []
    while offset < len(png):
        length = struct.unpack_from(">I", png, offset)[0]
        kind = png[offset + 4:offset + 8]
        data = png[offset + 8:offset + 8 + length]
        expected_crc = struct.unpack_from(">I", png, offset + 8 + length)[0]
        actual_crc = zlib.crc32(data, zlib.crc32(kind)) & 0xFFFF_FFFF
        if actual_crc != expected_crc:
            raise AssertionError(f"CRC mismatch in {kind!r}")
        chunks.append((kind, data))
        offset += length + 12
    kinds = [kind for kind, _ in chunks]
    if kinds != [b"IHDR", b"IDAT", b"IEND"]:
        raise AssertionError(f"unexpected PNG chunks: {kinds!r}")
    width, height, bit_depth, color_type, compression, filtering, interlace = struct.unpack(
        ">IIBBBBB", chunks[0][1]
    )
    if (bit_depth, color_type, compression, filtering, interlace) != (8, 6, 0, 0, 0):
        raise AssertionError("unexpected PNG format")
    raw = zlib.decompress(chunks[1][1])
    stride = width * 4
    rows = []
    cursor = 0
    for _ in range(height):
        if raw[cursor] != 0:
            raise AssertionError("expected unfiltered scanline")
        cursor += 1
        rows.append(raw[cursor:cursor + stride])
        cursor += stride
    if cursor != len(raw):
        raise AssertionError("unexpected extra PNG scanline data")
    return width, height, b"".join(rows), kinds


class StaticFallbackPreviewTests(unittest.TestCase):
    def test_renders_reference_fallback_endpoint_pixels(self):
        rgba = render_static_fallback_rgba(
            2, 2, "#0A100E", "#1A2E22", 0.58
        )
        self.assertEqual(len(rgba), 16)
        self.assertEqual(rgba[:4], bytes([6, 9, 8, 255]))
        self.assertEqual(rgba[-4:], bytes([15, 27, 20, 255]))

    def test_png_is_valid_metadata_free_and_byte_reproducible(self):
        rgba = render_static_fallback_rgba(3, 2, "#0A100E", "#1A2E22", 0.58)
        first = encode_png_rgba(3, 2, rgba)
        second = encode_png_rgba(3, 2, rgba)
        self.assertEqual(first, second)
        width, height, decoded, chunks = decode_png(first)
        self.assertEqual((width, height), (3, 2))
        self.assertEqual(decoded, rgba)
        self.assertEqual(chunks, [b"IHDR", b"IDAT", b"IEND"])

    def test_one_pixel_preview_uses_canvas_endpoint(self):
        rgba = render_static_fallback_rgba(1, 1, "#0A100E", "#1A2E22", 1.0)
        self.assertEqual(rgba, bytes([10, 16, 14, 255]))

    def test_png_multiple_stored_deflate_blocks_round_trip(self):
        width, height = 200, 100
        rgba = bytes([17, 23, 31, 255]) * (width * height)
        png = encode_png_rgba(width, height, rgba)
        decoded_width, decoded_height, decoded_rgba, _ = decode_png(png)
        self.assertEqual((decoded_width, decoded_height), (width, height))
        self.assertEqual(decoded_rgba, rgba)

    def test_invalid_brightness_and_dimensions_fail_closed(self):
        for brightness in (float("nan"), float("inf"), -0.1, 1.1):
            with self.subTest(brightness=brightness):
                with self.assertRaises(PreviewError):
                    render_static_fallback_rgba(
                        2, 2, "#0A100E", "#1A2E22", brightness
                    )
        for width, height in ((0, 1), (1, -1), (4097, 1), (4096, 2160)):
            with self.subTest(dimensions=(width, height)):
                with self.assertRaises(PreviewError):
                    render_static_fallback_rgba(
                        width, height, "#0A100E", "#1A2E22", 1.0
                    )

    def test_invalid_palette_is_rejected(self):
        with self.assertRaisesRegex(PreviewError, "palette.canvas"):
            render_static_fallback_rgba(2, 2, "green", "#1A2E22", 1.0)

    def test_cli_validates_pack_then_writes_preview(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "nested" / "first-germination.png"
            status = main([
                str(EXAMPLE_PATH),
                "--width", "2",
                "--height", "2",
                "--output", str(output),
            ])
            self.assertEqual(status, 0)
            self.assertTrue(output.is_file())
            width, height, rgba, _ = decode_png(output.read_bytes())
            self.assertEqual((width, height), (2, 2))
            self.assertEqual(rgba[:4], bytes([6, 9, 8, 255]))

    def test_cli_refuses_to_overwrite_the_manifest(self):
        status = main([
            str(EXAMPLE_PATH),
            "--width", "2",
            "--height", "2",
            "--output", str(EXAMPLE_PATH),
        ])
        self.assertEqual(status, 2)
 
    def test_cli_refuses_to_overwrite_a_declared_asset(self):
        with tempfile.TemporaryDirectory() as directory:
            package = Path(directory)
            manifest_path = package / "scene.json"
            asset_path = package / "preview.svg"
            manifest = json.loads(EXAMPLE_PATH.read_text(encoding="utf-8"))
            asset_bytes = b"<svg xmlns='http://www.w3.org/2000/svg'/>"
            asset_path.write_bytes(asset_bytes)
            manifest["assets"] = [{
                "assetId": "preview-image",
                "path": "preview.svg",
                "sha256": hashlib.sha256(asset_bytes).hexdigest(),
                "mediaType": "image/svg+xml",
                "license": {"spdxId": "MIT"},
            }]
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            status = main([
                str(manifest_path),
                "--asset-root", str(package),
                "--width", "2",
                "--height", "2",
                "--output", str(asset_path),
            ])
            self.assertEqual(status, 2)
            self.assertEqual(asset_path.read_bytes(), asset_bytes)


if __name__ == "__main__":
    unittest.main(verbosity=2)
