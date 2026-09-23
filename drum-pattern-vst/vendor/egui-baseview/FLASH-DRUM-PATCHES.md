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
