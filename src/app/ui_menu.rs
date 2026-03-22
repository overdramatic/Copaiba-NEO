use egui::RichText;
use super::state::CopaibaApp;

impl CopaibaApp {
    pub fn show_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Arquivo", |ui| {
                    if ui.button("📁 Abrir Voicebank...\tCtrl+O").clicked() { self.open_voicebank_dir(); ui.close_menu(); }
                    if ui.button("📄 Abrir oto.ini...\tCtrl+O").clicked() { self.open_oto(); ui.close_menu(); }
                    ui.separator();
                    let save_enabled = self.cur().dirty || self.cur().oto_path.is_some();
                    if ui.add_enabled(save_enabled, egui::Button::new("💾 Salvar\tCtrl+S")).clicked() { self.save_oto(); ui.close_menu(); }
                    if ui.button("💾 Salvar como...\tCtrl+Shift+S").clicked() { self.save_as(); ui.close_menu(); }
                    ui.separator();
                    if ui.button("📂 Abrir pasta no explorer\tCtrl+P").clicked() {
                        if let Some(ref d) = self.cur().oto_dir {
                            #[cfg(target_os = "windows")] let _ = std::process::Command::new("explorer").arg(d).spawn();
                            #[cfg(target_os = "linux")] let _ = std::process::Command::new("xdg-open").arg(d).spawn();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("🚪 Sair").clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                });

                ui.menu_button("Editar", |ui| {
                    if ui.button("↩ Desfazer\tCtrl+Z").clicked() { self.undo(ctx); ui.close_menu(); }
                    if ui.button("↪ Refazer\tCtrl+Y").clicked() { self.redo(ctx); ui.close_menu(); }
                    ui.separator();

                    let tab = self.cur_mut();
                    let mut snap = tab.wave_view.snap_to_peaks;
                    if ui.checkbox(&mut snap, "🪄 Auto-oto [Canário]")
                        .on_hover_text("Ajusta automaticamente os limites baseando-se nos picos de energia")
                        .changed()
                    {
                        tab.wave_view.snap_to_peaks = snap;
                    }

                    ui.add_space(8.0);
                    ui.menu_button("📍 Modo de snap", |ui| {
                        let tab = self.cur_mut();
                        if ui.button("SRP - Snap Relativo de Preutterance\tShift+1").clicked() {
                            tab.wave_view.srp = !tab.wave_view.srp;
                            if tab.wave_view.srp { tab.wave_view.srna = false; }
                            ui.close_menu();
                        }
                        if ui.button("SRnA - Snap Relativo a Nada\tShift+2").clicked() {
                            tab.wave_view.srna = !tab.wave_view.srna;
                            if tab.wave_view.srna { tab.wave_view.srp = false; }
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    if ui.button("📝 Renomear alias...\tCtrl+R").clicked() { self.is_renaming = true; ui.close_menu(); }
                    if ui.button("🗑️ Deletar alias\tCtrl+D").clicked() { ui.close_menu(); }
                });

                ui.menu_button("Visualizar", |ui| {
                    ui.checkbox(&mut self.show_spectrogram, "Espectrograma");
                    ui.checkbox(&mut self.show_minimap, "Minimapa");
                    ui.separator();
                    ui.checkbox(&mut self.play_on_select, "Tocar ao selecionar");
                    ui.checkbox(&mut self.auto_scroll_to_selected, "Auto-scroll para selecionado");
                    ui.separator();
                    if ui.button("Redefinir visualizações").clicked() {
                        let tab = self.cur_mut();
                        tab.wave_view.scroll_accum = 0.0;
                        tab.wave_view.mouse_ms = None;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Reprodução", |ui| {
                    if ui.button("▶ Tocar segmento (Espaço)\tSpace").clicked() { self.play_current_segment(false); ui.close_menu(); }
                    if ui.button("▶ Tocar áudio completo\tShift+Space").clicked() { self.play_current_segment(true); ui.close_menu(); }
                    ui.separator();
                    if ui.button("🧪 Teste de Síntese\tCtrl+Shift+Space").clicked() { self.resample_current(); ui.close_menu(); }
                    ui.separator();
                    ui.checkbox(&mut self.play_on_select, "Tocar setor ao clicar");
                });

                ui.menu_button("Configurações", |ui| {
                    if ui.button("⚙ Configurações Gerais...\tCtrl+,").clicked() { self.show_settings = true; ui.close_menu(); }
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.auto_save_enabled, "Auto-salvar");
                        if self.auto_save_enabled {
                            ui.add(egui::DragValue::new(&mut self.auto_save_interval_mins).suffix(" min").range(1..=60));
                        }
                    });
                });

                ui.menu_button("Plugins", |ui| {
                    if ui.button("🔍 Verificador de Consistência").clicked() { self.show_consistency_checker = true; ui.close_menu(); }
                    if ui.button("✂ Detector de Duplicatas").clicked() { self.show_duplicate_detector = true; ui.close_menu(); }
                    if ui.button("🎵 Análise de Pitch").clicked() { self.show_pitch_analyzer = true; ui.close_menu(); }
                    ui.separator();
                    if ui.button("↕ Ordenar Aliases...").clicked() { self.show_alias_sorter = true; ui.close_menu(); }
                    ui.separator();
                    if ui.button("📝 Renomear em Massa (Enxertia)").clicked() { self.show_batch_rename = true; ui.close_menu(); }
                    if ui.button("📊 Edição em Lote").clicked() { self.show_batch_edit = true; ui.close_menu(); }
                });

                ui.menu_button("Ajuda", |ui| {
                    if ui.button("⌨ Atalhos de Teclado (F1)\tF1").clicked() { self.show_help = true; ui.close_menu(); }
                });

                ui.separator();
                if ui.button(RichText::new("Regravar áudio (F9)").color(egui::Color32::from_rgb(100, 200, 100)).strong()).clicked() {
                    self.show_recorder = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("NEO").color(egui::Color32::from_rgb(140, 100, 200)).strong());
                    ui.label(RichText::new("Copaiba").strong());
                });
            });
        });
    }
}
