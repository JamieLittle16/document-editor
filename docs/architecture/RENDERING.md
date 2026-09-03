# Rendering and Viewport

## Principle

The UI owns the viewport. The document engine supplies renderable content and geometry asynchronously.

## Bootstrap pipeline

```text
viewport -> required regions -> priority scheduler -> worker render requests
         <- cached tiles/regions <- responses tagged with document revision
```

Priority classes:

1. currently visible;
2. immediate scroll neighbourhood;
3. predictive prefetch;
4. thumbnails/background.

## UI-owned overlays

Where correctness permits, the shell should own lightweight responsive overlays such as:

- caret;
- selection affordances;
- spelling/grammar underlines;
- search highlights;
- comment/review indicators;
- remote cursors in future collaboration.

They must be derived from geometry valid for the rendered revision.

## Cache rules

- cache memory is bounded;
- keys include enough document/layout revision information to prevent stale reuse;
- stale tiles can remain briefly as visual fallback during scrolling but are never treated as authoritative hit-test geometry;
- cache policy is benchmarked on large documents.

## Native future

A future engine may provide display lists or GPU-oriented primitives rather than raster tiles. The viewport contract should allow this without rewriting application semantics.
