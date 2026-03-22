use std::sync::Arc;
use std::sync::atomic::Ordering;
use egui::RichText;
use egui_plot::{Plot, Line, PlotPoints};

use crate::audio::WavData;
use super::state::CopaibaApp;
use crate::app::recorder;

impl CopaibaApp {
    pub fn modal_recorder(&mut self, ctx: &egui::Context) {
        if !self.show_recorder { return; }

        let mut open = true;
        egui::Window::new("Regravar Alias")
            .open(&mut open)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                let tab = self.cur();
                if let Some(&idx) = tab.filtered.get(tab.selected) {
                    let entry = tab.entries[idx].clone();
                    ui.horizontal(|ui| {
                        ui.heading(format!("Regravando: {}", entry.alias));
                        ui.label(RichText::new(format!(" ({})", entry.filename)).weak());
                    });
                    ui.separator();

                    if self.is_recording {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label(RichText::new("🔴 Gravando...").color(egui::Color32::RED).size(24.0));
                            ui.add_space(20.0);
                            if ui.button(RichText::new("⏹ Parar Gravação").size(18.0)).clicked() {
                                self.stop_recording();
                            }
                        });
                    } else if let Some(recorded) = self.recorded_wav.clone() {
                        ui.label("Gravação concluída. Revise abaixo:");
                        
                        // Mini waveform preview
                        let points: PlotPoints = recorded.samples.iter().enumerate()
                            .step_by(10) // downsample for preview
                            .map(|(i, &s)| [i as f64, s as f64])
                            .collect();
                        
                        Plot::new("recorder_plot")
                            .height(120.0)
                            .show_axes([false, false])
                            .allow_drag(false)
                            .allow_zoom(false)
                            .allow_boxed_zoom(false)
                            .allow_double_click_reset(false)
                            .show(ui, |plot_ui| {
                                plot_ui.line(Line::new(points).color(egui::Color32::from_rgb(100, 255, 100)));
                            });

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("▶ Escutar").clicked() {
                                self.play_wav_data(recorded);
                            }
                            if ui.button("⏺ Regravar").clicked() {
                                self.recorded_wav = None;
                                self.start_recording();
                            }
                        });

                        ui.add_space(16.0);
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("✅ Substituir Gravação Atual").color(egui::Color32::GOLD)).clicked() {
                                if let Err(e) = self.save_recorded_wav(&entry.filename) {
                                    self.status = format!("Erro ao salvar: {}", e);
                                } else {
                                    self.status = format!("Gravado com sucesso: {}", entry.alias);
                                    self.show_recorder = false;
                                    self.recorded_wav = None;
                                }
                            }
                            if ui.button("❌ Descartar").clicked() {
                                self.recorded_wav = None;
                                self.show_recorder = false;
                            }
                        });
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            if ui.button(RichText::new("⏺ Iniciar Gravação").size(24.0)).clicked() {
                                self.start_recording();
                            }
                            ui.add_space(40.0);
                        });
                    }
                } else {
                    ui.label("Nenhum alias selecionado na tabela.");
                    if ui.button("Fechar").clicked() { self.show_recorder = false; }
                }
            });

        if !open {
            if self.is_recording { self.stop_recording(); }
            self.show_recorder = false;
        }
    }

    fn start_recording(&mut self) {
        self.recorder_samples.lock().unwrap().clear();
        self.recorder_stop_signal.store(false, Ordering::SeqCst);
        match recorder::start_recording(self.recorder_samples.clone(), self.recorder_stop_signal.clone()) {
            Ok((stream, rate)) => {
                self.recorder_stream = Some(stream);
                self.recorder_sample_rate = rate;
                self.is_recording = true;
                self.status = format!("Gravando a {} Hz...", rate);
            }
            Err(e) => {
                self.status = format!("Erro ao iniciar gravação: {}", e);
            }
        }
    }

    fn stop_recording(&mut self) {
        if self.is_recording {
            self.recorder_stop_signal.store(true, Ordering::SeqCst);
            self.recorder_stream = None; // Drop stream
            self.is_recording = false;

            let samples = self.recorder_samples.lock().unwrap().clone();
            if !samples.is_empty() {
                self.recorded_wav = Some(WavData {
                    samples: Arc::new(samples),
                    sample_rate: self.recorder_sample_rate,
                    duration_ms: (self.recorder_samples.lock().unwrap().len() as f64 / self.recorder_sample_rate as f64) * 1000.0,
                });
            }
        }
    }

    pub fn save_recorded_wav(&mut self, filename: &str) -> Result<(), String> {
        if let Some(wav) = self.recorded_wav.clone() {
            let path = {
                let tab = self.cur();
                if let Some(dir) = &tab.oto_dir { dir.join(filename) } else { std::path::PathBuf::from(filename) }
            };

            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: wav.sample_rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };

            let mut writer = hound::WavWriter::create(&path, spec).map_err(|e| e.to_string())?;
            for &sample in wav.samples.iter() {
                let amplitude = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                writer.write_sample(amplitude).map_err(|e| e.to_string())?;
            }
            writer.finalize().map_err(|e| e.to_string())?;

            // Update caches
            self.wav_cache.insert(filename.to_string(), wav.clone());
            self.spec_data_cache.remove(filename);
            
            // Invalidate waveform view cache for this tab
            self.cur_mut().wave_view.spec_cache = crate::waveform::SpecCache::default();
            self.cur_mut().wave_view.wave_cache = crate::waveform::WaveCache::default();
            
            Ok(())
        } else {
            Err("Nenhuma gravação para salvar".to_string())
        }
    }
}
