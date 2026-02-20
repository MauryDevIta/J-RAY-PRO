use crate::app::{JRayPro, DiffStatus, LicenseTier};
use eframe::egui;
use similar::{ChangeTag, TextDiff};

impl eframe::App for JRayPro {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        // --- 🛑 MODALITÀ EXPIRED (BLOCCO TOTALE E CENTRATO) ---
        // Deve stare in cima! Se scatta, disegna e usa il "return" per bloccare il resto.
        if self.license_tier == LicenseTier::Expired {
            // Sfondo nero puro per coprire tutto
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.painter().rect_filled(ctx.screen_rect(), 0.0, egui::Color32::from_rgb(12, 12, 15));
                
                // Finestra Modale Assoluta
                egui::Window::new("🔐 ATTIVAZIONE RICHIESTA")
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .collapsible(false)
                    .resizable(false)
                    .movable(false)
                    .show(ctx, |ui| {
                        ui.add_space(10.0);
                        ui.vertical_centered(|ui| {
                            ui.heading(egui::RichText::new("IL PERIODO DI PROVA È TERMINATO").size(22.0).color(egui::Color32::from_rgb(255, 80, 80)));
                            ui.add_space(15.0);
                            ui.label(egui::RichText::new("Sblocca la licenza Lifetime per continuare a usare il motore Nitro.").size(15.0).color(egui::Color32::WHITE));
                            ui.add_space(25.0);
                            
                            // Tasto di Acquisto
                            if ui.button(egui::RichText::new("🛒 ACQUISTA J-RAY PRO").size(20.0).strong()).clicked() {
                                let _ = open::that("https://jraypro.com/#pricing");
                            }
                            
                            ui.add_space(20.0);
                            ui.separator();
                            ui.add_space(15.0);
                            
                            ui.label(egui::RichText::new("Hai già una licenza? Inseriscila qui:").color(egui::Color32::LIGHT_GRAY));
                            ui.add_space(5.0);
                            ui.add(egui::TextEdit::singleline(&mut self.license_key).hint_text("AAAA-BBBB-CCCC-DDDD").desired_width(280.0));
                            ui.add_space(10.0);
                            
                            if ui.button(egui::RichText::new("🚀 Attiva Software").size(16.0)).clicked() {
                                println!("Richiesta attivazione per la chiave: {}", self.license_key);
                            }
                            
                            ui.add_space(20.0);
                            ui.label(egui::RichText::new(format!("ID Dispositivo: {}", self.machine_id)).small().color(egui::Color32::from_gray(100)));
                        });
                        ui.add_space(10.0);
                    });
            });
            
            // FONDAMENTALE: Ferma l'esecuzione qui. Niente grafo, niente menu!
            return; 
        }
        // --- FINE MODALITÀ EXPIRED ---


        // 👇 DA QUI INIZIA IL PROGRAMMA NORMALE (Se non sei Expired) 👇

        // ✨ RADAR: LOOP DI RETE ASINCRONO
        if self.is_api_live {
            let now = std::time::Instant::now();
            let should_fetch = match self.last_api_fetch {
                None => true,
                Some(last) => now.duration_since(last).as_secs_f32() >= self.api_interval,
            };

            if should_fetch {
                self.last_api_fetch = Some(now);
                let url = self.api_url.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                self.api_receiver = Some(rx);

                std::thread::spawn(move || {
                    if let Ok(response) = reqwest::blocking::get(&url) {
                        if let Ok(text) = response.text() {
                            let _ = tx.send(text);
                        }
                    }
                });
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(200)); 
        }

        // ✨ RADAR: RICEZIONE DATI E AUTO-AGGIORNAMENTO
        if let Some(rx) = &self.api_receiver {
            if let Ok(new_json) = rx.try_recv() {
                
                if self.is_diff_mode {
                    self.json_input_b = self.json_input.clone();
                    self.raw_full_json_b = self.raw_full_json.clone();
                    
                    self.json_input = new_json.clone();
                    self.raw_full_json = Some(new_json.clone());
                    
                    self.run_diff(); 
                    self.status_msg = format!("📡 Live Diff aggiornato alle {}", chrono::Local::now().format("%H:%M:%S"));
                } else {
                    self.json_input = new_json.clone();
                    self.raw_full_json = Some(new_json.clone());
                    self.active_tab = 0;
                    self.generate_graph_from_string(&new_json);
                    self.status_msg = format!("📡 Radar: Ricevuti dati alle {}", chrono::Local::now().format("%H:%M:%S"));
                }
            }
        }

        // --- CARICAMENTO SMART STACK ---
        if self.loading_state == 2 {
            if let Some(path) = self.pending_path.take() {
                let current_limit = self.array_limits.get(&path).copied().unwrap_or(5);
                self.array_limits.insert(path, current_limit + 50);

                let text_to_parse = if self.is_diff_mode {
                    None
                } else {
                    let text = if self.active_tab == 0 {
                        if self.json_input.starts_with("/* ⚠️") && self.raw_full_json.is_some() { self.raw_full_json.as_ref().unwrap().clone() } else { self.json_input.clone() }
                    } else {
                        if self.json_input_b.starts_with("/* ⚠️") && self.raw_full_json_b.is_some() { self.raw_full_json_b.as_ref().unwrap().clone() } else { self.json_input_b.clone() }
                    };
                    Some(text)
                };

                if self.is_diff_mode {
                    self.run_diff(); 
                } else if let Some(text) = text_to_parse {
                    self.generate_graph_from_string(&text); 
                }
            }
            self.loading_state = 0; 
        }

        // --- TOP PANEL MENU ---
        if !self.is_zen_mode {
            egui::TopBottomPanel::top("menu").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("J-RAY PRO").strong().color(egui::Color32::from_rgb(99, 102, 241)));
                    
                    // Info Licenza in Alto
                    if self.license_tier == LicenseTier::Trial {
                        ui.label(egui::RichText::new(format!("(Trial: {}gg)", self.trial_days_left)).color(egui::Color32::from_rgb(234, 179, 8)));
                    } else if self.license_tier == LicenseTier::Pro {
                        ui.label(egui::RichText::new("(PRO)").color(egui::Color32::from_rgb(236, 72, 153)));
                    }

                    ui.separator();
                    
                    if ui.button("📂 File A").clicked() { self.open_file(false); }
                    if ui.button("📂 File B").clicked() { self.open_file(true); }
                    ui.add_enabled(!self.is_huge_file, egui::Button::new("💾 Salva")).clicked().then(|| { self.save_file(); });

                    ui.separator();
                    let diff_lbl = if self.is_diff_mode { "❌ Chiudi Diff" } else { "⚖️ Visual Diff" };
                    let diff_col = if self.is_diff_mode { egui::Color32::from_rgb(239, 68, 68) } else { egui::Color32::YELLOW };
                    
                    if ui.button(egui::RichText::new(diff_lbl).color(diff_col)).clicked() { 
                        if self.is_diff_mode {
                            let text = if self.active_tab == 0 {
                                if self.json_input.starts_with("/* ⚠️") && self.raw_full_json.is_some() { self.raw_full_json.as_ref().unwrap().clone() } else { self.json_input.clone() }
                            } else {
                                if self.json_input_b.starts_with("/* ⚠️") && self.raw_full_json_b.is_some() { self.raw_full_json_b.as_ref().unwrap().clone() } else { self.json_input_b.clone() }
                            };
                            self.generate_graph_from_string(&text);
                            self.status_msg = "Diff chiuso. Grafo ripristinato.".to_string();
                        } else {
                            self.run_diff(); 
                        }
                    }
                    
                    ui.separator();
                    
                    ui.label("📡 API:");
                    ui.add(egui::TextEdit::singleline(&mut self.api_url).hint_text("https://...").desired_width(150.0));
                    ui.add(egui::Slider::new(&mut self.api_interval, 0.5..=10.0).text("sec"));                    
                    let live_btn_text = if self.is_api_live { "🛑 Stop" } else { "▶ LIVE" };
                    let live_btn_color = if self.is_api_live { egui::Color32::RED } else { egui::Color32::from_rgb(34, 197, 94) }; 
                    
                    if ui.button(egui::RichText::new(live_btn_text).color(live_btn_color)).clicked() {
                        self.is_api_live = !self.is_api_live;
                        if self.is_api_live { self.last_api_fetch = None; } 
                    }

                    ui.separator();
                    if ui.button(egui::RichText::new("📊 Profiler").color(egui::Color32::from_rgb(34, 211, 238))).on_hover_text("Rileva Anomalie Dati (AI)").clicked() { 
                        self.run_profiler(); 
                    }

                    ui.separator();
                    if ui.button("📸 Esporta SVG").clicked() { self.export_to_svg(); }
                    if ui.button("🧬 Code Gen").clicked() { self.generate_types(); self.show_code_gen = true; }

                    ui.add_enabled(!self.is_huge_file, egui::Button::new("🚀 Genera Grafo")).clicked().then(|| {
                        if !self.is_huge_file { 
                            let text = if self.active_tab == 0 { self.json_input.clone() } else { self.json_input_b.clone() };
                            self.generate_graph_from_string(&text); 
                        }
                    });
                    
                    ui.separator();
                    ui.label("🔍");
                    let view_center = ctx.available_rect().center();
                    let s_resp = ui.add(egui::TextEdit::singleline(&mut self.search_query).hint_text("Cerca o $.jsonPath...").desired_width(120.0));
                    if s_resp.changed() { self.apply_search(view_center); }
                    if s_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { self.next_search_match(view_center); }

                    if !self.search_results_idx.is_empty() {
                        ui.label(egui::RichText::new(format!("{}/{}", self.current_search_match + 1, self.search_results_idx.len())).color(egui::Color32::LIGHT_GRAY).monospace());
                        if ui.button("⬆").clicked() { self.prev_search_match(view_center); }
                        if ui.button("⬇").clicked() { self.next_search_match(view_center); }
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(&self.status_msg).color(egui::Color32::LIGHT_BLUE));
                        if self.is_diff_mode {
                            ui.label(egui::RichText::new("■ Mod").color(egui::Color32::from_rgb(234, 179, 8)));
                            ui.label(egui::RichText::new("■ Rim").color(egui::Color32::from_rgb(239, 68, 68)));
                            ui.label(egui::RichText::new("■ Agg").color(egui::Color32::from_rgb(34, 197, 94)));
                        }
                    });
                });
            });

            // --- SIDE PANEL EDITOR ---
            egui::SidePanel::left("editor").width_range(300.0..=600.0).show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Editor");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_enabled(!self.is_huge_file && !self.is_diff_mode, egui::Button::new("✨ Format")).clicked().then(|| { self.format_json(); });
                            ui.add_enabled(!self.is_huge_file && !self.nodes.is_empty() && !self.is_diff_mode, egui::Button::new("🔄 Sync Grafo")).clicked().then(|| { self.sync_graph_to_json(); });
                        });
                    });

                    if self.is_diff_mode {
                        ui.separator();
                        ui.label(egui::RichText::new("👀 Comparazione Testo").color(egui::Color32::YELLOW).strong());
                        ui.separator();
                        if self.is_huge_file {
                            ui.label(egui::RichText::new("⚠️ TESTO TROPPO GRANDE PER IL DIFF INLINE").color(egui::Color32::RED));
                            ui.label("Guarda il Grafo 3D a destra. I nodi sono stati compressi con lo Smart Stack per prestazioni estreme.");
                        } else {
                            egui::ScrollArea::both().show(ui, |ui| {
                                let diff = TextDiff::from_lines(&self.json_input, &self.json_input_b);
                                let mut job = egui::text::LayoutJob::default();
                                for change in diff.iter_all_changes() {
                                    let (color, bg, sign) = match change.tag() {
                                        ChangeTag::Delete => (egui::Color32::from_rgb(255, 120, 120), egui::Color32::from_black_alpha(100), "- "),
                                        ChangeTag::Insert => (egui::Color32::from_rgb(120, 255, 120), egui::Color32::from_black_alpha(100), "+ "),
                                        ChangeTag::Equal => (egui::Color32::LIGHT_GRAY, egui::Color32::TRANSPARENT, "  "),
                                    };
                                    let font = egui::FontId::monospace(12.0);
                                    job.append(sign, 0.0, egui::text::TextFormat { font_id: font.clone(), color, background: bg, ..Default::default() });
                                    job.append(change.value(), 0.0, egui::text::TextFormat { font_id: font, color, background: bg, ..Default::default() });
                                }
                                ui.label(job);
                            });
                        }
                    } else {
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.active_tab, 0, "📄 File A");
                            ui.selectable_value(&mut self.active_tab, 1, "📄 File B");
                        });
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let tc = if self.is_huge_file { egui::Color32::from_rgb(250, 200, 100) } else { egui::Color32::LIGHT_GRAY };
                            if self.active_tab == 0 {
                                ui.add(egui::TextEdit::multiline(&mut self.json_input).font(egui::TextStyle::Monospace).text_color(tc).interactive(!self.is_huge_file).desired_width(f32::INFINITY));
                            } else {
                                ui.add(egui::TextEdit::multiline(&mut self.json_input_b).font(egui::TextStyle::Monospace).text_color(tc).interactive(!self.is_huge_file).desired_width(f32::INFINITY));
                            }
                        });
                    }
                });
            });
        }

        // --- FINESTRE MODALI / UTILITIES ---
        let mut show_prof = self.show_profiler;
        if show_prof {
            egui::Window::new("📊 AI Data Profiler & Anomaly Detector")
                .open(&mut show_prof)
                .default_size(egui::vec2(600.0, 400.0))
                .vscroll(true)
                .show(ctx, |ui| {
                    ui.heading("Report Integrità Dataset");
                    ui.separator();
                    ui.add_space(10.0);
                    
                    for rep in &self.profiler_reports {
                        if rep.starts_with("⚠️") {
                            ui.label(egui::RichText::new(rep).color(egui::Color32::from_rgb(234, 179, 8)).strong().size(14.0));
                        } else if rep.starts_with("📉") {
                            ui.label(egui::RichText::new(rep).color(egui::Color32::from_rgb(239, 68, 68)).size(13.0));
                        } else {
                            ui.label(egui::RichText::new(rep).color(egui::Color32::from_rgb(34, 197, 94)).size(14.0));
                        }
                        ui.add_space(8.0);
                    }
                });
        }
        self.show_profiler = show_prof;

        let mut show_window = self.show_code_gen;
        if show_window {
            egui::Window::new("🧬 Type Inference Engine").open(&mut show_window).default_size(egui::vec2(500.0, 600.0)).vscroll(true).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Linguaggio: ");
                    if ui.radio_value(&mut self.code_gen_lang, 0, "TypeScript").changed() { self.generate_types(); }
                    if ui.radio_value(&mut self.code_gen_lang, 1, "Rust").changed() { self.generate_types(); }
                    if ui.radio_value(&mut self.code_gen_lang, 2, "Python (Pydantic)").changed() { self.generate_types(); }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("📋 Copia Codice").clicked() { ctx.output_mut(|o| o.copied_text = self.generated_code.clone()); self.status_msg = "Codice copiato negli appunti!".to_string(); }
                    });
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| { ui.add(egui::TextEdit::multiline(&mut self.generated_code).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).code_editor()); });
            });
        }
        self.show_code_gen = show_window;

        let mut expand_stack_path: Option<String> = None;
        let mut do_decode: Option<String> = None; 

        // --- MOTORE GRAFICO NITRO ---
        egui::CentralPanel::default().show(ctx, |ui| {
            let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());
            let pointer_pos = ctx.input(|i| i.pointer.hover_pos());

            let minimap_size = egui::vec2(240.0, 160.0);
            let minimap_rect = egui::Rect::from_min_size(ui.max_rect().right_bottom() - minimap_size - egui::vec2(20.0, 20.0), minimap_size);
            let mut is_hovering_minimap = false;
            
            if let Some(pos) = pointer_pos { if minimap_rect.contains(pos) { is_hovering_minimap = true; } }

            let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 && !is_hovering_minimap {
                let old_zoom = self.zoom;
                self.zoom = (self.zoom + scroll * 0.002).clamp(0.01, 5.0);
                let zoom_center = pointer_pos.unwrap_or(ui.max_rect().center());
                let world_pos = (zoom_center.to_vec2() - self.pan) / old_zoom;
                self.pan = zoom_center.to_vec2() - world_pos * self.zoom;
            }

            if resp.dragged() && self.dragged_node.is_none() && !is_hovering_minimap { 
                self.pan += resp.drag_delta(); 
            }

            let current_zoom = self.zoom; let current_pan = self.pan;
            let to_screen = move |p: egui::Pos2| egui::pos2(p.x * current_zoom + current_pan.x, p.y * current_zoom + current_pan.y);
            let screen_rect = painter.clip_rect();

            let dot_gap = 40.0 * self.zoom;
            if self.zoom > 0.15 {
                let dot_color = egui::Color32::from_gray(30);
                for x in 0..=(ui.available_width() / dot_gap) as i32 + 1 {
                    for y in 0..=(ui.available_height() / dot_gap) as i32 + 1 { painter.circle_filled(egui::pos2((x as f32 * dot_gap) + (self.pan.x % dot_gap), (y as f32 * dot_gap) + (self.pan.y % dot_gap)), 1.0, dot_color); }
                }
            }

            let edge_stroke = egui::Stroke::new(1.2 * self.zoom, egui::Color32::from_rgb(99, 102, 241));
            for (s_i, e_i) in &self.connections {
                if !self.nodes[*s_i].visible || !self.nodes[*e_i].visible { continue; } 
                let p1 = to_screen(self.nodes[*s_i].pos + egui::vec2(220.0, 32.5));
                let p2 = to_screen(self.nodes[*e_i].pos + egui::vec2(0.0, 32.5));

                if screen_rect.contains(p1) || screen_rect.contains(p2) {
                    if self.zoom > 0.4 {
                        painter.add(egui::epaint::CubicBezierShape::from_points_stroke([p1, p1 + egui::vec2(80.0 * self.zoom, 0.0), p2 - egui::vec2(80.0 * self.zoom, 0.0), p2], false, egui::Color32::TRANSPARENT, edge_stroke));
                    } else { painter.line_segment([p1, p2], edge_stroke); }
                }
            }

            let mut requires_visibility_update = false;

            if ctx.input(|i| i.pointer.any_pressed()) && !is_hovering_minimap {
                if let Some(pos) = pointer_pos {
                    for (idx, node) in self.nodes.iter_mut().enumerate().rev() {
                        if !node.visible { continue; }
                        let rect = egui::Rect::from_min_size(to_screen(node.pos), egui::vec2(220.0, 65.0) * self.zoom);
                        let header_rect = egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + 25.0 * self.zoom));
                        let is_container = &*node.node_type == "OBJ" || &*node.node_type == "ARR" || &*node.node_type == "STACK";
                        let fold_rect = egui::Rect::from_center_size(rect.right_top() + egui::vec2(-15.0, 12.5) * self.zoom, egui::vec2(20.0, 15.0) * self.zoom);

                        if is_container && fold_rect.contains(pos) { 
                            if &*node.node_type == "STACK" {
                                expand_stack_path = Some(node.path.clone()); 
                            } else {
                                node.collapsed = !node.collapsed; requires_visibility_update = true; 
                            }
                            break; 
                        }
                        if header_rect.contains(pos) && !fold_rect.contains(pos) { self.dragged_node = Some(idx); break; }
                    }
                }
            }

            if requires_visibility_update { self.update_visibility(); }
            if ctx.input(|i| i.pointer.any_released()) { self.dragged_node = None; }
            if let Some(idx) = self.dragged_node { self.nodes[idx].pos += resp.drag_delta() / self.zoom; }

            let mut min_w = egui::pos2(f32::MAX, f32::MAX); let mut max_w = egui::pos2(f32::MIN, f32::MIN);
            for n in &self.nodes {
                if !n.visible { continue; }
                min_w.x = min_w.x.min(n.pos.x); min_w.y = min_w.y.min(n.pos.y);
                max_w.x = max_w.x.max(n.pos.x + 220.0); max_w.y = max_w.y.max(n.pos.y + 65.0);
            }
            let world_size = max_w - min_w;
            let radar_scale = (minimap_size.x / world_size.x.max(1.0)).min(minimap_size.y / world_size.y.max(1.0)) * 0.9;
            let radar_offset = minimap_rect.center() - ((min_w.to_vec2() + max_w.to_vec2()) * 0.5 * radar_scale);

            for (idx, node) in self.nodes.iter_mut().enumerate() {
                if !node.visible { continue; }
                if !self.is_zen_mode { painter.circle_filled(radar_offset + node.pos.to_vec2() * radar_scale, 1.5, egui::Color32::from_rgb(100, 100, 180)); }

                let rect = egui::Rect::from_min_size(to_screen(node.pos), egui::vec2(220.0, 65.0) * self.zoom);
                if !screen_rect.intersects(rect) { continue; }

                let is_dragged = self.dragged_node == Some(idx);
                let base_color = if is_dragged { egui::Color32::from_rgb(40, 40, 50) } else if node.matches_search { egui::Color32::from_rgb(24, 24, 27) } else { egui::Color32::from_rgb(15, 15, 18) };
                
                let b_color = if self.is_diff_mode {
                    match node.status {
                        DiffStatus::Added => egui::Color32::from_rgb(34, 197, 94), DiffStatus::Removed => egui::Color32::from_rgb(239, 68, 68),
                        DiffStatus::Modified => egui::Color32::from_rgb(234, 179, 8), DiffStatus::Normal => egui::Color32::from_rgb(99, 102, 241),
                    }
                } else { 
                    if &*node.node_type == "OBJ" { egui::Color32::from_rgb(34, 211, 238) } 
                    else if &*node.node_type == "STACK" { egui::Color32::from_rgb(236, 72, 153) }
                    else { egui::Color32::from_rgb(99, 102, 241) } 
                };

                let stroke_w = if self.is_diff_mode && node.status != DiffStatus::Normal { 2.5 } else { 1.2 };
                let is_currently_focused = !self.search_results_idx.is_empty() && self.search_results_idx[self.current_search_match] == idx;
                
                let final_stroke = if node.matches_search && !self.search_query.is_empty() {
                    if is_currently_focused { egui::Stroke::new(4.0 * self.zoom, egui::Color32::from_rgb(255, 255, 100)) } 
                    else { egui::Stroke::new(2.5 * self.zoom, egui::Color32::WHITE) }
                } else { egui::Stroke::new(stroke_w * self.zoom, b_color.gamma_multiply(if node.matches_search || self.search_query.is_empty() { 1.0 } else { 0.15 })) };

                if &*node.node_type == "STACK" && self.zoom > 0.3 {
                    let offset1 = egui::vec2(6.0, -6.0) * self.zoom;
                    let offset2 = egui::vec2(12.0, -12.0) * self.zoom;
                    painter.rect_filled(rect.translate(offset2), 8.0 * self.zoom, base_color.linear_multiply(0.5));
                    painter.rect_stroke(rect.translate(offset2), 8.0 * self.zoom, final_stroke.clone());
                    painter.rect_filled(rect.translate(offset1), 8.0 * self.zoom, base_color.linear_multiply(0.8));
                    painter.rect_stroke(rect.translate(offset1), 8.0 * self.zoom, final_stroke.clone());
                }

                painter.rect_filled(rect, 8.0 * self.zoom, base_color);
                painter.rect_stroke(rect, 8.0 * self.zoom, final_stroke);

                if self.zoom > 0.18 {
                    let h_rect = egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + 25.0 * self.zoom));
                    painter.rect_filled(h_rect, egui::Rounding { nw: 4.0 * self.zoom, ne: 4.0 * self.zoom, ..Default::default() }, egui::Color32::from_black_alpha(100));
                    painter.text(h_rect.center(), egui::Align2::CENTER_CENTER, &*node.label, egui::FontId::proportional(12.0 * self.zoom), egui::Color32::from_rgb(165, 180, 252));

                    if self.zoom > 0.4 {
                        let is_container = &*node.node_type == "OBJ" || &*node.node_type == "ARR" || &*node.node_type == "STACK";
                        if !is_container {
                            let text_rect = egui::Rect::from_center_size(rect.center() + egui::vec2(0.0, 10.0 * self.zoom), egui::vec2(190.0, 20.0) * self.zoom);
                            ui.put(text_rect, egui::TextEdit::singleline(&mut node.value).font(egui::FontId::monospace(11.0 * self.zoom)).text_color(egui::Color32::from_rgb(200, 200, 200)).frame(false).horizontal_align(egui::Align::Center));
                            
                            if node.is_secret {
                                let btn_rect = egui::Rect::from_center_size(rect.right_center() + egui::vec2(-20.0 * self.zoom, 8.0 * self.zoom), egui::vec2(24.0, 16.0) * self.zoom);
                                if ui.put(btn_rect, egui::Button::new(egui::RichText::new("🔓").size(10.0 * self.zoom)).fill(egui::Color32::from_rgb(220, 38, 38))).on_hover_text("Decripta Token (JWT/Base64)").clicked() {
                                    do_decode = Some(node.value.clone());
                                }
                            }

                        } else if &*node.node_type == "STACK" {
                             painter.text(rect.center() + egui::vec2(0.0, 10.0 * self.zoom), egui::Align2::CENTER_CENTER, &node.value, egui::FontId::proportional(11.0 * self.zoom), egui::Color32::from_rgb(236, 72, 153));
                        }
                        
                        let badge_c = match &*node.node_type { "STR" => egui::Color32::from_rgb(16, 185, 129), "NUM" => egui::Color32::from_rgb(245, 158, 11), "BOOL" => egui::Color32::from_rgb(168, 85, 247), "NULL" => egui::Color32::from_rgb(239, 68, 68), "ARR" => egui::Color32::from_rgb(56, 189, 248), "STACK" => egui::Color32::from_rgb(236, 72, 153), _ => egui::Color32::from_rgb(99, 102, 241) };
                        let badge_rect = egui::Rect::from_center_size(rect.left_bottom() + egui::vec2(30.0, -16.0) * self.zoom, egui::vec2(34.0, 15.0) * self.zoom);
                        painter.rect_stroke(badge_rect, 2.0 * self.zoom, egui::Stroke::new(1.0 * self.zoom, badge_c));
                        painter.text(badge_rect.center(), egui::Align2::CENTER_CENTER, &*node.node_type, egui::FontId::proportional(9.0 * self.zoom), badge_c);
                    }

                    if &*node.node_type == "OBJ" || &*node.node_type == "ARR" || &*node.node_type == "STACK" {
                        let fold_rect = egui::Rect::from_center_size(rect.right_top() + egui::vec2(-15.0, 12.5) * self.zoom, egui::vec2(20.0, 15.0) * self.zoom);
                        
                        let bg_c = if &*node.node_type == "STACK" { egui::Color32::from_rgb(236, 72, 153).linear_multiply(0.5) } else { egui::Color32::from_black_alpha(150) };
                        painter.rect_filled(fold_rect, 2.0 * self.zoom, bg_c);
                        painter.rect_stroke(fold_rect, 2.0 * self.zoom, egui::Stroke::new(1.0 * self.zoom, egui::Color32::from_gray(100)));
                        
                        let (icon, f_size) = if &*node.node_type == "STACK" { ("+50", 9.0) } else if node.collapsed { ("+", 14.0) } else { ("-", 14.0) };
                        painter.text(fold_rect.center(), egui::Align2::CENTER_CENTER, icon, egui::FontId::monospace(f_size * self.zoom), egui::Color32::WHITE);
                    }
                }
            }

            if !self.is_zen_mode && self.nodes.len() > 0 {
                painter.rect_filled(minimap_rect, 8.0, egui::Color32::from_black_alpha(180));
                painter.rect_stroke(minimap_rect, 8.0, egui::Stroke::new(1.0, egui::Color32::from_gray(80)));
                
                let vp_min = egui::pos2((screen_rect.min.x - self.pan.x) / self.zoom, (screen_rect.min.y - self.pan.y) / self.zoom);
                let vp_max = egui::pos2((screen_rect.max.x - self.pan.x) / self.zoom, (screen_rect.max.y - self.pan.y) / self.zoom);
                
                let mut radar_vp_rect = egui::Rect::from_min_max(
                    radar_offset + vp_min.to_vec2() * radar_scale,
                    radar_offset + vp_max.to_vec2() * radar_scale,
                );
                radar_vp_rect = radar_vp_rect.intersect(minimap_rect.shrink(1.0));
                
                if radar_vp_rect.is_positive() {
                    painter.rect_stroke(radar_vp_rect, 2.0, egui::Stroke::new(1.5, egui::Color32::WHITE.linear_multiply(0.8)));
                }

                if is_hovering_minimap && ctx.input(|i| i.pointer.any_down()) {
                    if let Some(pos) = pointer_pos { self.pan = screen_rect.center().to_vec2() - ((pos - radar_offset) / radar_scale) * self.zoom; }
                }
            }

            let zen_pos = ui.max_rect().left_bottom() + egui::vec2(25.0, -45.0);
            if ui.put(egui::Rect::from_center_size(zen_pos, egui::vec2(45.0, 45.0)), egui::Button::new("🧘").rounding(25.0)).clicked() { self.is_zen_mode = !self.is_zen_mode; }
        });

        // Eventi Post-Rendering (Floating Windows)
        if let Some(secret) = do_decode {
            self.decoded_payload = Some(Self::decode_secret(&secret));
        }

        if let Some(mut payload) = self.decoded_payload.clone() {
            egui::Window::new("🔓 L'Occhio a Raggi X - Payload Decriptato")
                .collapsible(false)
                .resizable(true)
                .default_size([500.0, 350.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("I dati nascosti in questo Token sono:").color(egui::Color32::LIGHT_GREEN));
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(&mut payload)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY));
                    });
                    ui.separator();
                    if ui.button("Chiudi finestra").clicked() {
                        self.decoded_payload = None;
                    }
                });
        }

        if self.loading_state == 1 {
            egui::Window::new("⏳ ESTRAZIONE CARTE...")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Ricalcolo del Grafo 3D in corso.\nNon cliccare, ci vorrà qualche istante...");
                });
            self.loading_state = 2;
            ctx.request_repaint(); 
        }

        if let Some(path) = expand_stack_path {
            self.pending_path = Some(path);
            self.loading_state = 1; 
            ctx.request_repaint();
        }
    }
}