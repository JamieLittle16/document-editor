#!/usr/bin/env python3
"""Cross-process R0A qualification harness for the native LibreOfficeKit adapter."""

from __future__ import annotations

import os
import struct
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import BinaryIO

MAGIC = b"DETR"
FRAME_VERSION = 1
REQUEST_KIND = 1
RESPONSE_KIND = 2
HEADER = struct.Struct("<4sHBBQI")
MAX_PAYLOAD = 1024

STATUS_OK = 0
STATUS_INVALID_REQUEST = 1
STATUS_LOAD_FAILED = 2

COMMAND_ENGINE_INFO = 1
COMMAND_OPEN = 2
COMMAND_CLOSE = 3
COMMAND_SHUTDOWN = 4


def read_exact(stream: BinaryIO, size: int, *, clean_eof: bool = False) -> bytes | None:
    chunks: list[bytes] = []
    received = 0
    while received < size:
        chunk = stream.read(size - received)
        if not chunk:
            if clean_eof and received == 0:
                return None
            raise RuntimeError(f"truncated native-adapter stream: {received}/{size} bytes")
        chunks.append(chunk)
        received += len(chunk)
    return b"".join(chunks)


def write_frame(stream: BinaryIO, request_id: int, payload: bytes) -> None:
    if len(payload) > MAX_PAYLOAD:
        raise ValueError("R0A native-adapter payload exceeds qualification limit")
    stream.write(
        HEADER.pack(
            MAGIC,
            FRAME_VERSION,
            REQUEST_KIND,
            0,
            request_id,
            len(payload),
        )
    )
    stream.write(payload)
    stream.flush()


def read_frame(stream: BinaryIO) -> tuple[int, bytes] | None:
    header_bytes = read_exact(stream, HEADER.size, clean_eof=True)
    if header_bytes is None:
        return None
    magic, version, kind, flags, request_id, payload_len = HEADER.unpack(header_bytes)
    if magic != MAGIC:
        raise RuntimeError(f"bad native-adapter frame magic: {magic!r}")
    if version != FRAME_VERSION:
        raise RuntimeError(f"unexpected native-adapter frame version: {version}")
    if kind != RESPONSE_KIND:
        raise RuntimeError(f"unexpected native-adapter response kind: {kind}")
    if flags != 0:
        raise RuntimeError(f"unexpected native-adapter flags: {flags}")
    if payload_len > MAX_PAYLOAD:
        raise RuntimeError(f"native-adapter response exceeds bound: {payload_len}")
    payload = read_exact(stream, payload_len)
    assert payload is not None
    return request_id, payload


class NativeAdapter:
    def __init__(self, executable: Path, install: Path, root: Path) -> None:
        self.profile = root / "profile"
        self.home = root / "home"
        self.profile.mkdir(parents=True)
        self.home.mkdir(parents=True)
        env = os.environ.copy()
        env["HOME"] = str(self.home)
        self.process = subprocess.Popen(
            [str(executable), str(install), self.profile.as_uri()],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
        )
        assert self.process.stdin is not None
        assert self.process.stdout is not None
        assert self.process.stderr is not None
        self.stdin = self.process.stdin
        self.stdout = self.process.stdout
        self.stderr = self.process.stderr

    def request(self, request_id: int, payload: bytes) -> bytes:
        write_frame(self.stdin, request_id, payload)
        response = read_frame(self.stdout)
        if response is None:
            stderr = self.stderr.read().decode("utf-8", errors="replace")
            raise RuntimeError(
                f"native adapter exited before response id={request_id}; "
                f"status={self.process.poll()} stderr={stderr!r}"
            )
        response_id, response_payload = response
        if response_id != request_id:
            raise RuntimeError(
                f"request correlation mismatch: sent {request_id}, received {response_id}"
            )
        return response_payload

    def open_document(self, request_id: int, path: Path) -> tuple[int, int]:
        payload = self.request(request_id, bytes([COMMAND_OPEN]) + os.fsencode(path))
        if len(payload) != 19 or payload[0:3] != bytes([STATUS_OK, COMMAND_OPEN, 1]):
            raise RuntimeError(f"unexpected open response: {payload!r}")
        width, height = struct.unpack_from("<QQ", payload, 3)
        if width <= 0 or height <= 0:
            raise RuntimeError(f"invalid native-adapter layout: {width}x{height}")
        return width, height

    def graceful_shutdown(self, request_id: int) -> str:
        payload = self.request(request_id, bytes([COMMAND_SHUTDOWN]))
        if payload != bytes([STATUS_OK, COMMAND_SHUTDOWN]):
            raise RuntimeError(f"unexpected shutdown response: {payload!r}")
        self.stdin.close()
        status = self.process.wait(timeout=10)
        if status != 0:
            stderr = self.stderr.read().decode("utf-8", errors="replace")
            raise RuntimeError(f"native adapter shutdown failed: {status} {stderr!r}")
        if read_frame(self.stdout) is not None:
            raise RuntimeError("native adapter emitted a frame after shutdown")
        return self.stderr.read().decode("utf-8", errors="replace")


def check_engine_info(adapter: NativeAdapter, request_id: int) -> str:
    payload = adapter.request(request_id, bytes([COMMAND_ENGINE_INFO]))
    if len(payload) < 3 or payload[0:2] != bytes([STATUS_OK, COMMAND_ENGINE_INFO]):
        raise RuntimeError(f"unexpected engine-info response: {payload!r}")
    version = payload[2:].decode("utf-8")
    if "LibreOffice" not in version:
        raise RuntimeError(f"engine-info response does not identify LibreOffice: {version!r}")
    return version


def check_typed_load_failure(adapter: NativeAdapter, request_id: int, missing: Path) -> None:
    payload = adapter.request(request_id, bytes([COMMAND_OPEN]) + os.fsencode(missing))
    if len(payload) < 2 or payload[0:2] != bytes([STATUS_LOAD_FAILED, COMMAND_OPEN]):
        raise RuntimeError(f"missing document did not return typed load failure: {payload!r}")


def count_profile_files(profile: Path) -> int:
    return sum(1 for path in profile.rglob("*") if path.is_file())


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit("usage: process_harness.py ADAPTER INSTALL_PATH INPUT.docx")

    executable = Path(sys.argv[1]).resolve()
    install = Path(sys.argv[2]).resolve()
    input_docx = Path(sys.argv[3]).resolve()

    with tempfile.TemporaryDirectory(prefix="document-editor-lok-process-") as temp:
        root = Path(temp)

        graceful = NativeAdapter(executable, install, root / "graceful")
        version = check_engine_info(graceful, 0x1122334455667788)
        width, height = graceful.open_document(2, input_docx)
        close_payload = graceful.request(3, bytes([COMMAND_CLOSE]))
        if close_payload != bytes([STATUS_OK, COMMAND_CLOSE]):
            raise RuntimeError(f"unexpected close response: {close_payload!r}")
        check_typed_load_failure(graceful, 4, root / "does-not-exist.docx")
        graceful.graceful_shutdown(5)

        profile_files = count_profile_files(graceful.profile)
        if profile_files == 0:
            raise RuntimeError("explicit LibreOffice profile remained empty after engine use")
        if (graceful.home / ".config" / "libreoffice").exists():
            raise RuntimeError("LibreOffice unexpectedly used HOME profile instead of explicit profile URL")

        crashed = NativeAdapter(executable, install, root / "crashed")
        crash_width, crash_height = crashed.open_document(10, input_docx)
        crashed.process.kill()
        crash_status = crashed.process.wait(timeout=10)
        if crash_status == 0:
            raise RuntimeError("force-killed native adapter reported successful exit")
        crashed.stdin.close()
        if read_frame(crashed.stdout) is not None:
            raise RuntimeError("force-killed native adapter emitted a complete response after death")
        crashed.stderr.read()

        restarted = NativeAdapter(executable, install, root / "restarted")
        restart_width, restart_height = restarted.open_document(11, input_docx)
        restarted.graceful_shutdown(12)

        invalid = NativeAdapter(executable, install, root / "invalid-command")
        invalid_payload = invalid.request(20, bytes([99]))
        if len(invalid_payload) < 2 or invalid_payload[0:2] != bytes(
            [STATUS_INVALID_REQUEST, 99]
        ):
            raise RuntimeError(f"invalid command was not typed: {invalid_payload!r}")
        invalid.graceful_shutdown(21)

        print(f"native_adapter_version_json={version}")
        print(f"native_adapter_width_twips={width}")
        print(f"native_adapter_height_twips={height}")
        print(f"native_adapter_crash_open_twips={crash_width}x{crash_height}")
        print(f"native_adapter_restart_open_twips={restart_width}x{restart_height}")
        print(f"native_adapter_profile_files={profile_files}")
        print("native_adapter_typed_load_failure=ok")
        print("native_adapter_invalid_command=ok")
        print("native_adapter_graceful_exit=ok")
        print("native_adapter_forced_exit=observed")
        print("native_adapter_restart=ok")
        print("native_adapter_status=ok")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
