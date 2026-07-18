#![allow(dead_code)] // UI integration continues in follow-up tasks (Phase 1+).

//! Rendered edit session coordinator — single owner of active WYSIWYG block state.
//!
//! See [`docs/technical/markdown/rendered-edit-session.md`](../../docs/technical/markdown/rendered-edit-session.md)
//! for architecture, commit policy, and regression matrix. PRD:
//! `docs/ai-workflow/prds/prd-rendered-edit-session.md`.

use std::collections::HashMap;

use eframe::egui::{Context, Id, Ui};

/// Stable block identity keyed by source line (1-indexed) and block kind.
///
/// Stable egui widget id suffixes for rendered block TextEdits (see `widget_id_in_scope`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockRef {
    Heading {
        line: usize,
        structural: bool,
    },
    Paragraph {
        line: usize,
    },
    ListItem {
        line: usize,
        item: u32,
    },
    FormattedParagraph {
        line: usize,
        structural: bool,
    },
    FormattedListItem {
        line: usize,
        item: u32,
        structural: bool,
    },
    TableCell {
        table_line: usize,
        row: usize,
        col: usize,
    },
}

impl BlockRef {
    /// egui widget id under the current rendered editor scope.
    pub fn widget_id(self, ui: &Ui) -> Id {
        self.widget_id_in_scope(ui.id())
    }

    /// Widget id under a known scope (`ui.id()` inside `push_id(editor_id)` + `push_id(source_epoch)`).
    pub fn widget_id_in_scope(self, scope_id: Id) -> Id {
        match self {
            BlockRef::Heading { line, structural } => {
                let key = if structural {
                    "heading_text_sk"
                } else {
                    "heading_text"
                };
                scope_id.with(key).with(line)
            }
            BlockRef::Paragraph { line } => scope_id.with("para_text").with(line),
            BlockRef::ListItem { line, .. } => scope_id.with("list_item_text").with(line),
            BlockRef::FormattedParagraph { line, structural } => {
                let key = if structural {
                    "formatted_paragraph_sk"
                } else {
                    "formatted_paragraph"
                };
                scope_id.with(key).with(line).with("text_edit")
            }
            BlockRef::FormattedListItem {
                line,
                item,
                structural,
            } => {
                let key = if structural {
                    "formatted_list_item_sk"
                } else {
                    "formatted_list_item"
                };
                scope_id.with(key).with(line).with(item).with("text_edit")
            }
            BlockRef::TableCell {
                table_line,
                row,
                col,
            } => scope_id
                .with("table")
                .with(table_line)
                .with("cell")
                .with(row)
                .with(col),
        }
    }
}

/// One-shot focus/cursor request applied on the next frame by the block widget.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PendingActivation {
    pub cursor_char_index: Option<usize>,
    pub request_focus: bool,
}

/// Per-block edit buffer and mode flags.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockEditState {
    /// TextEdit buffer (raw markdown for formatted blocks).
    pub text: String,
    /// Formatted blocks: false = styled display, true = raw TextEdit.
    pub formatted_editing: bool,
    pub dirty: bool,
    pub pending_activation: Option<PendingActivation>,
}

/// Whether closing the active block writes its buffer to source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitPolicy {
    SaveIfDirty,
    Discard,
}

/// Single coordinator for rendered-mode block editing within one tab/editor instance.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderedEditSession {
    pub active: Option<BlockRef>,
    pub blocks: HashMap<BlockRef, BlockEditState>,
}

impl RenderedEditSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Switch active block: close previous (per policy), open `block`, queue activation.
    ///
    /// Does not surrender egui focus — use [`Self::switch_to_ui`] at render boundaries.
    pub fn switch_to<F>(
        &mut self,
        block: BlockRef,
        activation: PendingActivation,
        commit_fn: &mut F,
    ) where
        F: FnMut(BlockRef, &BlockEditState),
    {
        if self.active == Some(block) {
            log::trace!("RenderedEditSession: re-activate same block {:?}", block);
            let state = self
                .blocks
                .entry(block)
                .or_insert_with(BlockEditState::default);
            state.pending_activation = Some(activation);
            return;
        }

        log::trace!(
            "RenderedEditSession: switch {:?} -> {:?}",
            self.active,
            block
        );
        self.close_active(CommitPolicy::SaveIfDirty, commit_fn);
        self.active = Some(block);
        let state = self
            .blocks
            .entry(block)
            .or_insert_with(BlockEditState::default);
        state.pending_activation = Some(activation);
    }

    /// Close the active block, optionally committing or discarding its buffer.
    ///
    /// Does not surrender egui focus — use [`Self::close_active_ui`] at render boundaries.
    pub fn close_active<F>(&mut self, policy: CommitPolicy, commit_fn: &mut F)
    where
        F: FnMut(BlockRef, &BlockEditState),
    {
        let Some(active_block) = self.active.take() else {
            return;
        };

        log::trace!(
            "RenderedEditSession: close_active {:?} policy={:?}",
            active_block,
            policy
        );

        if let Some(state) = self.blocks.get_mut(&active_block) {
            if state.formatted_editing {
                state.formatted_editing = false;
            }
            match policy {
                CommitPolicy::SaveIfDirty if state.dirty => {
                    commit_fn(active_block, state);
                    state.dirty = false;
                }
                CommitPolicy::Discard => {
                    state.dirty = false;
                }
                CommitPolicy::SaveIfDirty => {}
            }
        }
    }

    /// Update buffer text without writing source (marks dirty).
    pub fn on_text_changed(&mut self, block: BlockRef, new_text: String) {
        let state = self
            .blocks
            .entry(block)
            .or_insert_with(BlockEditState::default);
        if state.text != new_text {
            log::trace!("RenderedEditSession: text changed block {:?}", block);
        }
        state.text = new_text;
        state.dirty = true;
    }

    /// Force-commit the active block buffer via callback.
    pub fn commit_active<F>(&mut self, mut commit_fn: F)
    where
        F: FnMut(BlockRef, &BlockEditState),
    {
        let Some(block) = self.active else {
            return;
        };
        if let Some(state) = self.blocks.get_mut(&block) {
            if state.dirty {
                log::trace!("RenderedEditSession: commit_active {:?}", block);
                commit_fn(block, state);
                state.dirty = false;
            }
        }
    }

    /// Discard active buffer and reload from source via callback; stay on same block.
    pub fn discard_active<F>(&mut self, mut reload_fn: F)
    where
        F: FnMut(BlockRef, &mut BlockEditState),
    {
        let Some(block) = self.active else {
            return;
        };
        log::trace!("RenderedEditSession: discard_active {:?}", block);
        if let Some(state) = self.blocks.get_mut(&block) {
            reload_fn(block, state);
            state.dirty = false;
            state.formatted_editing = false;
        }
    }

    /// Clear all buffers after external invalidation (`source_epoch` bump).
    pub fn invalidate_buffers(&mut self) {
        log::trace!("RenderedEditSession: invalidate_buffers (clear all)");
        self.active = None;
        self.blocks.clear();
    }

    /// Ensure formatted block exists in display mode (not raw TextEdit).
    pub fn open_formatted_display(&mut self, block: BlockRef) {
        let state = self
            .blocks
            .entry(block)
            .or_insert_with(BlockEditState::default);
        state.formatted_editing = false;
    }
}

impl BlockRef {
    /// Surrender egui focus for this block using the live render `Ui` scope.
    pub fn surrender_focus(self, ui: &Ui) {
        ui.memory_mut(|mem| mem.surrender_focus(self.widget_id(ui)));
    }
}

impl RenderedEditSession {
    /// Close active block and surrender focus using the render-time `Ui` scope.
    pub fn close_active_ui<F>(&mut self, ui: &mut Ui, policy: CommitPolicy, commit_fn: &mut F)
    where
        F: FnMut(BlockRef, &BlockEditState),
    {
        let closing = self.active;
        self.close_active(policy, commit_fn);
        if let Some(block) = closing {
            block.surrender_focus(ui);
        }
    }

    /// Switch active block using render-time `Ui` for focus surrender on the previous block.
    pub fn switch_to_ui<F>(
        &mut self,
        ui: &mut Ui,
        block: BlockRef,
        activation: PendingActivation,
        commit_fn: &mut F,
    ) where
        F: FnMut(BlockRef, &BlockEditState),
    {
        if self.active == Some(block) {
            log::trace!("RenderedEditSession: re-activate same block {:?}", block);
            let state = self
                .blocks
                .entry(block)
                .or_insert_with(BlockEditState::default);
            state.pending_activation = Some(activation);
            return;
        }

        let previous = self.active;
        self.switch_to(block, activation, commit_fn);
        if let Some(prev) = previous {
            prev.surrender_focus(ui);
        }
    }
}

// ── egui temp storage (tab/editor scoped) ───────────────────────────────────

/// Stable rendered-pane editor id for a tab — shared by [`ViewMode::Rendered`] and split preview.
///
/// Both single rendered and split-view right panes must use this id so they share one
/// [`RenderedEditSession`] and the same viewport-culling / widget-id scope in egui temp memory.
pub fn rendered_editor_id(tab_id: usize) -> Id {
    Id::new("main_editor_rendered").with(tab_id)
}

fn session_storage_id(editor_id: Id) -> Id {
    editor_id.with("rendered_edit_session")
}

pub fn load(ui: &Ui, editor_id: Id) -> RenderedEditSession {
    ui.memory(|mem| {
        mem.data
            .get_temp(session_storage_id(editor_id))
            .unwrap_or_default()
    })
}

pub fn save(ui: &mut Ui, editor_id: Id, session: RenderedEditSession) {
    ui.memory_mut(|mem| {
        mem.data.insert_temp(session_storage_id(editor_id), session);
    });
}

fn session_epoch_id(editor_id: Id) -> Id {
    editor_id.with("rendered_edit_session_epoch")
}

/// Load session; clears buffers when `source_epoch` changed (raw edit, external reload).
pub fn load_for_epoch(ui: &Ui, editor_id: Id, source_epoch: u64) -> RenderedEditSession {
    let stored_epoch: u64 = ui.memory(|mem| {
        mem.data
            .get_temp(session_epoch_id(editor_id))
            .unwrap_or(u64::MAX)
    });
    let mut session = load(ui, editor_id);
    if stored_epoch != source_epoch {
        log::trace!(
            "RenderedEditSession: source_epoch mismatch (stored={}, current={}) — \
             invalidating buffers for editor {:?}",
            stored_epoch,
            source_epoch,
            editor_id
        );
        session.invalidate_buffers();
    }
    session
}

/// Persist session and record epoch so the next load can detect external invalidation.
pub fn save_for_epoch(ui: &mut Ui, editor_id: Id, source_epoch: u64, session: RenderedEditSession) {
    save(ui, editor_id, session);
    ui.memory_mut(|mem| {
        mem.data
            .insert_temp(session_epoch_id(editor_id), source_epoch);
    });
}

pub fn load_ctx(ctx: &Context, editor_id: Id) -> RenderedEditSession {
    ctx.data(|d| {
        d.get_temp(session_storage_id(editor_id))
            .unwrap_or_default()
    })
}

pub fn save_ctx(ctx: &Context, editor_id: Id, session: RenderedEditSession) {
    ctx.data_mut(|d| d.insert_temp(session_storage_id(editor_id), session));
}

/// Scope id after `ui.push_id(parent).push_id(editor_id).push_id(source_epoch)`.
pub fn rendered_widget_scope_id(parent_id: Id, editor_id: Id, source_epoch: u64) -> Id {
    parent_id.with(editor_id).with(source_epoch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;

    fn with_ui(mut f: impl FnMut(&mut Ui)) {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show_inside(ctx, |ui| {
                f(ui);
            });
        });
    }

    fn heading(line: usize) -> BlockRef {
        BlockRef::Heading {
            line,
            structural: false,
        }
    }

    fn paragraph(line: usize) -> BlockRef {
        BlockRef::Paragraph { line }
    }

    #[test]
    fn switch_to_dirty_block_commits_on_switch_pure() {
        let mut session = RenderedEditSession::new();
        let mut commits: Vec<(BlockRef, String)> = Vec::new();

        session.on_text_changed(heading(1), "Hello".to_string());
        session.active = Some(heading(1));

        session.switch_to(
            heading(2),
            PendingActivation {
                cursor_char_index: Some(0),
                request_focus: true,
            },
            &mut |block, state| {
                commits.push((block, state.text.clone()));
            },
        );

        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].0, heading(1));
        assert_eq!(session.active, Some(heading(2)));
    }

    #[test]
    fn switch_to_dirty_block_commits_on_switch_ui() {
        let mut session = RenderedEditSession::new();
        let mut commits: Vec<(BlockRef, String)> = Vec::new();

        session.on_text_changed(heading(1), "Hello".to_string());
        session.active = Some(heading(1));

        with_ui(|ui| {
            session.switch_to_ui(
                ui,
                heading(2),
                PendingActivation {
                    cursor_char_index: Some(0),
                    request_focus: true,
                },
                &mut |block, state| {
                    commits.push((block, state.text.clone()));
                },
            );
        });

        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].0, heading(1));
        assert_eq!(commits[0].1, "Hello");
        assert_eq!(session.active, Some(heading(2)));
        assert!(session.blocks.get(&heading(1)).is_some_and(|s| !s.dirty));
    }

    #[test]
    fn switch_to_same_block_updates_pending_activation_only() {
        let mut session = RenderedEditSession::new();
        let mut commit_count = 0u32;

        with_ui(|ui| {
            session.switch_to_ui(
                ui,
                heading(3),
                PendingActivation {
                    cursor_char_index: Some(1),
                    request_focus: true,
                },
                &mut |_block, _state| commit_count += 1,
            );
        });

        with_ui(|ui| {
            session.switch_to_ui(
                ui,
                heading(3),
                PendingActivation {
                    cursor_char_index: Some(5),
                    request_focus: false,
                },
                &mut |_block, _state| commit_count += 1,
            );
        });

        assert_eq!(commit_count, 0);
        assert_eq!(session.active, Some(heading(3)));
        let state = session.blocks.get(&heading(3)).unwrap();
        assert_eq!(
            state.pending_activation,
            Some(PendingActivation {
                cursor_char_index: Some(5),
                request_focus: false,
            })
        );
    }

    #[test]
    fn close_active_discard_clears_active_without_commit() {
        let mut session = RenderedEditSession::new();
        session.on_text_changed(paragraph(2), "draft".to_string());
        session.active = Some(paragraph(2));
        let mut commit_count = 0u32;

        with_ui(|ui| {
            session.close_active_ui(ui, CommitPolicy::Discard, &mut |_block, _state| {
                commit_count += 1
            });
        });

        assert_eq!(commit_count, 0);
        assert_eq!(session.active, None);
        assert!(session.blocks.get(&paragraph(2)).is_some_and(|s| !s.dirty));
    }

    #[test]
    fn at_most_one_active_invariant() {
        let mut session = RenderedEditSession::new();

        with_ui(|ui| {
            session.switch_to_ui(
                ui,
                heading(1),
                PendingActivation::default(),
                &mut |_b, _s| {},
            );
            session.switch_to_ui(
                ui,
                heading(2),
                PendingActivation::default(),
                &mut |_b, _s| {},
            );
            session.switch_to_ui(
                ui,
                paragraph(10),
                PendingActivation::default(),
                &mut |_b, _s| {},
            );
        });

        assert_eq!(session.active, Some(paragraph(10)));
        assert!(session.active.is_some());
        let active_count = session.active.iter().count();
        assert_eq!(active_count, 1);
    }

    #[test]
    fn on_text_changed_marks_dirty() {
        let mut session = RenderedEditSession::new();
        session.on_text_changed(paragraph(4), "x".to_string());
        let state = session.blocks.get(&paragraph(4)).unwrap();
        assert!(state.dirty);
        assert_eq!(state.text, "x");
    }

    #[test]
    fn heading_widget_id_uses_structural_suffix() {
        with_ui(|ui| {
            let block = BlockRef::Heading {
                line: 7,
                structural: true,
            };
            let expected = ui.id().with("heading_text_sk").with(7usize);
            assert_eq!(block.widget_id(ui), expected);
        });
    }

    #[test]
    fn widget_id_in_scope_matches_live_ui_under_epoch_scope() {
        with_ui(|ui| {
            let editor_id = ui.id().with("test_editor");
            let epoch = 3u64;
            ui.push_id(editor_id, |ui| {
                ui.push_id(epoch, |ui| {
                    let block = BlockRef::Heading {
                        line: 2,
                        structural: false,
                    };
                    assert_eq!(block.widget_id(ui), block.widget_id_in_scope(ui.id()));
                });
            });
        });
    }

    #[test]
    fn rendered_editor_id_is_tab_scoped() {
        let id_a = rendered_editor_id(7);
        let id_b = rendered_editor_id(7);
        let id_other = rendered_editor_id(8);
        assert_eq!(id_a, id_b);
        assert_ne!(id_a, id_other);
    }

    #[test]
    fn rendered_editor_id_unifies_single_and_split_view() {
        // Split view previously used a separate id; both panes must share one session.
        let tab_id = 42usize;
        let unified = rendered_editor_id(tab_id);
        let legacy_split = egui::Id::new("split_preview_rendered").with(tab_id);
        assert_ne!(unified, legacy_split);
        assert_eq!(unified, egui::Id::new("main_editor_rendered").with(tab_id));
    }

    #[test]
    fn session_persists_across_rendered_view_mode_switch() {
        with_ui(|ui| {
            let editor_id = rendered_editor_id(5);
            let mut session = RenderedEditSession::new();
            session.active = Some(heading(1));
            session.on_text_changed(heading(1), "draft".to_string());
            save_for_epoch(ui, editor_id, 0, session);

            // Same tab id whether user is in Rendered-only or Split preview pane.
            let loaded = load_for_epoch(ui, rendered_editor_id(5), 0);
            assert_eq!(loaded.active, Some(heading(1)));
            assert_eq!(
                loaded.blocks.get(&heading(1)).map(|s| s.text.as_str()),
                Some("draft")
            );
        });
    }

    #[test]
    fn rs6_raw_edit_epoch_bump_invalidates_session_buffers() {
        with_ui(|ui| {
            let editor_id = rendered_editor_id(10);
            let mut session = RenderedEditSession::new();
            session.active = Some(paragraph(3));
            session.on_text_changed(paragraph(3), "from rendered pane".to_string());
            save_for_epoch(ui, editor_id, 0, session);

            // Simulate raw-pane edit in split view: Tab::record_external_edit_from_snapshot bumps epoch.
            let after_raw_edit = load_for_epoch(ui, editor_id, 1);
            assert_eq!(after_raw_edit.active, None);
            assert!(after_raw_edit.blocks.is_empty());
        });
    }

    #[test]
    fn widget_id_stable_across_content_change_same_epoch() {
        let parent = egui::Id::new("scroll_viewport");
        let editor_id = rendered_editor_id(1);
        let epoch = 0u64;
        let scope = rendered_widget_scope_id(parent, editor_id, epoch);
        let block = BlockRef::Heading {
            line: 1,
            structural: false,
        };
        let id_before = block.widget_id_in_scope(scope);
        // Simulated content edit does not change scope when epoch is unchanged.
        let id_after = block.widget_id_in_scope(scope);
        assert_eq!(id_before, id_after);
    }

    #[test]
    fn widget_id_changes_when_source_epoch_bumps() {
        let parent = egui::Id::new("scroll_viewport");
        let editor_id = rendered_editor_id(1);
        let block = BlockRef::Heading {
            line: 1,
            structural: false,
        };
        let id_epoch_0 = block.widget_id_in_scope(rendered_widget_scope_id(parent, editor_id, 0));
        let id_epoch_1 = block.widget_id_in_scope(rendered_widget_scope_id(parent, editor_id, 1));
        assert_ne!(id_epoch_0, id_epoch_1);
    }

    #[test]
    fn widget_id_differs_from_content_hash_scope() {
        let parent = egui::Id::new("scroll_viewport");
        let editor_id = rendered_editor_id(1);
        let content_hash = 0xDEAD_BEEF_u64;
        let block = BlockRef::Heading {
            line: 1,
            structural: false,
        };
        let epoch_scope = block.widget_id_in_scope(rendered_widget_scope_id(parent, editor_id, 0));
        let hash_scope = block.widget_id_in_scope(parent.with(content_hash));
        assert_ne!(epoch_scope, hash_scope);
    }

    #[test]
    fn session_storage_roundtrip() {
        with_ui(|ui| {
            let editor_id = ui.id().with("test_editor");
            let mut session = RenderedEditSession::new();
            session.active = Some(heading(1));
            save(ui, editor_id, session.clone());
            let loaded = load(ui, editor_id);
            assert_eq!(loaded, session);
        });
    }

    #[test]
    fn invalidate_buffers_clears_state() {
        let mut session = RenderedEditSession::new();
        session.active = Some(heading(1));
        session.on_text_changed(heading(1), "a".to_string());
        session.invalidate_buffers();
        assert_eq!(session.active, None);
        assert!(session.blocks.is_empty());
    }

    #[test]
    fn load_for_epoch_invalidates_when_epoch_changes() {
        with_ui(|ui| {
            let editor_id = ui.id().with("epoch_test_editor");
            let mut session = RenderedEditSession::new();
            session.active = Some(paragraph(1));
            session.on_text_changed(paragraph(1), "draft".to_string());
            save_for_epoch(ui, editor_id, 0, session);

            let loaded = load_for_epoch(ui, editor_id, 0);
            assert_eq!(loaded.active, Some(paragraph(1)));

            let reloaded = load_for_epoch(ui, editor_id, 1);
            assert_eq!(reloaded.active, None);
            assert!(reloaded.blocks.is_empty());
        });
    }

    fn formatted_paragraph(line: usize) -> BlockRef {
        BlockRef::FormattedParagraph {
            line,
            structural: false,
        }
    }

    fn formatted_list_item(line: usize, item: u32) -> BlockRef {
        BlockRef::FormattedListItem {
            line,
            item,
            structural: false,
        }
    }

    #[test]
    fn formatted_editing_flag_resets_on_close() {
        let mut session = RenderedEditSession::new();
        let block = formatted_paragraph(2);
        session.on_text_changed(block, "draft".to_string());
        session.active = Some(block);
        if let Some(state) = session.blocks.get_mut(&block) {
            state.formatted_editing = true;
        }

        let mut commits: Vec<BlockRef> = Vec::new();
        session.close_active(CommitPolicy::SaveIfDirty, &mut |block, _state| {
            commits.push(block);
        });

        assert_eq!(commits, vec![block]);
        assert_eq!(session.active, None);
        let state = session.blocks.get(&block).expect("state retained");
        assert!(!state.formatted_editing);
        assert!(!state.dirty);
    }

    #[test]
    fn formatted_switch_commits_previous_and_resets_editing_flag() {
        let mut session = RenderedEditSession::new();
        let a = formatted_paragraph(1);
        let b = formatted_list_item(5, 0);
        session.on_text_changed(a, "**bold** text".to_string());
        session.active = Some(a);
        if let Some(state) = session.blocks.get_mut(&a) {
            state.formatted_editing = true;
        }

        let mut commits: Vec<(BlockRef, String)> = Vec::new();
        with_ui(|ui| {
            session.switch_to_ui(
                ui,
                b,
                PendingActivation {
                    cursor_char_index: Some(3),
                    request_focus: true,
                },
                &mut |block, state| commits.push((block, state.text.clone())),
            );
        });

        assert_eq!(commits.len(), 1, "previous active block committed");
        assert_eq!(commits[0].0, a);
        assert_eq!(commits[0].1, "**bold** text");
        assert_eq!(session.active, Some(b));
        assert!(
            !session.blocks.get(&a).unwrap().formatted_editing,
            "previous formatted_editing flag cleared on close",
        );
    }

    #[test]
    fn formatted_discard_reload_resets_flags_via_reload_fn() {
        let mut session = RenderedEditSession::new();
        let block = formatted_paragraph(7);
        session.on_text_changed(block, "edited draft".to_string());
        session.active = Some(block);
        if let Some(state) = session.blocks.get_mut(&block) {
            state.formatted_editing = true;
        }

        session.discard_active(|_block, state| {
            state.text = "original source".to_string();
        });

        let state = session.blocks.get(&block).expect("state retained");
        assert_eq!(state.text, "original source");
        assert!(!state.dirty);
        assert!(!state.formatted_editing);
        // discard_active intentionally leaves `active` set; caller clears + surrenders focus.
        assert_eq!(session.active, Some(block));
    }

    #[test]
    fn formatted_widget_ids_remain_stable_with_legacy_keys() {
        with_ui(|ui| {
            let fp = formatted_paragraph(3);
            let fp_sk = BlockRef::FormattedParagraph {
                line: 3,
                structural: true,
            };
            let fli = formatted_list_item(9, 2);
            let fli_sk = BlockRef::FormattedListItem {
                line: 9,
                item: 2,
                structural: true,
            };

            let expected_fp = ui
                .id()
                .with("formatted_paragraph")
                .with(3usize)
                .with("text_edit");
            let expected_fp_sk = ui
                .id()
                .with("formatted_paragraph_sk")
                .with(3usize)
                .with("text_edit");
            let expected_fli = ui
                .id()
                .with("formatted_list_item")
                .with(9usize)
                .with(2u32)
                .with("text_edit");
            let expected_fli_sk = ui
                .id()
                .with("formatted_list_item_sk")
                .with(9usize)
                .with(2u32)
                .with("text_edit");

            assert_eq!(fp.widget_id(ui), expected_fp);
            assert_eq!(fp_sk.widget_id(ui), expected_fp_sk);
            assert_eq!(fli.widget_id(ui), expected_fli);
            assert_eq!(fli_sk.widget_id(ui), expected_fli_sk);
        });
    }

    #[test]
    fn paragraph_and_list_widget_id_suffixes() {
        with_ui(|ui| {
            let para = BlockRef::Paragraph { line: 5 };
            assert_eq!(para.widget_id(ui), ui.id().with("para_text").with(5usize));

            let item = BlockRef::ListItem { line: 9, item: 2 };
            assert_eq!(
                item.widget_id(ui),
                ui.id().with("list_item_text").with(9usize)
            );
        });
    }

    // ─── TableCell session integration (Task 101) ────────────────────────────

    fn table_cell(table_line: usize, row: usize, col: usize) -> BlockRef {
        BlockRef::TableCell {
            table_line,
            row,
            col,
        }
    }

    /// Cross-block enter: switching from a heading to a TableCell must fire
    /// commit_fn for the heading (its buffer is dirty) so the source updates.
    #[test]
    fn switch_from_heading_to_table_cell_commits_heading() {
        let mut session = RenderedEditSession::new();
        let h = heading(2);
        session.on_text_changed(h, "Header text".to_string());
        session.active = Some(h);

        let mut commits: Vec<(BlockRef, String)> = Vec::new();
        with_ui(|ui| {
            session.switch_to_ui(
                ui,
                table_cell(10, 0, 0),
                PendingActivation::default(),
                &mut |block, state| commits.push((block, state.text.clone())),
            );
        });

        assert_eq!(commits.len(), 1, "previous heading must commit on switch");
        assert_eq!(commits[0].0, h);
        assert_eq!(session.active, Some(table_cell(10, 0, 0)));
    }

    /// Leaving a TableCell for a heading must fire commit_fn for the TableCell.
    /// The cell's BlockEditState text stays empty (cell content lives in the widget,
    /// not the session) — what matters is that the callback runs so the editor can
    /// write the force-commit signal.
    #[test]
    fn switch_from_table_cell_to_heading_invokes_commit_for_cell() {
        let mut session = RenderedEditSession::new();
        let cell = table_cell(7, 1, 2);
        // Simulate the editor setting the cell as active (no dirty buffer — text
        // belongs to EditableTable, but commit_fn must still see the TableCell
        // block_ref so it can signal the table to flush).
        session.active = Some(cell);
        // Mark dirty so close_active actually calls commit_fn (CommitPolicy::SaveIfDirty).
        if let Some(s) = session.blocks.get_mut(&cell) {
            s.dirty = true;
        } else {
            session.blocks.insert(
                cell,
                BlockEditState {
                    dirty: true,
                    ..Default::default()
                },
            );
        }

        let mut committed_blocks: Vec<BlockRef> = Vec::new();
        with_ui(|ui| {
            session.switch_to_ui(
                ui,
                heading(4),
                PendingActivation::default(),
                &mut |block, _state| committed_blocks.push(block),
            );
        });

        assert_eq!(committed_blocks, vec![cell]);
        assert_eq!(session.active, Some(heading(4)));
    }

    /// Direct assignment of session.active for intra-table movement must NOT
    /// trigger commit_fn (which would write the force-commit signal). This
    /// preserves the EditableTable's deferred-commit semantics while Tabbing
    /// between cells.
    #[test]
    fn intra_table_direct_assign_does_not_fire_commit() {
        let mut session = RenderedEditSession::new();
        let a = table_cell(3, 0, 0);
        let b = table_cell(3, 0, 1);
        session.active = Some(a);
        session.blocks.insert(
            a,
            BlockEditState {
                dirty: true,
                ..Default::default()
            },
        );

        // Direct assign, as `sync_table_cell_session_active` does for intra-table moves.
        session.active = Some(b);

        // Verify dirty flag preserved (we did not commit through the session API).
        assert!(session.blocks.get(&a).unwrap().dirty);
        assert_eq!(session.active, Some(b));
    }

    /// Switching between cells of different tables via switch_to_ui must fire
    /// commit_fn for the source TableCell so the previous table receives the
    /// force-commit signal.
    #[test]
    fn cross_table_switch_invokes_commit_for_source_cell() {
        let mut session = RenderedEditSession::new();
        let table_a = table_cell(2, 0, 0);
        let table_b = table_cell(20, 0, 0);
        session.active = Some(table_a);
        session.blocks.insert(
            table_a,
            BlockEditState {
                dirty: true,
                ..Default::default()
            },
        );

        let mut committed: Vec<BlockRef> = Vec::new();
        with_ui(|ui| {
            session.switch_to_ui(
                ui,
                table_b,
                PendingActivation::default(),
                &mut |block, _state| committed.push(block),
            );
        });

        assert_eq!(committed, vec![table_a]);
        assert_eq!(session.active, Some(table_b));
    }

    /// epoch invalidation must clear any active TableCell so a hash-driven source
    /// reload does not leave stale session state pointing at a now-mismatched table.
    #[test]
    fn invalidate_buffers_clears_active_table_cell() {
        let mut session = RenderedEditSession::new();
        let cell = table_cell(11, 2, 3);
        session.active = Some(cell);
        session.blocks.insert(cell, BlockEditState::default());

        session.invalidate_buffers();

        assert_eq!(session.active, None);
        assert!(session.blocks.is_empty());
    }
}
