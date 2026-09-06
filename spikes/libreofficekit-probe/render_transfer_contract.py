#!/usr/bin/env python3
"""Qualify structural properties of the real Writer render-transfer workload.

Timing values are intentionally observational. The contract pins byte geometry and
successful real-engine rendering, not CI-machine performance.
"""

from __future__ import annotations

import subprocess
import sys


def parse_observations(stdout: str) -> dict[str, str]:
    observations: dict[str, str] = {}
    for line in stdout.splitlines():
        key, separator, value = line.partition("=")
        if separator:
            observations[key] = value
    return observations


def require(observations: dict[str, str], key: str, expected: str) -> None:
    actual = observations.get(key)
    if actual != expected:
        raise RuntimeError(
            f"render-transfer qualification lost {key}: "
            f"expected {expected!r}, observed {actual!r}"
        )


def require_positive_int(observations: dict[str, str], key: str) -> int:
    raw = observations.get(key)
    if raw is None:
        raise RuntimeError(f"render-transfer qualification omitted {key}")
    try:
        value = int(raw)
    except ValueError as error:
        raise RuntimeError(f"render-transfer qualification emitted non-integer {key}: {raw!r}") from error
    if value <= 0:
        raise RuntimeError(f"render-transfer qualification emitted non-positive {key}: {value}")
    return value


def parse_dimensions(value: str, key: str) -> tuple[int, int]:
    left, separator, right = value.partition("x")
    if not separator:
        raise RuntimeError(f"render-transfer qualification emitted malformed {key}: {value!r}")
    try:
        width = int(left)
        height = int(right)
    except ValueError as error:
        raise RuntimeError(f"render-transfer qualification emitted malformed {key}: {value!r}") from error
    if width <= 0 or height <= 0:
        raise RuntimeError(f"render-transfer qualification emitted non-positive {key}: {value!r}")
    return width, height


def main() -> int:
    if len(sys.argv) != 5:
        raise SystemExit(
            "usage: render_transfer_contract.py PROBE INSTALL_PATH PROFILE_URL INPUT.docx"
        )

    completed = subprocess.run(
        sys.argv[1:],
        capture_output=True,
        text=True,
        check=False,
    )
    sys.stdout.write(completed.stdout)
    sys.stderr.write(completed.stderr)
    if completed.returncode != 0:
        return completed.returncode

    observations = parse_observations(completed.stdout)

    require(observations, "render_transfer_probe_status", "observed")
    require(observations, "render_transfer_timing_policy", "observational-no-ci-threshold")
    require(observations, "render_transfer_bytes_per_pixel", "4")
    require(observations, "render_transfer_logical_viewport_pixels", "1024x768")
    require(observations, "render_transfer_1x_tile_pixels", "256x256")
    require(observations, "render_transfer_2x_tile_pixels", "512x512")
    require(observations, "render_transfer_1x_grid_tiles", "12")
    require(observations, "render_transfer_2x_grid_tiles", "12")

    pixel_mode = observations.get("render_transfer_pixel_mode")
    if pixel_mode not in {"rgba", "bgra"}:
        raise RuntimeError(f"unsupported qualified tile pixel mode: {pixel_mode!r}")

    # These values are arithmetic consequences of the qualified workload geometry,
    # not environment-specific performance goldens.
    require(observations, "render_transfer_1x_raw_bytes_per_tile", "262144")
    require(observations, "render_transfer_2x_raw_bytes_per_tile", "1048576")
    require(observations, "render_transfer_viewport_1x_raw_bytes", "3145728")
    require(observations, "render_transfer_viewport_2x_raw_bytes", "12582912")
    require(observations, "render_transfer_1x_raw_bytes_per_grid_pass", "3145728")
    require(observations, "render_transfer_2x_raw_bytes_per_grid_pass", "12582912")

    document_dimensions = observations.get("render_transfer_document_twips")
    if document_dimensions is None:
        raise RuntimeError("render-transfer qualification omitted document dimensions")
    parse_dimensions(document_dimensions, "render_transfer_document_twips")

    page_1x = observations.get("render_transfer_page_1x_pixels")
    page_2x = observations.get("render_transfer_page_2x_pixels")
    if page_1x is None or page_2x is None:
        raise RuntimeError("render-transfer qualification omitted page pixel dimensions")
    page_1x_width, page_1x_height = parse_dimensions(page_1x, "render_transfer_page_1x_pixels")
    page_2x_width, page_2x_height = parse_dimensions(page_2x, "render_transfer_page_2x_pixels")
    if (page_2x_width, page_2x_height) != (page_1x_width * 2, page_1x_height * 2):
        raise RuntimeError("2x page geometry is not exactly double the 1x pixel dimensions")

    page_1x_bytes = require_positive_int(observations, "render_transfer_page_1x_raw_bytes")
    page_2x_bytes = require_positive_int(observations, "render_transfer_page_2x_raw_bytes")
    if page_2x_bytes != page_1x_bytes * 4:
        raise RuntimeError("2x page raw byte volume is not exactly four times the 1x volume")

    # Timings must be valid observations, but there is deliberately no speed gate.
    for prefix in ("render_transfer_1x", "render_transfer_2x"):
        minimum = require_positive_int(observations, f"{prefix}_grid_min_us")
        median = require_positive_int(observations, f"{prefix}_grid_p50_us")
        if median < minimum:
            raise RuntimeError(f"{prefix} median timing is below its measured minimum")
        require_positive_int(observations, f"{prefix}_checksum")

    print("render_transfer_contract=qualified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
