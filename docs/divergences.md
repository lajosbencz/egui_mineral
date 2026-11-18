# Divergences

Deliberate departures from the MD3 spec, Compose behavior or our own [conventions](conventions.md).

| Area | Divergence | Reason |
|---|---|---|
| Sizing of `CheckButton`, `RadioButton`, `Switch` | Never measure below the control (box, icon, 52x32dp track), against the [sizing convention](conventions.md) that `layout` honors tight constraints. A narrower row overflows. | The spec sizes do not shrink, a squeezed control would paint over its neighbours. |
| `SegmentedButton::equal_width(false)` | Segments size to their content instead of the widest one (Compose `weight(1f)`). Opt-in, equal is the default. | Dense toolbars: equal widths waste room beside short labels. |
| `TreeView` paddings | Start, end and slot paddings (MD3 list: 16dp) and the default indent narrow 4dp per density step, to 4dp at `Density::MIN`. | Desktop trees in narrow columns, MD3 list paddings ignore density. |
| `SegmentedButton` press | No z-index raise on press, segments paint in row order. | egui paints in call order. |
| `SegmentedButton` check | The check's scale spring keeps running while hidden, a quick reselect starts part-way. | One spring per segment. |
| `SegmentedButton` overflow | A row wider than the available width shrinks its segments evenly and clips their labels. | Compose has no defined overflow. |
