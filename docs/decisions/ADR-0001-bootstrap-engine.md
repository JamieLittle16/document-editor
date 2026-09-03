# ADR-0001: Use LibreOffice Writer as a quarantined bootstrap engine

- Status: Accepted
- Date: 2026-09-03

## Context

Word-grade DOCX layout and round-trip behaviour is an enormous accumulated engineering problem. Building it before producing a useful editor would delay product validation by years.

## Decision

Use LibreOffice Writer/LibreOfficeKit as the initial production compatibility/layout engine, but only behind our engine adapter and worker boundary.

## Consequences

Positive:
- mature DOCX/Writer functionality immediately;
- focus bespoke work on product UX, language, reliability and architecture;
- realistic path to a useful product quickly.

Negative:
- historical C++ architecture and global/serialized behaviour;
- API adaptation complexity;
- memory/process overhead;
- fidelity still differs from Word in some cases.

## Constraints

- LO types never cross the engine adapter;
- LO does not run on the UI thread;
- upstream patch stack stays minimal;
- product architecture must support replacing LO.
