pub fn test_egui_data(ctx: &egui::Context) {
    ctx.memory_mut(|mem| {
        // Iterate or something
        let data = &mut mem.data;
        // See if there's extract_by_type
        let values = data.extract_by_type::<crate::markdown::editor::ImageLoadResult>();
    });
}
