//! ISS-59: parity between [`super::text::EguiTextMeasurer`] and painted egui galleys.

#[cfg(test)]
mod tests {
    use egui::{Color32, FontId, Vec2};

    use crate::config::CjkFontPreference;
    use crate::fonts::{self, FontSelection};
    use crate::markdown::mermaid::flowchart::{layout_flowchart, parse_flowchart};
    use crate::markdown::mermaid::text::{EguiTextMeasurer, EstimatedTextMeasurer, TextMeasurer};

    /// GrokBot FC-165a pure-CJK node label (ISS-59).
    const CJK_ISS59_LABEL: &str = "堡垒机运维系统";
    const LATIN_LABEL: &str = "Hello";
    const FONT_SIZE: f32 = 14.0;
    /// Task 17 / prd-v0.3.1-mermaid-fidelity.md ISS-59: measurer vs paint galley.
    const MAX_MEASURER_TO_PAINT_RATIO: f32 = 1.1;

    fn run_ui<R>(ctx: &egui::Context, mut f: impl FnMut(&mut egui::Ui) -> R) -> R {
        let mut out = None;
        let mut raw = egui::RawInput::default();
        raw.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            Vec2::new(800.0, 600.0),
        ));
        let _ = ctx.run_ui(raw, |ctx| {
            egui::CentralPanel::default().show_inside(ctx, |ui| {
                out = Some(f(ui));
            });
        });
        out.expect("egui frame")
    }

    fn width_triple(ui: &mut egui::Ui, text: &str, font_size: f32) -> (f32, f32, f32) {
        let font_id = FontId::proportional(font_size);
        let measurer_w = EguiTextMeasurer::new(ui).measure(text, font_size).width;
        let fonts_galley_w = ui.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(text.to_owned(), font_id.clone(), Color32::PLACEHOLDER)
                .size()
                .x
        });
        let painter_galley_w = ui
            .painter()
            .layout_no_wrap(text.to_owned(), font_id, Color32::PLACEHOLDER)
            .size()
            .x;
        (measurer_w, fonts_galley_w, painter_galley_w)
    }

    fn assert_width_ratio(name: &str, measurer_w: f32, reference_w: f32) {
        assert!(
            reference_w > 0.0,
            "{name}: reference galley width must be positive"
        );
        let ratio = measurer_w / reference_w;
        assert!(
            ratio <= MAX_MEASURER_TO_PAINT_RATIO && ratio >= 1.0 / MAX_MEASURER_TO_PAINT_RATIO,
            "{name}: measurer/paint width ratio {ratio:.4} (measurer={measurer_w:.2}, \
             paint={reference_w:.2}); expected within {MAX_MEASURER_TO_PAINT_RATIO} (ISS-59)"
        );
    }

    fn setup_ctx_with_cjk_for_label(label: &str) -> egui::Context {
        let ctx = egui::Context::default();
        fonts::setup_fonts_lazy(&ctx);
        // Frame 1: request lazy CJK load (fonts apply on the next frame).
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            fonts::load_cjk_for_text(
                label,
                ctx,
                &FontSelection::default(),
                CjkFontPreference::Auto,
                None,
            );
        });
        ctx
    }

    #[test]
    fn test_measure_cjk_label_width_matches_painted_galley() {
        let ctx = setup_ctx_with_cjk_for_label(CJK_ISS59_LABEL);
        let (measurer_w, _fonts_w, painter_w) =
            run_ui(&ctx, |ui| width_triple(ui, CJK_ISS59_LABEL, FONT_SIZE));
        assert_width_ratio("CJK ISS-59 label", measurer_w, painter_w);
    }

    #[test]
    fn test_measure_latin_label_width_matches_painted_galley() {
        let ctx = egui::Context::default();
        fonts::setup_fonts_lazy(&ctx);
        let (measurer_w, _fonts_w, painter_w) =
            run_ui(&ctx, |ui| width_triple(ui, LATIN_LABEL, FONT_SIZE));
        assert_width_ratio("Latin label", measurer_w, painter_w);
    }

    #[test]
    fn test_measure_flowchart_node_width_tracks_measurer_not_byte_estimate() {
        let ctx = setup_ctx_with_cjk_for_label(CJK_ISS59_LABEL);
        let source = format!("flowchart TD\n  A[{CJK_ISS59_LABEL}]");
        let flowchart = parse_flowchart(&source).unwrap();
        let node_id = flowchart.nodes[0].id.clone();

        let egui_text_w = run_ui(&ctx, |ui| {
            EguiTextMeasurer::new(ui)
                .measure(CJK_ISS59_LABEL, FONT_SIZE)
                .width
        });

        let layout_w = run_ui(&ctx, |ui| {
            let layout = layout_flowchart(&flowchart, 800.0, FONT_SIZE, &EguiTextMeasurer::new(ui));
            layout.nodes.get(&node_id).unwrap().size.x
        });

        let estimated_layout_w = {
            let layout =
                layout_flowchart(&flowchart, 800.0, FONT_SIZE, &EstimatedTextMeasurer::new());
            layout.nodes.get(&node_id).unwrap().size.x
        };

        // Layout uses text_width_factor 1.15 and horizontal padding 24*2.
        let expected_min = egui_text_w * 1.15 + 48.0;
        assert!(
            layout_w >= expected_min - 0.5,
            "node box {layout_w:.1} should be at least measurer-based width {expected_min:.1}"
        );

        // Byte-length estimate must not drive layout when using EguiTextMeasurer.
        assert!(
            estimated_layout_w > layout_w * 1.2,
            "EstimatedTextMeasurer layout {estimated_layout_w:.1} should be much wider than \
             real-font layout {layout_w:.1} for pure CJK (ISS-59 cache/estimate guard)"
        );
    }
}
