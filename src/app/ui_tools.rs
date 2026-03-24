use super::state::CopaibaApp;

impl CopaibaApp {
    pub fn show_tools_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("tools_panel")
            .resizable(true)
            .default_width(180.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().id_salt("tools_scroll").show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.heading("🛠️ Presets");
                    ui.separator();

                    let display_presets = self.presets.clone();
                    for (i, preset) in display_presets.iter().enumerate() {
                        let shortcut = format!("Ctrl+{}", i + 1);
                        if ui.button(format!("{} ({shortcut})", preset.name)).clicked() {
                            let idx = {
                                let tab = self.cur();
                                tab.filtered.get(tab.selected).copied()
                            };
                            if let Some(idx) = idx {
                                self.save_undo_state();
                                let tab = self.cur_mut();
                                if let Some(entry) = tab.entries.get_mut(idx) {
                                    entry.consonant = preset.consonant;
                                    entry.cutoff = preset.cutoff;
                                    entry.preutter = preset.preutter;
                                    entry.overlap = preset.overlap;
                                    tab.dirty = true;
                                }
                            }
                        }
                    }

                    ui.add_space(4.0);
                    if ui.button("Editar Presets...").clicked() { self.ui.show_preset_editor = true; }

                    ui.add_space(20.0);
                    ui.heading("🕹️ Modos de Edição");
                    ui.separator();
                    {
                        let tab = self.cur_mut();
                        ui.checkbox(&mut tab.wave_view.srp, "SRP (Shift+1)");
                        ui.label(egui::RichText::new("Move tudo relativo à Preutt").small());
                        ui.add_space(8.0);
                        ui.checkbox(&mut tab.wave_view.srna, "SRnA (Shift+2)");
                        ui.label(egui::RichText::new("Fixa marcadores ao mover Offset").small());
                        ui.add_space(8.0);
                        ui.checkbox(&mut tab.wave_view.snap_to_peaks, "Auto-oto [Canário]");
                        ui.label(egui::RichText::new("Atrai marcadores para picos").small());
                    }
                    ui.add_space(8.0);
                    ui.checkbox(&mut self.visual.persistent_zoom, "Zoom Persistente");
                    ui.label(egui::RichText::new("Não reseta zoom ao trocar alias").small());

                    ui.add_space(20.0);
                    ui.heading("📊 Status");
                    ui.separator();
                    {
                        let tab = self.cur();
                        ui.label(format!("Aliases: {}", tab.entries.len()));
                        ui.label(format!("Filtrados: {}", tab.filtered.len()));
                    }
                });
            });
    }
}
