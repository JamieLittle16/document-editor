# LibreOfficeKit boundary probe

This directory contains a deliberately standalone C++ technical spike. It is **not** part of the product runtime or Rust workspace.

The probe exists to measure the real LibreOfficeKit ABI/runtime boundary before we freeze an FFI layer, transport, rendering payload or engine protocol around assumptions.

## What it proves

On the CI Linux reference environment the probe must:

1. initialise LibreOfficeKit from an explicit installation path and isolated user profile;
2. load a deterministic DOCX fixture;
3. identify it as a text document;
4. initialise tiled rendering;
5. obtain positive document dimensions in TWIPs;
6. render a tile into caller-owned memory;
7. prove the render buffer was written;
8. save a DOCX round trip;
9. reopen the round-tripped document successfully.

## What it deliberately does not prove

- production IPC;
- stable rendering ABI;
- semantic snapshot/object identity;
- transaction mapping;
- editing/undo semantics;
- shared-memory transport;
- Windows/macOS integration;
- Rust FFI safety.

Those require later spikes and contracts.

## Why C++ first

LibreOffice ships an ABI-oriented C API plus a small C++ RAII wrapper. Current tiled-rendering methods are exposed when `LOK_USE_UNSTABLE_API` is enabled. Binding that directly into Rust now would require unsafe code and would risk letting an unstable external ABI shape our stable product API.

Keeping the probe standalone lets us learn the boundary while the production Rust workspace retains `unsafe_code = "forbid"`.

## Local Linux run

On Ubuntu 24.04-compatible systems:

```bash
sudo apt-get install g++ libreofficekit-dev libreoffice-core-nogui libreoffice-writer-nogui
python3 spikes/libreofficekit-probe/make_fixture.py /tmp/lok-probe.docx
mkdir -p /tmp/lok-profile

g++ -std=c++17 -O2 -Wall -Wextra -Wpedantic \
  spikes/libreofficekit-probe/probe.cxx -ldl \
  -o /tmp/lok-probe

/tmp/lok-probe \
  /usr/lib/libreoffice/program \
  file:///tmp/lok-profile \
  /tmp/lok-probe.docx \
  /tmp/lok-roundtrip.docx
```

CI is the normative R0A qualification environment.
