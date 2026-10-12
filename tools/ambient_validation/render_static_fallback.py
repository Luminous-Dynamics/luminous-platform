#!/usr/bin/env python3
"""Render the validated Scene Pack v1 staticFallback to a deterministic PNG.

This is a contract-level fallback preview, not the animated renderer and not
runtime or platform qualification. It intentionally uses only the Python
standard library for pixel generation and PNG encoding.
"""
from __future__ import annotations

import argparse
import hashlib
import math
import re
import struct
import sys
import zlib
from pathlib import Path
from typing import Any

from validate_scene_pack import DEFAULT_SCHEMA, load_json_file, validate_manifest

ROOT = Path(__file__).resolve().parents[2]
Q16_ONE = 65_535
MAX_DIMENSION = 4_096
MAX_PIXELS = 8_294_400
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


class PreviewError(ValueError):
    """The pack or requested preview cannot be rendered safely."""


def _rgb_from_hex(value: Any, field: str) -> tuple[int, int, int]:
    if not isinstance(value, str) or re.fullmatch(r"#[0-9A-Fa-f]{6}", value) is None:
        raise PreviewError(f"{field} must be a #RRGGBB color")
    return tuple(int(value[offset:offset + 2], 16) for offset in (1, 3, 5))


def _to_f32(value: float) -> float:
    """Match the f32 precision used by the Rust Scene Pack parser."""
    return struct.unpack("!f", struct.pack("!f", float(value)))[0]


def validate_dimensions(width: int, height: int) -> None:
    if type(width) is not int or type(height) is not int or width < 1 or height < 1:
        raise PreviewError("preview dimensions must be positive integers")
    if width > MAX_DIMENSION or height > MAX_DIMENSION:
        raise PreviewError(f"preview width and height must not exceed {MAX_DIMENSION}")
    if width * height > MAX_PIXELS:
        raise PreviewError(f"preview must not exceed {MAX_PIXELS} pixels")


def render_static_fallback_rgba(
    width: int,
    height: int,
    canvas: str,
    substrate: str,
    brightness: float,
) -> bytes:
    """Render the contract's integer-fixed-point canvas-to-substrate gradient.

    Interpolation follows sovereign-boot's render_static_gradient_rgba:
    brightness is first rounded to f32, then quantized to Q16; pixel projection,
    color interpolation, and brightness scaling use integer arithmetic.
    """
    validate_dimensions(width, height)
    if not isinstance(brightness, (int, float)) or isinstance(brightness, bool):
        raise PreviewError("staticFallback.brightness must be numeric")
    brightness_f32 = _to_f32(float(brightness))
    if not math.isfinite(brightness_f32) or not 0.0 <= brightness_f32 <= 1.0:
        raise PreviewError("staticFallback.brightness must be finite and in 0..=1")

    start = _rgb_from_hex(canvas, "palette.canvas")
    end = _rgb_from_hex(substrate, "palette.substrate")
    brightness_q16 = math.floor(float(brightness_f32) * Q16_ONE + 0.5)
    dx, dy = width - 1, height - 1
    denominator = dx * dx + dy * dy
    rgba = bytearray(width * height * 4)
    offset = 0

    for y in range(height):
        for x in range(width):
            projection = x * dx + y * dy
            if denominator == 0:
                t_q16 = 0
            else:
                t_q16 = min(
                    Q16_ONE,
                    (projection * Q16_ONE + denominator // 2) // denominator,
                )
            for start_channel, end_channel in zip(start, end):
                mixed = (
                    start_channel * (Q16_ONE - t_q16)
                    + end_channel * t_q16
                    + Q16_ONE // 2
                ) // Q16_ONE
                channel = (mixed * brightness_q16 + Q16_ONE // 2) // Q16_ONE
                rgba[offset] = channel
                offset += 1
            rgba[offset] = 255
            offset += 1

    return bytes(rgba)


def _png_chunk(kind: bytes, data: bytes) -> bytes:
    crc = zlib.crc32(kind)
    crc = zlib.crc32(data, crc) & 0xFFFF_FFFF
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", crc)


def _zlib_store(data: bytes) -> bytes:
    """Produce a valid, byte-stable zlib stream using uncompressed DEFLATE blocks."""
    result = bytearray(b"\x78\x01")
    block_size = 65_535
    for offset in range(0, len(data), block_size):
        block = data[offset:offset + block_size]
        is_final = offset + len(block) == len(data)
        # Stored DEFLATE block: BFINAL, BTYPE=00, then byte alignment.
        result.append(1 if is_final else 0)
        length = len(block)
        result.extend(struct.pack("<HH", length, 0xFFFF - length))
        result.extend(block)
    result.extend(struct.pack(">I", zlib.adler32(data) & 0xFFFF_FFFF))
    return bytes(result)


def encode_png_rgba(width: int, height: int, rgba: bytes) -> bytes:
    validate_dimensions(width, height)
    expected = width * height * 4
    if len(rgba) != expected:
        raise PreviewError(f"RGBA buffer is {len(rgba)} bytes; expected {expected}")
    stride = width * 4
    scanlines = bytearray((stride + 1) * height)
    target = 0
    for y in range(height):
        scanlines[target] = 0  # PNG filter type None: stable and easy to inspect.
        target += 1
        start = y * stride
        scanlines[target:target + stride] = rgba[start:start + stride]
        target += stride

    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return (
        PNG_SIGNATURE
        + _png_chunk(b"IHDR", ihdr)
        + _png_chunk(b"IDAT", _zlib_store(bytes(scanlines)))
        + _png_chunk(b"IEND", b"")
    )


def _preview_from_manifest(
    manifest: dict[str, Any],
    width: int,
    height: int,
) -> bytes:
    try:
        fallback = manifest["presentations"]["staticFallback"]
        palette = manifest["palette"]
        rgba = render_static_fallback_rgba(
            width,
            height,
            palette["canvas"],
            palette["substrate"],
            fallback["brightness"],
        )
    except (KeyError, TypeError) as exc:
        raise PreviewError(f"manifest has no usable staticFallback palette: {exc}") from exc
    return encode_png_rgba(width, height, rgba)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path, help="Scene Pack v1 JSON manifest")
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA, help="Scene Pack JSON Schema")
    parser.add_argument("--asset-root", type=Path, help="package root used to resolve and hash local assets")
    parser.add_argument("--supports-capability", action="append", default=[],
                        choices=("gpu", "dmabuf", "vulkan", "egl", "wayland-layer-shell"))
    parser.add_argument("--width", type=int, default=640, help="preview width (default: 640)")
    parser.add_argument("--height", type=int, default=360, help="preview height (default: 360)")
    parser.add_argument("--output", type=Path, required=True, help="output PNG path")
    args = parser.parse_args(argv)

    try:
        validate_dimensions(args.width, args.height)
        schema = load_json_file(args.schema)
        manifest = load_json_file(args.manifest)
        package_root = args.asset_root if args.asset_root is not None else args.manifest.parent
        issues = validate_manifest(
            schema,
            manifest,
            package_root=package_root,
            supported_capabilities=set(args.supports_capability),
        )
        if issues:
            for issue in issues:
                print(f"ERROR: {issue}", file=sys.stderr)
            return 1

        output = args.output.resolve()
        protected_inputs = {args.manifest.resolve(), args.schema.resolve()}
        if output in protected_inputs:
            raise PreviewError("output path must not overwrite the manifest or schema")

        png = _preview_from_manifest(manifest, args.width, args.height)
        output.parent.mkdir(parents=True, exist_ok=True)
        # Replace atomically so interruption cannot leave a partial PNG.
        temporary = output.with_name(output.name + ".tmp")
        try:
            temporary.write_bytes(png)
            temporary.replace(output)
        finally:
            if temporary.exists():
                temporary.unlink()

        digest = hashlib.sha256(png).hexdigest()
        print(
            f"PASS: validated Scene Pack v1; wrote staticFallback preview "
            f"{output} ({args.width}x{args.height}, sha256={digest})"
        )
        print("NOTE: static fallback preview only; not an animated-renderer or platform qualification result")
        return 0
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
