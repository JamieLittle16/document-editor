#!/usr/bin/env python3
"""Fail CI when internal Rust crate dependencies bypass documented architecture."""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# This is intentionally explicit. A new crate or production/build dependency edge should require
# an architectural decision in the same change rather than silently expanding the shipping graph.
# R0A harness exceptions (notably desktop -> mock/session) are documented by their presence here
# and should be removed as the production app-core path replaces them.
ALLOWED_INTERNAL_DEPENDENCIES: dict[str, set[str]] = {
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

# Test-only edges are intentionally separate from the shipping graph above. A crate listed here may
# use the dependency from [dev-dependencies], but moving the same edge into [dependencies] or
# [build-dependencies] still fails the guard. This keeps deterministic test harnesses from becoming
# accidental product architecture.
ALLOWED_INTERNAL_DEV_DEPENDENCIES: dict[str, set[str]] = {
    "app-core": {"document-engine-mock", "document-protocol"},
}

PRODUCTION_DEPENDENCY_TABLES = ("dependencies", "build-dependencies")
DEV_DEPENDENCY_TABLES = ("dev-dependencies",)


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


def path_dependencies(
    manifest: dict[str, object], table_names: tuple[str, ...]
) -> set[str]:
    dependencies: set[str] = set()
    for table_name in table_names:
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

        allowed_production = ALLOWED_INTERNAL_DEPENDENCIES[name]
        allowed_dev = allowed_production | ALLOWED_INTERNAL_DEV_DEPENDENCIES.get(name, set())

        actual_production = (
            path_dependencies(manifest, PRODUCTION_DEPENDENCY_TABLES) & known_crates
        )
        actual_dev = path_dependencies(manifest, DEV_DEPENDENCY_TABLES) & known_crates

        forbidden_production = actual_production - allowed_production
        if forbidden_production:
            errors.append(
                f"{name} has forbidden production/build internal dependency edge(s): "
                + ", ".join(sorted(forbidden_production))
            )

        forbidden_dev = actual_dev - allowed_dev
        if forbidden_dev:
            errors.append(
                f"{name} has forbidden test-only internal dependency edge(s): "
                + ", ".join(sorted(forbidden_dev))
            )

    stale_policy = set(ALLOWED_INTERNAL_DEPENDENCIES) - known_crates
    if stale_policy:
        errors.append(
            "architecture dependency policy names missing crate(s): "
            + ", ".join(sorted(stale_policy))
        )

    stale_dev_policy = set(ALLOWED_INTERNAL_DEV_DEPENDENCIES) - known_crates
    if stale_dev_policy:
        errors.append(
            "architecture dev-dependency policy names missing crate(s): "
            + ", ".join(sorted(stale_dev_policy))
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
