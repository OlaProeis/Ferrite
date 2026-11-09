//! Pure title-bar drag / restore state machine (ISS-54 / GH #153).
//!
//! Linux X11 window managers often leave the client area at maximised size after
//! `Maximized(false)` unless pre-maximise `OuterPosition` / `InnerSize` are
//! re-applied. Drag-from-maximised must un-maximise one frame before `StartDrag`.

use eframe::egui;

/// Geometry captured while the window is in its normal (non-maximised) state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreMaximizeGeometry {
    pub outer_position: egui::Pos2,
    pub inner_size: egui::Vec2,
}

/// Viewport side-effects emitted by [`title_bar_transition`].
#[derive(Clone, Debug, PartialEq)]
pub enum TitleBarViewportCmd {
    Maximized(bool),
    OuterPosition(egui::Pos2),
    InnerSize(egui::Vec2),
    StartDrag,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TitleBarDragState {
    #[default]
    Idle,
    /// Un-maximise was sent; wait one frame before `StartDrag`.
    PendingUnmaximise,
    /// WM owns the drag loop until the pointer is released.
    Dragging,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleBarEvent {
    /// Title-bar double-click while the window is maximised (restore).
    DoubleClickRestore,
    /// Pointer moved past the drag threshold while maximised.
    DragStartWhileMaximized,
    /// Frame tick while in [`TitleBarDragState::PendingUnmaximise`].
    PendingUnmaximiseFrame,
    /// Primary button released after a maximised drag-out.
    DragFinished,
}

/// Compute the next drag state and viewport commands for one title-bar input.
pub fn title_bar_transition(
    state: TitleBarDragState,
    event: TitleBarEvent,
    stored: Option<PreMaximizeGeometry>,
) -> (TitleBarDragState, Vec<TitleBarViewportCmd>) {
    match (state, event) {
        (TitleBarDragState::Idle, TitleBarEvent::DoubleClickRestore) => {
            let mut cmds = vec![TitleBarViewportCmd::Maximized(false)];
            if let Some(g) = stored {
                cmds.push(TitleBarViewportCmd::OuterPosition(g.outer_position));
                cmds.push(TitleBarViewportCmd::InnerSize(g.inner_size));
            }
            (TitleBarDragState::Idle, cmds)
        }
        (TitleBarDragState::Idle, TitleBarEvent::DragStartWhileMaximized) => (
            TitleBarDragState::PendingUnmaximise,
            vec![TitleBarViewportCmd::Maximized(false)],
        ),
        (TitleBarDragState::PendingUnmaximise, TitleBarEvent::PendingUnmaximiseFrame) => (
            TitleBarDragState::Dragging,
            vec![TitleBarViewportCmd::StartDrag],
        ),
        (TitleBarDragState::Dragging, TitleBarEvent::DragFinished) => {
            (TitleBarDragState::Idle, Vec::new())
        }
        (current, _) => (current, Vec::new()),
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn apply_viewport_cmds(ctx: &egui::Context, cmds: &[TitleBarViewportCmd]) {
    for cmd in cmds {
        match cmd {
            TitleBarViewportCmd::Maximized(v) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(*v));
            }
            TitleBarViewportCmd::OuterPosition(p) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(*p));
            }
            TitleBarViewportCmd::InnerSize(s) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(*s));
            }
            TitleBarViewportCmd::StartDrag => {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GEOM: PreMaximizeGeometry = PreMaximizeGeometry {
        outer_position: egui::pos2(120.0, 80.0),
        inner_size: egui::vec2(960.0, 640.0),
    };

    #[test]
    fn drag_start_while_maximised_enters_pending_and_unmaximises() {
        let (state, cmds) = title_bar_transition(
            TitleBarDragState::Idle,
            TitleBarEvent::DragStartWhileMaximized,
            Some(GEOM),
        );
        assert_eq!(state, TitleBarDragState::PendingUnmaximise);
        assert_eq!(cmds, vec![TitleBarViewportCmd::Maximized(false)]);
    }

    #[test]
    fn pending_frame_starts_drag() {
        let (state, cmds) = title_bar_transition(
            TitleBarDragState::PendingUnmaximise,
            TitleBarEvent::PendingUnmaximiseFrame,
            Some(GEOM),
        );
        assert_eq!(state, TitleBarDragState::Dragging);
        assert_eq!(cmds, vec![TitleBarViewportCmd::StartDrag]);
    }

    #[test]
    fn double_click_restore_reapplies_stored_geometry() {
        let (state, cmds) = title_bar_transition(
            TitleBarDragState::Idle,
            TitleBarEvent::DoubleClickRestore,
            Some(GEOM),
        );
        assert_eq!(state, TitleBarDragState::Idle);
        assert_eq!(
            cmds,
            vec![
                TitleBarViewportCmd::Maximized(false),
                TitleBarViewportCmd::OuterPosition(GEOM.outer_position),
                TitleBarViewportCmd::InnerSize(GEOM.inner_size),
            ]
        );
    }

    #[test]
    fn double_click_restore_without_stored_geometry_only_unmaximises() {
        let (state, cmds) = title_bar_transition(
            TitleBarDragState::Idle,
            TitleBarEvent::DoubleClickRestore,
            None,
        );
        assert_eq!(state, TitleBarDragState::Idle);
        assert_eq!(cmds, vec![TitleBarViewportCmd::Maximized(false)]);
    }

    #[test]
    fn drag_finished_returns_to_idle() {
        let (state, cmds) = title_bar_transition(
            TitleBarDragState::Dragging,
            TitleBarEvent::DragFinished,
            Some(GEOM),
        );
        assert_eq!(state, TitleBarDragState::Idle);
        assert!(cmds.is_empty());
    }
}
