"""Run versioned compatibility fixtures through the qualified LibreOffice seam."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Final

from .docx_semantics import ParagraphTextProjection, project_paragraph_text

MANIFEST_SCHEMA: Final = "office.compatibility-manifest.v1"
RESULT_SCHEMA: Final = "office.compatibility-result.v1"
PROJECTION: Final = "docx-paragraph-text-v1"
GENERATOR: Final = "writer-r0a-basic-v1"
OPERATION: Final = "lok-r0a-prefix-edit-v1"
EDIT_MARKER: Final = "R0A_EDIT_MARKER_7F3D"
MAX_MANIFEST_BYTES: Final = 1 << 20
MAX_FIXTURES: Final = 256
MAX_PARAGRAPHS: Final = 4096
MAX_PARAGRAPH_BYTES: Final = 1 << 20
MAX_DOCX_BYTES: Final = 64 << 20
GENERATOR_TIMEOUT_SECONDS: Final = 15
PROBE_TIMEOUT_SECONDS: Final = 60
FIXTURE_ID = re.compile(r"^[a-z0-9][a-z0-9.-]{0,127}$")


class CompatibilityError(RuntimeError):
    """A fixture or harness contract was invalid or failed."""


@dataclass(frozen=True)
class FixtureSpec:
    fixture_id: str
    generator: str
    operation: str
    projection: str
    expected_before: tuple[str, ...]
    expected_after: tuple[str, ...]


@dataclass(frozen=True)
class Manifest:
    fixtures: tuple[FixtureSpec, ...]


def _expect_exact_keys(value: dict[str, object], expected: set[str], context: str) -> None:
    actual = set(value)
    if actual != expected:
        missing = sorted(expected - actual)
        extra = sorted(actual - expected)
        raise CompatibilityError(
            f"{context} keys changed: missing={missing!r}, extra={extra!r}"
        )


def _paragraphs(value: object, context: str) -> tuple[str, ...]:
    if not isinstance(value, list):
        raise CompatibilityError(f"{context} must be a JSON array")
    if len(value) > MAX_PARAGRAPHS:
        raise CompatibilityError(
            f"{context} exceeds paragraph bound {MAX_PARAGRAPHS}"
        )
    paragraphs: list[str] = []
    for index, paragraph in enumerate(value):
        if not isinstance(paragraph, str):
            raise CompatibilityError(f"{context}[{index}] must be a string")
        if len(paragraph.encode("utf-8")) > MAX_PARAGRAPH_BYTES:
            raise CompatibilityError(
                f"{context}[{index}] exceeds UTF-8 byte bound {MAX_PARAGRAPH_BYTES}"
            )
        paragraphs.append(paragraph)
    return tuple(paragraphs)


def load_manifest(path: Path) -> Manifest:
    if not path.is_file():
        raise CompatibilityError(f"manifest does not exist: {path}")
    if path.stat().st_size > MAX_MANIFEST_BYTES:
        raise CompatibilityError(
            f"manifest exceeds byte bound {MAX_MANIFEST_BYTES}: {path}"
        )

    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise CompatibilityError(f"could not decode manifest {path}: {error}") from error

    if not isinstance(raw, dict):
        raise CompatibilityError("manifest root must be a JSON object")
    _expect_exact_keys(raw, {"schema", "fixtures"}, "manifest")
    if raw["schema"] != MANIFEST_SCHEMA:
        raise CompatibilityError(
            f"unsupported manifest schema: expected {MANIFEST_SCHEMA!r}, "
            f"observed {raw['schema']!r}"
        )
    raw_fixtures = raw["fixtures"]
    if not isinstance(raw_fixtures, list) or not raw_fixtures:
        raise CompatibilityError("manifest fixtures must be a non-empty JSON array")
    if len(raw_fixtures) > MAX_FIXTURES:
        raise CompatibilityError(f"manifest exceeds fixture bound {MAX_FIXTURES}")

    fixtures: list[FixtureSpec] = []
    seen_ids: set[str] = set()
    expected_keys = {
        "id",
        "generator",
        "operation",
        "projection",
        "expected_before",
        "expected_after",
    }
    for index, raw_fixture in enumerate(raw_fixtures):
        context = f"fixtures[{index}]"
        if not isinstance(raw_fixture, dict):
            raise CompatibilityError(f"{context} must be a JSON object")
        _expect_exact_keys(raw_fixture, expected_keys, context)

        fixture_id = raw_fixture["id"]
        generator = raw_fixture["generator"]
        operation = raw_fixture["operation"]
        projection = raw_fixture["projection"]
        if not isinstance(fixture_id, str) or FIXTURE_ID.fullmatch(fixture_id) is None:
            raise CompatibilityError(f"{context}.id is not a safe fixture identifier")
        if fixture_id in seen_ids:
            raise CompatibilityError(f"duplicate fixture id {fixture_id!r}")
        seen_ids.add(fixture_id)
        for field, value in (
            ("generator", generator),
            ("operation", operation),
            ("projection", projection),
        ):
            if not isinstance(value, str):
                raise CompatibilityError(f"{context}.{field} must be a string")

        fixtures.append(
            FixtureSpec(
                fixture_id=fixture_id,
                generator=generator,
                operation=operation,
                projection=projection,
                expected_before=_paragraphs(
                    raw_fixture["expected_before"], f"{context}.expected_before"
                ),
                expected_after=_paragraphs(
                    raw_fixture["expected_after"], f"{context}.expected_after"
                ),
            )
        )

    return Manifest(tuple(fixtures))


def parse_key_values(stdout: str) -> dict[str, str]:
    observations: dict[str, str] = {}
    for line in stdout.splitlines():
        key, separator, value = line.partition("=")
        if separator:
            observations[key] = value
    return observations


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def ensure_bounded_docx(path: Path, context: str) -> None:
    if not path.is_file() or path.stat().st_size == 0:
        raise CompatibilityError(f"{context} DOCX is missing or empty: {path}")
    if path.stat().st_size > MAX_DOCX_BYTES:
        raise CompatibilityError(
            f"{context} DOCX exceeds byte bound {MAX_DOCX_BYTES}: {path}"
        )


def assert_projection(
    actual: ParagraphTextProjection,
    expected: tuple[str, ...],
    context: str,
) -> None:
    if actual.paragraphs != expected:
        raise CompatibilityError(
            f"{context} semantic projection mismatch: "
            f"expected {expected!r}, observed {actual.paragraphs!r}"
        )


def _write_text(path: Path, value: str) -> None:
    path.write_text(value, encoding="utf-8")


def _write_json(path: Path, value: object) -> None:
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def _run_generator(repo_root: Path, spec: FixtureSpec, input_path: Path, artifact: Path) -> None:
    if spec.generator != GENERATOR:
        raise CompatibilityError(
            f"fixture {spec.fixture_id!r} uses unsupported generator {spec.generator!r}"
        )
    generator = repo_root / "spikes/libreofficekit-probe/make_fixture.py"
    completed = subprocess.run(
        [sys.executable, str(generator), str(input_path)],
        capture_output=True,
        text=True,
        check=False,
        timeout=GENERATOR_TIMEOUT_SECONDS,
    )
    _write_text(artifact / "generator.stdout.txt", completed.stdout)
    _write_text(artifact / "generator.stderr.txt", completed.stderr)
    if completed.returncode != 0:
        raise CompatibilityError(
            f"fixture generator exited with status {completed.returncode}"
        )


def _run_operation(
    spec: FixtureSpec,
    probe: Path,
    install_path: Path,
    profile_root: Path,
    input_path: Path,
    output_path: Path,
    artifact: Path,
) -> None:
    if spec.operation != OPERATION:
        raise CompatibilityError(
            f"fixture {spec.fixture_id!r} uses unsupported operation {spec.operation!r}"
        )
    if not probe.is_file():
        raise CompatibilityError(f"LibreOfficeKit probe does not exist: {probe}")
    if not install_path.is_dir():
        raise CompatibilityError(f"LibreOffice install path does not exist: {install_path}")

    profile = profile_root / spec.fixture_id
    profile.mkdir(parents=True, exist_ok=True)
    completed = subprocess.run(
        [
            str(probe),
            str(install_path),
            profile.resolve().as_uri(),
            str(input_path),
            str(output_path),
        ],
        capture_output=True,
        text=True,
        check=False,
        timeout=PROBE_TIMEOUT_SECONDS,
    )
    _write_text(artifact / "probe.stdout.txt", completed.stdout)
    _write_text(artifact / "probe.stderr.txt", completed.stderr)
    if completed.returncode != 0:
        raise CompatibilityError(
            f"LibreOfficeKit compatibility operation exited with status {completed.returncode}"
        )

    observations = parse_key_values(completed.stdout)
    required = {
        "text_edit": "ok",
        "text_edit_marker": EDIT_MARKER,
        "roundtrip_reopen": "ok",
        "probe_status": "ok",
    }
    for key, expected in required.items():
        actual = observations.get(key)
        if actual != expected:
            raise CompatibilityError(
                f"compatibility operation lost {key}: "
                f"expected {expected!r}, observed {actual!r}"
            )


def run_fixture(
    repo_root: Path,
    spec: FixtureSpec,
    probe: Path,
    install_path: Path,
    profile_root: Path,
    artifact_root: Path,
) -> dict[str, object]:
    if spec.projection != PROJECTION:
        raise CompatibilityError(
            f"fixture {spec.fixture_id!r} uses unsupported projection {spec.projection!r}"
        )

    artifact = artifact_root / spec.fixture_id
    if artifact.exists():
        shutil.rmtree(artifact)
    artifact.mkdir(parents=True)
    input_path = artifact / "input.docx"
    output_path = artifact / "roundtrip.docx"

    _run_generator(repo_root, spec, input_path, artifact)
    ensure_bounded_docx(input_path, "generated input")
    before = project_paragraph_text(input_path)
    assert_projection(before, spec.expected_before, f"{spec.fixture_id} before")

    _run_operation(
        spec,
        probe,
        install_path,
        profile_root,
        input_path,
        output_path,
        artifact,
    )
    ensure_bounded_docx(output_path, "round-trip output")
    after = project_paragraph_text(output_path)
    assert_projection(after, spec.expected_after, f"{spec.fixture_id} after")

    result: dict[str, object] = {
        "schema": RESULT_SCHEMA,
        "fixture_id": spec.fixture_id,
        "status": "passed",
        "generator": spec.generator,
        "operation": spec.operation,
        "before": before.as_json(),
        "after": after.as_json(),
        "input_sha256": sha256(input_path),
        "roundtrip_sha256": sha256(output_path),
    }
    _write_json(artifact / "result.json", result)
    print(f"compatibility_fixture={spec.fixture_id}")
    print(f"compatibility_before_paragraphs={len(before.paragraphs)}")
    print(f"compatibility_after_paragraphs={len(after.paragraphs)}")
    print("compatibility_semantic_assertions=ok")
    print("compatibility_roundtrip_package=ok")
    return result


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run normalized Office compatibility fixtures through LibreOfficeKit"
    )
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--install-path", type=Path, required=True)
    parser.add_argument("--profile-root", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    repo_root = Path(__file__).resolve().parents[2]
    try:
        manifest = load_manifest(args.manifest)
        args.profile_root.mkdir(parents=True, exist_ok=True)
        args.artifact_root.mkdir(parents=True, exist_ok=True)

        results: list[dict[str, object]] = []
        for spec in manifest.fixtures:
            try:
                results.append(
                    run_fixture(
                        repo_root,
                        spec,
                        args.probe,
                        args.install_path,
                        args.profile_root,
                        args.artifact_root,
                    )
                )
            except Exception as error:
                artifact = args.artifact_root / spec.fixture_id
                artifact.mkdir(parents=True, exist_ok=True)
                _write_json(
                    artifact / "result.json",
                    {
                        "schema": RESULT_SCHEMA,
                        "fixture_id": spec.fixture_id,
                        "status": "failed",
                        "error": str(error),
                    },
                )
                raise

        _write_json(
            args.artifact_root / "summary.json",
            {
                "schema": RESULT_SCHEMA,
                "status": "passed",
                "fixture_count": len(results),
                "fixtures": [result["fixture_id"] for result in results],
            },
        )
        print(f"compatibility_fixture_count={len(results)}")
        print("compatibility_harness_status=qualified")
        return 0
    except (CompatibilityError, OSError, subprocess.SubprocessError) as error:
        print(f"compatibility_harness_error={error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
