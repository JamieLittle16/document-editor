# UI Shell and Command Architecture

## Product shape

The document occupies the centre. Optional left/right panels provide outline/search/pages and inspector/proofing/comments respectively. Panels disappear when not useful.

The product should be immediately legible to Word/ONLYOFFICE users without cloning the ribbon mechanically.

## Command registry

Every action has one canonical command identity, for example:

```text
file.save
format.bold
paragraph.style.heading_2
insert.table
review.comment
navigation.goto_heading
proofing.next_issue
```

Menus, toolbars, keyboard shortcuts, command palette, plugins and AI automation invoke the same command validation/execution path.

## Command state

Commands may expose:

- enabled/disabled;
- checked/mixed;
- contextual label/icon;
- shortcut;
- reason disabled where useful.

Command state is derived from application/session state and must not require a blocking engine call during UI interaction.

## Style inspector

For a selection, the UI should clearly distinguish:

- effective style;
- inheritance source;
- local/direct overrides;
- conflicting mixed state.

Users can remove overrides, update a style from selection, create a style and find inconsistent/direct formatting.

## UI framework qualification

Before freezing a framework, qualify:

- accessibility;
- IME and international text input;
- Windows/macOS/Linux integration;
- drag/drop, clipboard and menus;
- high-DPI and multi-monitor behaviour;
- document viewport composition performance;
- packaging/update story;
- licensing/attribution;
- project maintenance trajectory.
