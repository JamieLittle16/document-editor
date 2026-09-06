#!/usr/bin/env python3
"""Fail CI when internal Rust crate dependencies bypass documented architecture."""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# This is intentionally explicit. A new crate or dependency edge should require an
# architectural decision in the same change rather than silently expanding the graph.
# R0A harness exceptions (notably desktop -> mock/session) are documented by their
# presence here and should be removed as the production app-core path replaces them.
ALLOWED_INTERNAL_DEPENDENCIES: dict[str, set[str]] = {
    # Product-owned durable logical identity. This stays below application/history/session
    # consumers and must not depend on engine, transport, UI or file-format crates.
    "document-anchors": set(),
    "document-protocol": set(),
    "document-transport": {"document-protocol"},
    "document-engine-api": {"document-protocol"},
    "document-session": {"document-engine-api", "document-protocol"},
    "document-engine-mock": {"document-engine-api", "document-protocol"},
    "extension-api": set(),
    "extension-runtime": {"extension-api"},
    "feature-host": {"extension-api", "extension-runtime"},
    "app-core": {"document-engine-api", "document-session", "extension-runtime"},
    # R0A executable harness: direct mock/session access is temporary and visible.
    "desktop": {
        "app-core",
        "document-engine-mock",
        "document-protocol",
        "document-session",
    },
    # R0A worker/process spike. The transport/protocol edges are the intended process seam;
    # direct mock access disappears when the real adapter replaces the mock harness.
    "document-worker": {
        "document-engine-api",
        "document-engine-mock",
        "document-protocol",
        "document-transport",
    },
}

DEPENDENCY_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")


def manifest_paths() -> list[Path]:
    paths: list[Path] = []
    for directory in ("apps", "crates", "workers"):
        base = ROOT / directory
        if base.exists():
            paths.extend(sorted(base.glob("*/Cargo.toml")))
    return paths


def load_manifest(path: Path) -> dict[str, object]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def crate_name(manifest: dict[str, object], path: Path) -> str:
    package = manifest.get("package")
    if not isinstance(package, dict) or not isinstance(package.get("name"), str):
        raise ValueError(f"{path.relative_to(ROOT)} has no [package].name")
    return package["name"]


def path_dependencies(manifest: dict[str, object]) -> set[str]:
    dependencies: set[str] = set()
    for table_name in DEPENDENCY_TABLES:
        table = manifest.get(table_name, {})
        if not isinstance(table, dict):
            continue
        for dependency_name, specification in table.items():
            if isinstance(specification, dict) and "path" in specification:
                package_name = specification.get("package", dependency_name)
                if isinstance(package_name, str):
                    dependencies.add(package_name)
    return dependencies


def main() -> int:
    manifests: dict[str, tuple[Path, dict[str, object]]] = {}
    errors: list[str] = []

    for path in manifest_paths():
        try:
            manifest = load_manifest(path)
            name = crate_name(manifest, path)
        except (OSError, tomllib.TOMLDecodeError, ValueError) as error:
            errors.append(str(error))
            continue

        if name in manifests:
            errors.append(f"duplicate workspace crate name {name!r}")
        manifests[name] = (path, manifest)

    known_crates = set(manifests)

    for name, (path, manifest) in sorted(manifests.items()):
        if name not in ALLOWED_INTERNAL_DEPENDENCIES:
            errors.append(
                f"{path.relative_to(ROOT)} ({name}) is missing from the architecture dependency policy"
            )
            continue

        actual_internal = path_dependencies(manifest) & known_crates
        allowed = ALLOWED_INTERNAL_DEPENDENCIES[name]
        forbidden = actual_internal - allowed
        if forbidden:
            errors.append(
                f"{name} has forbidden internal dependency edge(s): "
                + ", ".join(sorted(forbidden))
            )

    stale_policy = set(ALLOWED_INTERNAL_DEPENDENCIES) - known_crates
    if stale_policy:
        errors.append(
            "architecture dependency policy names missing crate(s): "
            + ", ".join(sorted(stale_policy))
        )

    if errors:
        print("Architecture guard failed:\n", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        print(
            "\nUpdate code or the explicit policy with the relevant architecture/ADR change; "
            "do not bypass this guard.",
            file=sys.stderr,
        )
        return 1

    print(f"Architecture guard passed for {len(manifests)} workspace crates.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
