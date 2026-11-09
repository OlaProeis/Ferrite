//! Per-frame queue of rendered block commits for logical undo granularity.
//!
//! Session keystrokes stay in `BlockEditState` buffers; when a block closes or switches,
//! `commit_session_block` mutates tab source. We snapshot **immediately before** that
//! mutation (via a pre-commit clone) because `MarkdownEditor::show` only holds
//! `&mut String`, not `&mut Tab`.
//!
//! After `show` returns, `central_panel` drains the queue through
//! [`crate::state::Tab::apply_rendered_commit_undo_entries`].

use eframe::egui;

/// One logical undo step for a rendered block commit (or table flush).
#[derive(Debug, Clone)]
pub struct PendingRenderedCommitUndo {
    /// Document content immediately before the commit wrote to source.
    pub pre_commit_snapshot: String,
    /// Document content immediately after this commit (may differ from final tab content
    /// when multiple commits queue in one frame).
    pub post_commit_snapshot: String,
    /// When true, [`crate::state::Tab::break_undo_group`] runs before recording this entry.
    pub break_group_before: bool,
}

#[derive(Debug, Default, Clone)]
struct RenderedCommitUndoFrame {
    break_before_next: bool,
    /// When true, [`crate::state::Tab::break_undo_group`] runs after commit undo is applied.
    break_after_commit: bool,
    pending: Vec<PendingRenderedCommitUndo>,
}

fn frame_id() -> egui::Id {
    egui::Id::new("ferrite_rendered_commit_undo_frame")
}

/// Reset the per-frame commit queue at the start of rendered `MarkdownEditor::show`.
pub fn begin_frame(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        d.insert_temp(frame_id(), RenderedCommitUndoFrame::default());
    });
}

/// Call before [`crate::markdown::rendered_session::RenderedEditSession::switch_to_ui`]
/// so the upcoming commit of the previous block starts a fresh undo group.
pub fn mark_break_before_next_commit(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        let frame = d.get_temp_mut_or_default::<RenderedCommitUndoFrame>(frame_id());
        frame.break_before_next = true;
    });
}

/// Call when a rendered code block edit session commits (Done / focus-out).
pub fn mark_break_after_commit(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        let frame = d.get_temp_mut_or_default::<RenderedCommitUndoFrame>(frame_id());
        frame.break_after_commit = true;
    });
}

/// Whether a code-block commit requested an undo-group break this frame.
pub fn take_break_after_commit(ctx: &egui::Context) -> bool {
    ctx.data_mut(|d| {
        let frame = d.get_temp_mut_or_default::<RenderedCommitUndoFrame>(frame_id());
        std::mem::take(&mut frame.break_after_commit)
    })
}

/// Snapshot source, run the commit mutation, enqueue one undo entry if content changed.
pub fn record_source_commit<F>(ctx: &egui::Context, source: &mut String, mut apply: F)
where
    F: FnMut(&mut String),
{
    let pre = source.clone();
    apply(source);
    if pre == *source {
        return;
    }
    let post = source.clone();
    ctx.data_mut(|d| {
        let frame = d.get_temp_mut_or_default::<RenderedCommitUndoFrame>(frame_id());
        let break_group_before = frame.break_before_next;
        frame.break_before_next = false;
        frame.pending.push(PendingRenderedCommitUndo {
            pre_commit_snapshot: pre,
            post_commit_snapshot: post,
            break_group_before,
        });
    });
}

/// Drain queued commits for this frame (empty if rendered editor did not run).
pub fn take_pending_commits(ctx: &egui::Context) -> Vec<PendingRenderedCommitUndo> {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<RenderedCommitUndoFrame>(frame_id())
            .pending
            .drain(..)
            .collect()
    })
}

/// Whether any commit-boundary undo entries were queued this frame.
pub fn had_commits(ctx: &egui::Context) -> bool {
    ctx.data(|d| {
        d.get_temp::<RenderedCommitUndoFrame>(frame_id())
            .is_some_and(|f| !f.pending.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_source_commit_enqueues_on_change() {
        let ctx = egui::Context::default();
        begin_frame(&ctx);
        let mut source = "hello".to_string();
        record_source_commit(&ctx, &mut source, |s| s.push_str(" world"));
        let pending = take_pending_commits(&ctx);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].pre_commit_snapshot, "hello");
        assert_eq!(pending[0].post_commit_snapshot, "hello world");
        assert!(!pending[0].break_group_before);
        assert_eq!(source, "hello world");
    }

    #[test]
    fn record_source_commit_skips_no_op() {
        let ctx = egui::Context::default();
        begin_frame(&ctx);
        let mut source = "same".to_string();
        record_source_commit(&ctx, &mut source, |_s| {});
        assert!(take_pending_commits(&ctx).is_empty());
    }

    #[test]
    fn break_flag_applies_to_next_commit_only() {
        let ctx = egui::Context::default();
        begin_frame(&ctx);
        mark_break_before_next_commit(&ctx);
        let mut source = "a".to_string();
        record_source_commit(&ctx, &mut source, |s| *s = "b".to_string());
        record_source_commit(&ctx, &mut source, |s| *s = "c".to_string());
        let pending = take_pending_commits(&ctx);
        assert_eq!(pending.len(), 2);
        assert!(pending[0].break_group_before);
        assert!(!pending[1].break_group_before);
    }
}
