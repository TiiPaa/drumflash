# Flash Drum adapter patch

Upstream: BillyDM/egui-baseview, revision
`ec70c3fe6b2f070dcacbc22924431edbe24bd1c0` (same revision previously used).
Source and MIT license are kept with this copy.

[243]: forward baseview's existing file-drag events to egui and return
`AcceptDrop(Copy)` for an accepted destination. No second `RegisterDragDrop`
registration is needed: baseview already owns and cleans up the native target.
Drop state belongs to each editor, including the exact drop position; it is
discarded when the window closes. Windows coordinates in this pinned baseview
revision need conversion from screen to client coordinates before egui scaling.

[243] lane-flash fix (`window.rs`): after a `do_repaint_now` render, keep the
`repaint_delay` egui just requested instead of clearing `repaint_after`.
Upstream discarded it, so a time-based effect whose last rendered frame still
showed a sliver of the effect (the [238] lane-name flash) never got the frame
that turns it fully off — the lit pixels stayed on screen until an unrelated
repaint.

[273] rejected-hover reporting (`file_drop.rs`): a drag the editor refuses
(target disabled, outside the rectangle, wrong file type) reports its hover
position through `rejected_hover_position()` until it leaves or is accepted.
The grid uses it to say WHY next to the bare OS ⊘ cursor (full grid, open
modal). Covered by `rejected_drag_reports_its_position_until_it_leaves`.
