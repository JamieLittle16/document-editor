# UI Framework Qualification

Status: **R0A first Slint viability slice qualified — no production framework selected yet**

Date: 2026-09-06

## Why this is an architecture decision

A Word-class editor shell needs more than attractive widgets. The framework must remain a replaceable presentation layer over product-owned commands, authority, history and rendering while still providing first-class desktop integration.

The hard requirements are:

- Windows, macOS and Linux desktop support;
- reliable IME/international text input;
- platform accessibility integration and stable author-provided accessibility IDs for automation;
- menus and keyboard shortcuts;
- clipboard, drag/drop and file-dialog integration;
- high-DPI/multi-monitor correctness;
- efficient composition of large custom document viewports fed from the out-of-band render data plane selected by ADR-0011;
- a credible packaging, licensing and maintenance story;
- no requirement for application/domain crates to know toolkit types.

The UI framework is therefore qualified independently under `spikes/`. Nothing in `apps/desktop` or the foundational workspace depends on a candidate until the decision is accepted.

## Current candidate matrix

This matrix records the current ecosystem state, not a permanent ranking. It must be updated when evidence changes.

| Candidate | Current R0A disposition | Strong evidence | Current concern |
| --- | --- | --- | --- |
| **Slint 1.17.1** | **first executable viability slice qualified; still not selected** | stable 1.x API; Winit desktop backend; AccessKit accessibility feature; explicit accessibility landmarks/IDs; LineEdit/TextInput IME path; MenuBar/shortcuts; caller-owned `SharedPixelBuffer`; software/FemtoVG/Skia renderer choices; current spike compiles/tests on all three target desktop OSes and creates qualified 1×/2× Linux native windows | current crate MSRV is Rust 1.92; generated UI code requires a contained lint boundary rather than product-wide `unsafe_code = "forbid"`; Linux Winit/X11 packaging needs explicit font/XKB runtime dependencies; royalty-free proprietary use requires attribution; desktop feature set is still actively maturing and must be measured |
| **egui/eframe** | fallback/control candidate | mature Rust ecosystem; native AccessKit enabled by default in eframe; strong custom drawing; active IME/DPI work; permissive MIT/Apache licensing | immediate-mode shell ergonomics and native desktop/editor feel need an editor-shaped spike before selection |
| **Xilem/Masonry** | watchlist | strong modern architecture; winit + Vello/wgpu + Parley + AccessKit; Apache-2.0 | project explicitly describes itself as experimental/alpha-quality with missing features and breaking changes; current MSRV 1.92 |
| **Iced 0.14** | reject for current production selection | modern reactive architecture, IME and testing work | stable release still lacks a production AccessKit accessibility integration; accessibility is non-negotiable for Office |
| **GPUI** | reject for current production selection | high-performance editor-proven ideas from Zed; attractive ownership/rendering model | upstream documents pre-1.0/breaking-change status and macOS/Linux user target; accessibility work is recent and still maturing; Windows is not an acceptable assumption for our shell |
| **custom winit + AccessKit + renderer** | escape hatch only | maximum control and no toolkit lock-in | would force Office to own a very large amount of widget, text-input, accessibility and desktop integration infrastructure before product features |

## Slint qualification isolation

Current Slint 1.17.1 declares `rust-version = "1.92"`; the Office product workspace remains deliberately pinned to Rust 1.85.

The spike therefore lives in:

```text
spikes/ui-framework-slint/
```

with its own nested Cargo workspace and Rust 1.92 toolchain. This is intentional:

- qualifying a current toolkit must not silently raise the product MSRV;
- rejecting the toolkit must require deleting only a spike, not untangling product dependencies;
- accepting it later will require an explicit follow-up decision about the product toolchain.

Slint's generated `slint!` Rust contains a scoped `allow(unsafe_code)`. Rust deliberately does not allow that scoped attribute to override a crate-level `forbid(unsafe_code)`, even though Office-authored spike code contains no unsafe block. The qualification crate therefore uses `unsafe_code = "deny"` while the Office product workspace remains at `forbid`.

If Slint is eventually selected, the preferred shape is a small generated-UI adapter crate with a safe product-facing facade. Toolkit/codegen constraints must not weaken the safety policy of application, history, session, recovery or engine crates.

## Executable Slint workload

The qualification shell is editor-shaped rather than a hello-world demo. It compiles:

- a native `Window`;
- `MenuBar` commands with keyboard shortcuts;
- a search `LineEdit` using `input-type: search`, which exercises the framework text-input/IME and clipboard path;
- explicit accessibility landmarks, labels and stable `accessible-id` values;
- a scrollable custom document viewport;
- a zoomable rendered-page surface;
- a caller-owned 256 × 256 RGBA `SharedPixelBuffer` injected as a Slint `Image`;
- a status region.

The deterministic tile is exactly 262,144 bytes, matching the 1× tile data class measured from real Writer in the render-transfer qualification. The UI spike therefore tests the same host-owned pixel-buffer boundary selected by ADR-0011 instead of inventing a second render architecture.

## CI qualification

`.github/workflows/ui-qualification.yml` is separate from the stable product gate.

It performs:

1. compile + tests on Ubuntu, Windows and macOS using Rust 1.92;
2. fmt and clippy on Linux;
3. a real Winit/software-renderer window creation under Xvfb;
4. the native-window smoke at forced 1× and 2× Slint scale factors;
5. structural checking of the deterministic render buffer checksum/byte volume.

The Linux Winit/X11 qualification image explicitly installs `libfontconfig1-dev` and `libxkbcommon-x11-0`; these were discovered from actual build/native-window failures and are packaging evidence rather than assumed runner state.

`SLINT_SCALE_FACTOR` is consumed by both Slint compiler passes and the Winit backend. The DPI qualification therefore recompiles the small candidate crate before the 2× run while retaining already-built dependencies; changing only the runtime environment of a binary first compiled at 1× is not a valid forced-scale test.

Cross-platform compilation is necessary evidence but not sufficient evidence of native UX quality. Final acceptance still requires manual/platform-native IME and screen-reader checks on real Windows/macOS/Linux environments.

## Observed first viability slice

The first executable Slint viability slice is now green in CI without changing the Office product workspace or its Rust 1.85 safety/quality gates.

Qualified source/build evidence:

- Ubuntu 24.04: format, `cargo check --all-targets`, tests and pedantic clippy with `-D warnings` pass;
- macOS: `cargo check --all-targets` and tests pass;
- Windows: `cargo check --all-targets` and tests pass;
- the ordinary Office Rust 1.85 architecture/fmt/check/test/clippy job remains green on the same PR head;
- the full pinned LibreOffice native compatibility/render/identity/restart/invalidation qualification remains green on the same PR head.

Qualified Linux native-window evidence under Xvfb and `SLINT_BACKEND=winit-software`:

```text
1×:
ui_framework=slint-1.17.1
ui_backend=winit-software
ui_accessibility=enabled
ui_scale_factor=1
ui_physical_size=1100x800
ui_tile_bytes=262144
ui_tile_checksum=6744427103266065219

2×:
ui_framework=slint-1.17.1
ui_backend=winit-software
ui_accessibility=enabled
ui_scale_factor=2
ui_physical_size=2200x1600
ui_tile_bytes=262144
ui_tile_checksum=6744427103266065219
```

This establishes that Slint is **viable enough to continue qualifying** for the Office shell: the editor-shaped candidate compiles across the three target desktop OSes, the real Linux Winit/software path creates a native window at both forced DPI scales, and the host-owned raster payload survives unchanged.

It does **not** select Slint. In particular, this CI evidence does not prove production-quality IME behavior, real screen-reader behavior, native file-dialog/clipboard/drag-drop integration, large-viewport interaction latency, multi-monitor behavior or long-term maintenance/licensing suitability.

## Accessibility caution

Slint 1.17 has substantially expanded accessibility support, including landmark roles and accessible IDs, and its accessibility feature integrates with operating-system APIs through AccessKit. That is promising but not treated as proof that every accessibility edge is solved.

Current upstream issues include macOS VoiceOver behavior for some roles and virtualized accessibility focus. Office should therefore require a focused accessibility fixture before final selection, including:

- document canvas/main landmark discovery;
- search field role/name/value;
- menu and shortcut discoverability;
- status/live announcements;
- keyboard-only focus traversal;
- large/virtualized UI behavior;
- stable automation identifiers.

## Licensing caution

Slint 1.17.1 is offered under GPLv3, the Slint Royalty-Free Desktop/Mobile/Web Applications License, or a paid Slint Software License. The royalty-free route allows proprietary desktop distribution but requires Slint attribution, for example through the specified About widget or public-web attribution route.

Licensing is not a technical blocker, but an accepted Slint decision must record which licence route Office will use and satisfy its attribution/distribution obligations. Do not let a toolkit choice make the product licence ambiguous.

## Acceptance gate

Do **not** add Slint (or another candidate) to `apps/desktop` until all of the following are true:

- candidate compiles on the three desktop platforms we support;
- native window creation and 1×/2× DPI behavior are qualified;
- the editor-shaped raster composition path is viable;
- IME/text input is exercised on real platforms;
- screen-reader/accessibility tree behavior is exercised on real platforms;
- menus, shortcuts, clipboard and file-dialog strategy are explicit;
- packaging/licensing obligations are understood;
- performance of viewport composition/resizing/scrolling is measured sufficiently to reject obvious jank;
- ADR-0005 is superseded by an explicit selection or explicit continuation of the no-selection state.

The first three items now have positive Slint evidence. The remaining items are selection evidence, not reasons to weaken the architecture boundary in advance.

## Sources checked for this qualification

Primary/current references checked on 2026-09-06 include:

- Slint 1.17.1 crate metadata and feature documentation;
- Slint language/reference documentation for LineEdit, ScrollView, MenuBar and accessibility properties;
- Slint 1.17 source for Winit scale-factor handling and compiler constant-scale handling;
- Slint 1.17 release notes and licensing terms;
- egui/eframe accessibility documentation and current changelog;
- Linebender Xilem README/releases;
- upstream GPUI README and current accessibility work;
- current Iced accessibility status.

The executable results in this repository take precedence over marketing claims or ecosystem reputation.
