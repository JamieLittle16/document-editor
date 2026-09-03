# ADR-0005: Do not freeze the GUI framework before an R0A qualification spike

- Status: Accepted
- Date: 2026-09-03

## Context

Slint is currently a strong practical candidate; GPUI and Linebender stacks are architecturally interesting. GUI selection affects accessibility, platform integration, rendering, licensing and long-term maintenance.

## Decision

Keep application/domain logic UI-agnostic and run a short qualification spike before selecting the production GUI framework.

## Qualification criteria

- Windows/Linux/macOS quality;
- accessibility APIs;
- IME/international text input;
- menus/clipboard/drag-drop/file dialogs;
- high-DPI/multi-monitor behaviour;
- rendering latency and large viewport composition;
- packaging/update implications;
- licence/attribution obligations;
- maintenance trajectory.

## Consequence

The architecture may favour Slint, but no foundational crate may depend on Slint until the spike is accepted.
