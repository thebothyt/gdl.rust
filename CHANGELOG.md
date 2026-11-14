# Changelog

## Unreleased

### Added
- 45 table-driven Geometry Dash 2.2 triggers, including gradient, camera, song/SFX, pickup, count, item edit,
  item compare, timers, collision and random. Generated reference in `docs/TRIGGERS.md`.
- Named arguments for calls: `move(0, 0, 1, 0, 60, 2, easing = EASE_OUT)`, also for user functions.
- Constants for easing, item operators, comparisons, camera, gradient layers and blends.
- Block catalog (`block()`, `rect()`, `B_*` constants), 92 entries; helpers `set_rotation`, `set_scale`, `flip_x`,
  `flip_y`, `set_z`, `set_color`, `set_detail_color`.
- `.gdlrust` project format; `gdl new`, `gdl convert` (both directions) and `gdl compile`; `gdl run` accepts `.gdlrust`.
- `group()`/`color()` skip ids already used by a project's imported objects.
- Web editor: open/save/compile `.gdlrust`, unsaved-changes warning, shapes for spikes, slabs, orbs and pads,
  labels for the new triggers.
- Documentation overhaul, examples, contributor guide and CI.

### Changed
- `move`, `alpha`, `toggle`, `pulse`, `color_trigger`, `spawn`, `rotate`, `shake` and `stop` now come from the trigger
  table. Existing calls produce the same output; each gained optional named settings.
