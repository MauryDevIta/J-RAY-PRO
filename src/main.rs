use eframe::egui;
use serde_json::Value;
use similar::{ChangeTag, TextDiff}; 
use jsonpath_rust::JsonPathQuery; 
use std::fs;
use std::time::Instant;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("J-RAY PRO - NITRO GPU + CODE GEN + VISUAL DIFF"),
        ..Default::default()
    };

    eframe::run_native(
        "J-RAY PRO",
        native_options,
        Box::new(|cc| {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = egui::Color32::from_rgb(9, 9, 11);
            cc.egui_ctx.set_visuals(visuals);
            Box::new(JRayPro::default())
        }),
    )
}

// ⚖️ ENUM PER LO STATO DEL DIFF
#[derive(PartialEq, Clone, Copy)]
enum DiffStatus { Normal, Added, Removed, Modified }

struct Node {
    label: Box<str>,
    value: String,
    pos: egui::Pos2,
    node_type: Box<str>,
    matches_search: bool,
    collapsed: bool,
    visible: bool,
    status: DiffStatus, // ⚖️ Stato del nodo per i colori
    raw_val: Value, // ✨ Memorizza il valore reale per fixare JSONPath!
}

struct JRayPro {
    json_input: String,
    json_input_b: String, // 📄 Secondo file per il Diff
    active_tab: usize,    // 0 = File A, 1 = File B
    is_diff_mode: bool,   // ⚖️ Attiva la colorazione speciale
    
    search_query: String,
    search_results_idx: Vec<usize>, // ✨ Indici dei nodi trovati
    current_search_match: usize,    // ✨ Nodo attualmente puntato
    
    nodes: Vec<Node>,
    connections: Vec<(usize, usize)>,
    pan: egui::Vec2,
    zoom: f32,
    status_msg: String,
    is_zen_mode: bool,
    last_search_idx: usize,
    dragged_node: Option<usize>,
    
    // 🚀 GESTIONE FILE GIGANTI & CODE GEN
    is_huge_file: bool,
    raw_full_json: Option<String>,
    show_code_gen: bool,
    code_gen_lang: usize, 
    generated_code: String,
}

impl Default for JRayPro {
    fn default() -> Self {
        Self {
            json_input: r#"{"app": "J-RAY PRO", "features": ["Deep Code Gen", "Minimap", "Folding"]}"#.to_string(),
            json_input_b: r#"{"app": "J-RAY PRO", "features": ["Visual Diff", "Minimap", "Folding"]}"#.to_string(),
            active_tab: 0,
            is_diff_mode: false,
            search_query: "".to_string(),
            search_results_idx: Vec::new(),
            current_search_match: 0,
            nodes: Vec::new(),
            connections: Vec::new(),
            pan: egui::Vec2::ZERO,
            zoom: 1.0,
            status_msg: "Trifecta Engine Online".to_string(),
            is_zen_mode: false,
            last_search_idx: 0,
            dragged_node: None,
            is_huge_file: false,
            raw_full_json: None,
            show_code_gen: false,
            code_gen_lang: 0,
            generated_code: "".to_string(),
        }
    }
}

impl JRayPro {
    fn get_type_info(v: &Value) -> (&str, egui::Color32) {
        match v {
            Value::String(_) => ("STR", egui::Color32::from_rgb(16, 185, 129)),
            Value::Number(_) => ("NUM", egui::Color32::from_rgb(245, 158, 11)),
            Value::Bool(_) => ("BOOL", egui::Color32::from_rgb(168, 85, 247)),
            Value::Null => ("NULL", egui::Color32::from_rgb(239, 68, 68)),
            Value::Array(_) => ("ARR", egui::Color32::from_rgb(56, 189, 248)),
            _ => ("OBJ", egui::Color32::from_rgb(99, 102, 241)),
        }
    }

    fn format_json(&mut self) {
        if self.is_huge_file { return; } 
        let target_input = if self.active_tab == 0 { &mut self.json_input } else { &mut self.json_input_b };
        if let Ok(value) = serde_json::from_str::<Value>(target_input) {
            if let Ok(pretty) = serde_json::to_string_pretty(&value) {
                *target_input = pretty;
                self.status_msg = "JSON formattato".to_string();
            }
        }
    }

    fn build_json_value(&self, idx: usize) -> Value {
        let node = &self.nodes[idx];
        match &*node.node_type {
            "OBJ" => {
                let mut map = serde_json::Map::new();
                for &(p, c) in &self.connections {
                    if p == idx {
                        map.insert(self.nodes[c].label.to_string(), self.build_json_value(c));
                    }
                }
                Value::Object(map)
            },
            "ARR" => {
                let mut arr = Vec::new();
                let mut children: Vec<usize> = self.connections.iter()
                    .filter(|&&(p, _)| p == idx).map(|&(_, c)| c).collect();
                children.sort(); 
                for c in children { arr.push(self.build_json_value(c)); }
                Value::Array(arr)
            },
            _ => serde_json::from_str(&node.value).unwrap_or_else(|_| Value::String(node.value.clone()))
        }
    }

    fn sync_graph_to_json(&mut self) {
        if self.nodes.is_empty() || self.is_huge_file || self.is_diff_mode { return; }
        let root_val = self.build_json_value(0);
        if let Ok(pretty) = serde_json::to_string_pretty(&root_val) {
            if self.active_tab == 0 { self.json_input = pretty; } else { self.json_input_b = pretty; }
            self.status_msg = "Sincronizzazione completata!".to_string();
        }
    }

    fn save_file(&mut self) {
        if self.is_huge_file { return; } 
        self.sync_graph_to_json(); 
        if let Some(p) = rfd::FileDialog::new().add_filter("JSON", &["json"]).save_file() {
            let target_input = if self.active_tab == 0 { &self.json_input } else { &self.json_input_b };
            if fs::write(p, target_input).is_ok() {
                self.status_msg = "💾 File salvato con successo".to_string();
            }
        }
    }

    fn export_to_svg(&mut self) {
        if self.nodes.is_empty() { return; }

        if let Some(path) = rfd::FileDialog::new().add_filter("SVG Vector", &["svg"]).save_file() {
            let start = Instant::now();
            let mut min_x = f32::MAX; let mut min_y = f32::MAX;
            let mut max_x = f32::MIN; let mut max_y = f32::MIN;

            for n in &self.nodes {
                if !n.visible { continue; }
                if n.pos.x < min_x { min_x = n.pos.x; }
                if n.pos.y < min_y { min_y = n.pos.y; }
                if n.pos.x + 220.0 > max_x { max_x = n.pos.x + 220.0; }
                if n.pos.y + 65.0 > max_y { max_y = n.pos.y + 65.0; }
            }

            min_x -= 100.0; min_y -= 100.0;
            max_x += 100.0; max_y += 100.0;
            let width = max_x - min_x;
            let height = max_y - min_y;

            let mut svg = String::with_capacity(self.nodes.len() * 500);
            
            svg.push_str(&format!(
                "<svg viewBox=\"{} {} {} {}\" xmlns=\"http://www.w3.org/2000/svg\" style=\"background-color:#09090b; font-family: sans-serif;\">\n", 
                min_x, min_y, width, height
            ));

            for &(s_i, e_i) in &self.connections {
                let n1 = &self.nodes[s_i];
                let n2 = &self.nodes[e_i];
                if !n1.visible || !n2.visible { continue; }

                let p1x = n1.pos.x + 220.0; let p1y = n1.pos.y + 32.5;
                let p2x = n2.pos.x; let p2y = n2.pos.y + 32.5;
                
                svg.push_str(&format!(
                    "<path d=\"M {},{} C {},{} {},{} {},{}\" fill=\"none\" stroke=\"#6366f1\" stroke-width=\"1.5\" />\n", 
                    p1x, p1y, p1x + 80.0, p1y, p2x - 80.0, p2y, p2x, p2y
                ));
            }

            let escape_xml = |s: &str| -> String {
                s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
            };

            for n in &self.nodes {
                if !n.visible { continue; }
                
                let border_col = if self.is_diff_mode {
                    match n.status {
                        DiffStatus::Added => "#22c55e",
                        DiffStatus::Removed => "#ef4444",
                        DiffStatus::Modified => "#eab308",
                        DiffStatus::Normal => "#6366f1",
                    }
                } else {
                    if &*n.node_type == "OBJ" { "#22d3ee" } else { "#6366f1" }
                };

                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"220\" height=\"65\" rx=\"8\" fill=\"#18181b\" stroke=\"{}\" stroke-width=\"1.5\" />\n", n.pos.x, n.pos.y, border_col));
                svg.push_str(&format!("<path d=\"M {},{} a 8 8 0 0 1 8 -8 h 204 a 8 8 0 0 1 8 8 v 17 h -220 z\" fill=\"#000000\" fill-opacity=\"0.4\" />\n", n.pos.x, n.pos.y + 8.0));
                
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" fill=\"#a5b4fc\" font-size=\"12\" font-weight=\"bold\" text-anchor=\"middle\">{}</text>\n", n.pos.x + 110.0, n.pos.y + 17.0, escape_xml(&n.label)));
                
                if !n.value.is_empty() {
                    svg.push_str(&format!("<text x=\"{}\" y=\"{}\" fill=\"#9ca3af\" font-size=\"11\" font-family=\"monospace\" text-anchor=\"middle\">{}</text>\n", n.pos.x + 110.0, n.pos.y + 44.5, escape_xml(&n.value)));
                }

                let badge_col = match &*n.node_type {
                    "STR" => "#10b981", "NUM" => "#f59e0b", "BOOL" => "#a855f7", "NULL" => "#ef4444", "ARR" => "#38bdf8", _ => "#6366f1",
                };
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"34\" height=\"15\" rx=\"2\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\" />\n", n.pos.x + 13.0, n.pos.y + 41.5, badge_col));
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"9\" font-weight=\"bold\" text-anchor=\"middle\">{}</text>\n", n.pos.x + 30.0, n.pos.y + 52.0, badge_col, n.node_type));
            }

            svg.push_str("</svg>");
            
            if fs::write(path, svg).is_ok() {
                let elapsed = start.elapsed();
                self.status_msg = format!("📸 SVG generato in {:?}", elapsed);
            }
        }
    }

    fn capitalize(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }

    fn generate_types(&mut self) {
        if self.nodes.is_empty() {
            self.generated_code = "// Nessun dato presente nel grafo.".to_string();
            return;
        }

        let root_val = if self.is_huge_file && self.raw_full_json.is_some() {
            let raw = self.raw_full_json.as_ref().unwrap();
            serde_json::from_str(raw).unwrap_or(Value::Null)
        } else {
            self.build_json_value(0) 
        };

        let mut output = String::new();

        match self.code_gen_lang {
            0 => { Self::gen_ts(&root_val, "Root", &mut output); },
            1 => { 
                output.push_str("use serde::{Serialize, Deserialize};\n\n");
                Self::gen_rust(&root_val, "Root", &mut output); 
            },
            2 => { 
                output.push_str("from typing import List, Any, Optional\nfrom pydantic import BaseModel\n\n");
                Self::gen_py(&root_val, "Root", &mut output); 
            },
            _ => {}
        }
        self.generated_code = output;
    }

    fn gen_ts(value: &Value, name: &str, output: &mut String) -> String {
        match value {
            Value::Object(map) => {
                let mut fields = String::new();
                for (k, v) in map {
                    let field_type = Self::gen_ts(v, &Self::capitalize(k), output);
                    fields.push_str(&format!("  {}: {};\n", k, field_type));
                }
                output.push_str(&format!("export interface {} {{\n{}}}\n\n", name, fields));
                name.to_string()
            },
            Value::Array(arr) => {
                if arr.is_empty() { "any[]".to_string() }
                else { format!("{}[]", Self::gen_ts(&arr[0], &format!("{}Item", name), output)) }
            },
            Value::String(_) => "string".to_string(),
            Value::Number(_) => "number".to_string(),
            Value::Bool(_) => "boolean".to_string(),
            Value::Null => "any".to_string(),
        }
    }

    fn gen_rust(value: &Value, name: &str, output: &mut String) -> String {
        match value {
            Value::Object(map) => {
                let mut fields = String::new();
                for (k, v) in map {
                    let field_type = Self::gen_rust(v, &Self::capitalize(k), output);
                    fields.push_str(&format!("    pub {}: {},\n", k, field_type));
                }
                output.push_str(&format!("#[derive(Debug, Serialize, Deserialize)]\npub struct {} {{\n{}}}\n\n", name, fields));
                name.to_string()
            },
            Value::Array(arr) => {
                if arr.is_empty() { "Vec<Value>".to_string() }
                else { format!("Vec<{}>", Self::gen_rust(&arr[0], &format!("{}Item", name), output)) }
            },
            Value::String(_) => "String".to_string(),
            Value::Number(n) => if n.is_f64() { "f64".to_string() } else { "i64".to_string() },
            Value::Bool(_) => "bool".to_string(),
            Value::Null => "Option<Value>".to_string(),
        }
    }

    fn gen_py(value: &Value, name: &str, output: &mut String) -> String {
        match value {
            Value::Object(map) => {
                let mut fields = String::new();
                for (k, v) in map {
                    let field_type = Self::gen_py(v, &Self::capitalize(k), output);
                    fields.push_str(&format!("    {}: {}\n", k, field_type));
                }
                if fields.is_empty() { fields = "    pass\n".to_string(); }
                output.push_str(&format!("class {}(BaseModel):\n{}\n\n", name, fields));
                name.to_string()
            },
            Value::Array(arr) => {
                if arr.is_empty() { "List[Any]".to_string() }
                else { format!("List[{}]", Self::gen_py(&arr[0], &format!("{}Item", name), output)) }
            },
            Value::String(_) => "str".to_string(),
            Value::Number(n) => if n.is_f64() { "float".to_string() } else { "int".to_string() },
            Value::Bool(_) => "bool".to_string(),
            Value::Null => "Optional[Any]".to_string(),
        }
    }

    fn run_diff(&mut self) {
        let start = Instant::now();
        self.nodes.clear();
        self.connections.clear();
        self.is_diff_mode = true;
        
        if let Ok(v1) = serde_json::from_str::<Value>(&self.json_input) {
            if let Ok(p1) = serde_json::to_string_pretty(&v1) { self.json_input = p1; }
        }
        if let Ok(v2) = serde_json::from_str::<Value>(&self.json_input_b) {
            if let Ok(p2) = serde_json::to_string_pretty(&v2) { self.json_input_b = p2; }
        }

        let v1 = serde_json::from_str::<Value>(&self.json_input).unwrap_or(Value::Null);
        let v2 = serde_json::from_str::<Value>(&self.json_input_b).unwrap_or(Value::Null);
        let mut s_idx: f32 = 0.0;
        self.diff_traverse(Some(&v1), Some(&v2), "root".to_string(), None, 0, &mut s_idx);
        
        self.status_msg = format!("⚖️ Diff calcolato in {:?}", start.elapsed());
    }

    fn diff_traverse(&mut self, v1: Option<&Value>, v2: Option<&Value>, label: String, p_idx: Option<usize>, d: usize, s_idx: &mut f32) {
        if self.nodes.len() > 150000 { return; }

        let status = match (v1, v2) {
            (Some(a), Some(b)) if a == b => DiffStatus::Normal,
            (Some(_), Some(_)) => DiffStatus::Modified,
            (None, Some(_)) => DiffStatus::Added,
            (Some(_), None) => DiffStatus::Removed,
            (None, None) => return,
        };

        let val_to_show = v2.or(v1).unwrap(); 
        let (t_label, _) = Self::get_type_info(val_to_show);
        let n_idx = self.nodes.len();
        let val_str = if val_to_show.is_object() || val_to_show.is_array() { "".to_string() } else { val_to_show.to_string() };

        self.nodes.push(Node {
            label: label.into_boxed_str(),
            value: val_str,
            node_type: t_label.into(),
            pos: egui::pos2(d as f32 * 350.0, *s_idx * 120.0),
            matches_search: true,
            collapsed: false,
            visible: true,
            status,
            raw_val: val_to_show.clone(), // ✨ FIX
        });

        if let Some(pi) = p_idx { self.connections.push((pi, n_idx)); }

        let obj1 = v1.and_then(|v| v.as_object());
        let obj2 = v2.and_then(|v| v.as_object());

        if obj1.is_some() || obj2.is_some() {
            let mut keys = std::collections::HashSet::new();
            if let Some(o) = obj1 { keys.extend(o.keys()); }
            if let Some(o) = obj2 { keys.extend(o.keys()); }
            
            let mut sorted_keys: Vec<_> = keys.into_iter().collect();
            sorted_keys.sort();

            for k in sorted_keys {
                let child_v1 = obj1.and_then(|o| o.get(k));
                let child_v2 = obj2.and_then(|o| o.get(k));
                self.diff_traverse(child_v1, child_v2, k.clone(), Some(n_idx), d + 1, s_idx);
                *s_idx += 1.0;
            }
        } else {
            let arr1 = v1.and_then(|v| v.as_array());
            let arr2 = v2.and_then(|v| v.as_array());
            if arr1.is_some() || arr2.is_some() {
                let len1 = arr1.map_or(0, |a| a.len());
                let len2 = arr2.map_or(0, |a| a.len());
                let max_len = std::cmp::max(len1, len2);

                for i in 0..max_len {
                    let child_v1 = arr1.and_then(|a| a.get(i));
                    let child_v2 = arr2.and_then(|a| a.get(i));
                    self.diff_traverse(child_v1, child_v2, format!("[{}]", i), Some(n_idx), d + 1, s_idx);
                    *s_idx += 1.0;
                }
            }
        }
    }

    fn generate_graph_from_string(&mut self, text: &str) {
        let start = Instant::now();
        self.nodes.clear();
        self.connections.clear();
        self.is_diff_mode = false; 

        if let Ok(v) = serde_json::from_str::<Value>(text) {
            let mut s_idx: f32 = 0.0;
            self.traverse(&v, "root".to_string(), None, 0, &mut s_idx);
            
            let dummy_center = egui::pos2(0.0, 0.0);
            self.apply_search(dummy_center);
            let elapsed = start.elapsed();
            self.status_msg = format!("Nitro: {} nodi in {:?}", self.nodes.len(), elapsed);
        } else {
            self.status_msg = "ERRORE: JSON non valido".to_string();
        }
    }

    fn traverse(&mut self, value: &Value, label: String, p_idx: Option<usize>, d: usize, s_idx: &mut f32) {
        if self.nodes.len() > 150000 { return; } 
        let (t_label, _) = Self::get_type_info(value);
        let n_idx = self.nodes.len();
        let val_str = if value.is_object() || value.is_array() { "".to_string() } else { value.to_string() };

        self.nodes.push(Node {
            label: label.into_boxed_str(),
            value: val_str,
            node_type: t_label.into(),
            pos: egui::pos2(d as f32 * 350.0, *s_idx * 120.0),
            matches_search: true,
            collapsed: false,
            visible: true,
            status: DiffStatus::Normal,
            raw_val: value.clone(), // ✨ FIX
        });

        if let Some(pi) = p_idx { self.connections.push((pi, n_idx)); }

        if let Some(obj) = value.as_object() {
            for (k, v) in obj { 
                self.traverse(v, k.clone(), Some(n_idx), d + 1, s_idx); 
                *s_idx += 1.0; 
            }
        } else if let Some(arr) = value.as_array() {
            for (i, v) in arr.iter().enumerate() { 
                self.traverse(v, format!("[{}]", i), Some(n_idx), d + 1, s_idx); 
                *s_idx += 1.0; 
            }
        }
    }

    fn update_visibility(&mut self) {
        for n in &mut self.nodes { n.visible = true; } 
        for &(p, c) in &self.connections {
            if !self.nodes[p].visible || self.nodes[p].collapsed {
                self.nodes[c].visible = false;
            }
        }
    }

    // ✨ MOTORE DI RICERCA DEFINITIVO (Puntuale e Preciso)
    fn apply_search(&mut self, view_center: egui::Pos2) {
        let query = self.search_query.trim().to_string();
        self.search_results_idx.clear();
        self.current_search_match = 0;
        
        if query.is_empty() {
            for node in &mut self.nodes { node.matches_search = true; node.visible = true; }
            self.update_visibility();
            return;
        }

        // AUTO-EXPAND: Apre tutti i nodi così l'utente può vedere i risultati interni (gli OBJ)
        for node in &mut self.nodes { node.collapsed = false; }
        self.update_visibility();

        if query.starts_with("$.") || query.starts_with("$[") {
            let target_text = if self.active_tab == 0 { &self.json_input } else { &self.json_input_b };
            
            if let Ok(value) = serde_json::from_str::<Value>(target_text) {
                match value.path(&query) {
                    Ok(results) => {
                        if let Some(arr) = results.as_array() {
                            if arr.is_empty() {
                                for node in &mut self.nodes { node.matches_search = false; }
                                self.status_msg = "🔍 Nessun risultato per questa query".to_string();
                            } else {
                                // Confronto esatto tra il valore originale e quello cercato
                                for (i, node) in self.nodes.iter_mut().enumerate() {
                                    node.matches_search = arr.contains(&node.raw_val);
                                    if node.matches_search && node.visible {
                                        self.search_results_idx.push(i);
                                    }
                                }
                                self.status_msg = format!("🎯 Trovati {} match esatti", self.search_results_idx.len());
                            }
                        }
                    },
                    Err(_) => {
                        for node in &mut self.nodes { node.matches_search = false; }
                        self.status_msg = "❌ Sintassi JSONPath non valida".to_string();
                    }
                }
            }
        } else {
            let q_lower = query.to_lowercase();
            for (i, node) in self.nodes.iter_mut().enumerate() {
                node.matches_search = node.label.to_lowercase().contains(&q_lower) || 
                                     node.value.to_lowercase().contains(&q_lower);
                if node.matches_search && node.visible {
                    self.search_results_idx.push(i);
                }
            }
            self.status_msg = format!("🔍 Trovati {} risultati testuali", self.search_results_idx.len());
        }

        if !self.search_results_idx.is_empty() {
            self.focus_current_match(view_center);
        }
    }

    // ✨ SISTEMA DI NAVIGAZIONE MULTIPLA
    fn focus_current_match(&mut self, view_center: egui::Pos2) {
        if self.search_results_idx.is_empty() { return; }
        let target_idx = self.search_results_idx[self.current_search_match];
        let target_pos = self.nodes[target_idx].pos;
        self.zoom = 1.0;
        self.pan = view_center.to_vec2() - (target_pos + egui::vec2(110.0, 32.5)).to_vec2() * self.zoom;
    }

    fn next_search_match(&mut self, view_center: egui::Pos2) {
        if self.search_results_idx.is_empty() { return; }
        self.current_search_match = (self.current_search_match + 1) % self.search_results_idx.len();
        self.focus_current_match(view_center);
    }

    fn prev_search_match(&mut self, view_center: egui::Pos2) {
        if self.search_results_idx.is_empty() { return; }
        if self.current_search_match == 0 {
            self.current_search_match = self.search_results_idx.len() - 1;
        } else {
            self.current_search_match -= 1;
        }
        self.focus_current_match(view_center);
    }

    fn open_file(&mut self, is_file_b: bool) {
        if let Some(p) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
            if let Ok(metadata) = fs::metadata(&p) {
                let size_mb = metadata.len() as f64 / 1_048_576.0;
                
                if let Ok(full_text) = fs::read_to_string(p) {
                    if !is_file_b { self.raw_full_json = Some(full_text.clone()); }

                    let preview_text = if size_mb > 5.0 {
                        self.is_huge_file = true;
                        if !is_file_b { self.generate_graph_from_string(&full_text); }
                        format!("/* ⚠️ FILE ENORME: {:.1} MB ⚠️\n* Sincronizzazione ed Editing disabilitati.\n*/\n\n{}", size_mb, full_text.chars().take(10000).collect::<String>())
                    } else {
                        self.is_huge_file = false;
                        full_text
                    };

                    if is_file_b {
                        self.json_input_b = preview_text;
                        self.active_tab = 1;
                        self.status_msg = "File B caricato".to_string();
                    } else {
                        self.json_input = preview_text;
                        self.active_tab = 0;
                        self.status_msg = "File A caricato".to_string();
                    }
                }
            }
        }
    }
}

impl eframe::App for JRayPro {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 { self.zoom = (self.zoom + scroll * 0.002).clamp(0.01, 5.0); }

        if !self.is_zen_mode {
            egui::TopBottomPanel::top("menu").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("J-RAY PRO").strong().color(egui::Color32::from_rgb(99, 102, 241)));
                    ui.separator();
                    
                    if ui.button("📂 File A").clicked() { self.open_file(false); }
                    if ui.button("📂 File B").clicked() { self.open_file(true); }
                    
                    ui.add_enabled(!self.is_huge_file, egui::Button::new("💾 Salva")).clicked().then(|| {
                        self.save_file();
                    });

                    ui.separator();
                    if ui.button(egui::RichText::new("⚖️ Visual Diff").color(egui::Color32::YELLOW)).on_hover_text("Compara il File A col File B").clicked() { 
                        self.run_diff(); 
                    }

                    ui.separator();
                    if ui.button("📸 Esporta SVG").clicked() { self.export_to_svg(); }
                    if ui.button("🧬 Code Gen").on_hover_text("Genera interfacce e classi a partire dai dati").clicked() {
                        self.generate_types(); 
                        self.show_code_gen = true;
                    }

                    ui.add_enabled(!self.is_huge_file, egui::Button::new("🚀 Genera Grafo")).clicked().then(|| {
                        if !self.is_huge_file { 
                            let text = if self.active_tab == 0 { self.json_input.clone() } else { self.json_input_b.clone() };
                            self.generate_graph_from_string(&text); 
                        }
                    });
                    
                    ui.separator();
                    ui.label("🔍");
                    
                    let view_center = ctx.available_rect().center();
                    
                    // ✨ BARRA DI RICERCA CON FRECCE DI NAVIGAZIONE
                    let s_resp = ui.add(egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Cerca parola o $.jsonPath...")
                        .desired_width(200.0));
                        
                    if s_resp.changed() { self.apply_search(view_center); }
                    
                    // Se premi INVIO scatta la freccia Avanti
                    if s_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.next_search_match(view_center);
                    }

                    // ⬅️ CONTATORE E FRECCE ➡️
                    if !self.search_results_idx.is_empty() {
                        ui.label(egui::RichText::new(format!("{}/{}", self.current_search_match + 1, self.search_results_idx.len()))
                            .color(egui::Color32::LIGHT_GRAY).monospace());
                        
                        if ui.button("⬆").clicked() { self.prev_search_match(view_center); }
                        if ui.button("⬇").clicked() { self.next_search_match(view_center); }
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(&self.status_msg).color(egui::Color32::LIGHT_BLUE));
                        
                        if self.is_diff_mode {
                            ui.label(egui::RichText::new("■ Modificato").color(egui::Color32::from_rgb(234, 179, 8)));
                            ui.label(egui::RichText::new("■ Rimosso").color(egui::Color32::from_rgb(239, 68, 68)));
                            ui.label(egui::RichText::new("■ Aggiunto").color(egui::Color32::from_rgb(34, 197, 94)));
                        }
                    });
                });
            });

            // ✨ EDITOR E VISTA TESTO DIFF
            egui::SidePanel::left("editor").width_range(300.0..=600.0).show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Editor");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_enabled(!self.is_huge_file && !self.is_diff_mode, egui::Button::new("✨ Format")).clicked().then(|| {
                                self.format_json();
                            });
                            ui.add_enabled(!self.is_huge_file && !self.nodes.is_empty() && !self.is_diff_mode, egui::Button::new("🔄 Sync Grafo"))
                                .clicked().then(|| { self.sync_graph_to_json(); });
                        });
                    });

                    if self.is_diff_mode {
                        ui.separator();
                        ui.label(egui::RichText::new("👀 Comparazione Testo").color(egui::Color32::YELLOW).strong());
                        ui.separator();
                        
                        if self.is_huge_file {
                            ui.label(egui::RichText::new("⚠️ TESTO TROPPO GRANDE PER IL DIFF INLINE").color(egui::Color32::RED));
                            ui.label("Usa il grafo 3D a destra per esplorare le differenze.");
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
                                ui.add(egui::TextEdit::multiline(&mut self.json_input)
                                    .font(egui::TextStyle::Monospace).text_color(tc).interactive(!self.is_huge_file).desired_width(f32::INFINITY));
                            } else {
                                ui.add(egui::TextEdit::multiline(&mut self.json_input_b)
                                    .font(egui::TextStyle::Monospace).text_color(tc).interactive(!self.is_huge_file).desired_width(f32::INFINITY));
                            }
                        });
                    }
                });
            });
        }

        let mut show_window = self.show_code_gen;
        if show_window {
            egui::Window::new("🧬 Type Inference Engine")
                .open(&mut show_window)
                .default_size(egui::vec2(500.0, 600.0))
                .vscroll(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Linguaggio: ");
                        if ui.radio_value(&mut self.code_gen_lang, 0, "TypeScript").changed() { self.generate_types(); }
                        if ui.radio_value(&mut self.code_gen_lang, 1, "Rust").changed() { self.generate_types(); }
                        if ui.radio_value(&mut self.code_gen_lang, 2, "Python (Pydantic)").changed() { self.generate_types(); }
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("📋 Copia Codice").clicked() {
                                ctx.output_mut(|o| o.copied_text = self.generated_code.clone());
                                self.status_msg = "Codice copiato negli appunti!".to_string();
                            }
                        });
                    });
                    ui.separator();
                    
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(&mut self.generated_code)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .code_editor());
                    });
                });
        }
        self.show_code_gen = show_window;

        egui::CentralPanel::default().show(ctx, |ui| {
            let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());
            let pointer_pos = ctx.input(|i| i.pointer.hover_pos());

            let minimap_size = egui::vec2(240.0, 160.0);
            let minimap_rect = egui::Rect::from_min_size(ui.max_rect().right_bottom() - minimap_size - egui::vec2(20.0, 20.0), minimap_size);
            let mut is_hovering_minimap = false;
            
            if let Some(pos) = pointer_pos {
                if minimap_rect.contains(pos) { is_hovering_minimap = true; }
            }

            if resp.dragged() && self.dragged_node.is_none() && !is_hovering_minimap {
                self.pan += resp.drag_delta();
            }

            let current_zoom = self.zoom;
            let current_pan = self.pan;
            let to_screen = move |p: egui::Pos2| egui::pos2(p.x * current_zoom + current_pan.x, p.y * current_zoom + current_pan.y);
            let screen_rect = painter.clip_rect();

            let dot_gap = 40.0 * self.zoom;
            if self.zoom > 0.15 {
                let dot_color = egui::Color32::from_gray(30);
                for x in 0..=(ui.available_width() / dot_gap) as i32 + 1 {
                    for y in 0..=(ui.available_height() / dot_gap) as i32 + 1 {
                        let dot_pos = egui::pos2((x as f32 * dot_gap) + (self.pan.x % dot_gap), (y as f32 * dot_gap) + (self.pan.y % dot_gap));
                        painter.circle_filled(dot_pos, 1.0, dot_color);
                    }
                }
            }

            let edge_stroke = egui::Stroke::new(1.2 * self.zoom, egui::Color32::from_rgb(99, 102, 241));
            for (s_i, e_i) in &self.connections {
                if !self.nodes[*s_i].visible || !self.nodes[*e_i].visible { continue; } 

                let n1 = &self.nodes[*s_i];
                let n2 = &self.nodes[*e_i];
                let p1 = to_screen(n1.pos + egui::vec2(220.0, 32.5));
                let p2 = to_screen(n2.pos + egui::vec2(0.0, 32.5));

                if screen_rect.contains(p1) || screen_rect.contains(p2) {
                    if self.zoom > 0.4 {
                        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                            [p1, p1 + egui::vec2(80.0 * self.zoom, 0.0), p2 - egui::vec2(80.0 * self.zoom, 0.0), p2],
                            false, egui::Color32::TRANSPARENT, edge_stroke
                        ));
                    } else {
                        painter.line_segment([p1, p2], edge_stroke);
                    }
                }
            }

            let mut requires_visibility_update = false;

            if ctx.input(|i| i.pointer.any_pressed()) && !is_hovering_minimap {
                if let Some(pos) = pointer_pos {
                    for (idx, node) in self.nodes.iter_mut().enumerate().rev() {
                        if !node.visible { continue; }
                        let rect = egui::Rect::from_min_size(to_screen(node.pos), egui::vec2(220.0, 65.0) * self.zoom);
                        let header_rect = egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + 25.0 * self.zoom));
                        
                        let is_container = &*node.node_type == "OBJ" || &*node.node_type == "ARR";
                        let fold_rect = egui::Rect::from_center_size(
                            rect.right_top() + egui::vec2(-15.0, 12.5) * self.zoom, 
                            egui::vec2(20.0, 15.0) * self.zoom
                        );

                        if is_container && fold_rect.contains(pos) {
                            node.collapsed = !node.collapsed;
                            requires_visibility_update = true;
                            break; 
                        }

                        if header_rect.contains(pos) && !fold_rect.contains(pos) {
                            self.dragged_node = Some(idx); 
                            break; 
                        }
                    }
                }
            }

            if requires_visibility_update { self.update_visibility(); }
            if ctx.input(|i| i.pointer.any_released()) { self.dragged_node = None; }
            if let Some(idx) = self.dragged_node { self.nodes[idx].pos += resp.drag_delta() / self.zoom; }

            let mut min_w = egui::pos2(f32::MAX, f32::MAX);
            let mut max_w = egui::pos2(f32::MIN, f32::MIN);
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

                if !self.is_zen_mode {
                    let radar_pos = radar_offset + node.pos.to_vec2() * radar_scale;
                    painter.circle_filled(radar_pos, 1.5, egui::Color32::from_rgb(100, 100, 180));
                }

                let rect = egui::Rect::from_min_size(to_screen(node.pos), egui::vec2(220.0, 65.0) * self.zoom);
                if !screen_rect.intersects(rect) { continue; }

                let is_dragged = self.dragged_node == Some(idx);
                let base_color = if is_dragged { egui::Color32::from_rgb(40, 40, 50) }
                                 else if node.matches_search { egui::Color32::from_rgb(24, 24, 27) } 
                                 else { egui::Color32::from_rgb(15, 15, 18) };

                painter.rect_filled(rect, 8.0 * self.zoom, base_color);
                
                let b_color = if self.is_diff_mode {
                    match node.status {
                        DiffStatus::Added => egui::Color32::from_rgb(34, 197, 94),   // 🟩 Verde
                        DiffStatus::Removed => egui::Color32::from_rgb(239, 68, 68), // 🟥 Rosso
                        DiffStatus::Modified => egui::Color32::from_rgb(234, 179, 8),// 🟨 Giallo
                        DiffStatus::Normal => egui::Color32::from_rgb(99, 102, 241), // 🟦 Blu Default
                    }
                } else {
                    if &*node.node_type == "OBJ" { egui::Color32::from_rgb(34, 211, 238) } else { egui::Color32::from_rgb(99, 102, 241) }
                };

                let stroke_w = if self.is_diff_mode && node.status != DiffStatus::Normal { 2.5 } else { 1.2 };
                
                // ✨ LOGICA VISIVA MATCH: Bordo BIANCO brillante se fa match e c'è una query attiva
                let is_currently_focused = !self.search_results_idx.is_empty() && self.search_results_idx[self.current_search_match] == idx;
                
                let final_stroke = if node.matches_search && !self.search_query.is_empty() {
                    if is_currently_focused {
                        // Se è il nodo attualmente selezionato con le frecce, è ANCORA PIÙ EVIDENTE
                        egui::Stroke::new(4.0 * self.zoom, egui::Color32::from_rgb(255, 255, 100))
                    } else {
                        egui::Stroke::new(2.5 * self.zoom, egui::Color32::WHITE)
                    }
                } else {
                     egui::Stroke::new(stroke_w * self.zoom, b_color.gamma_multiply(if node.matches_search || self.search_query.is_empty() { 1.0 } else { 0.15 }))
                };

                painter.rect_stroke(rect, 8.0 * self.zoom, final_stroke);

                if self.zoom > 0.18 {
                    let h_rect = egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + 25.0 * self.zoom));
                    painter.rect_filled(h_rect, egui::Rounding { nw: 4.0 * self.zoom, ne: 4.0 * self.zoom, ..Default::default() }, egui::Color32::from_black_alpha(100));
                    painter.text(h_rect.center(), egui::Align2::CENTER_CENTER, &*node.label, egui::FontId::proportional(12.0 * self.zoom), egui::Color32::from_rgb(165, 180, 252));

                    if self.zoom > 0.4 {
                        let is_container = &*node.node_type == "OBJ" || &*node.node_type == "ARR";

                        if !is_container {
                            let text_rect = egui::Rect::from_center_size(
                                rect.center() + egui::vec2(0.0, 10.0 * self.zoom),
                                egui::vec2(190.0, 20.0) * self.zoom
                            );
                            
                            ui.put(text_rect, egui::TextEdit::singleline(&mut node.value)
                                .font(egui::FontId::monospace(11.0 * self.zoom))
                                .text_color(egui::Color32::from_rgb(200, 200, 200))
                                .frame(false) 
                                .horizontal_align(egui::Align::Center)
                            );
                        }
                        
                        let badge_c = match &*node.node_type {
                            "STR" => egui::Color32::from_rgb(16, 185, 129),
                            "NUM" => egui::Color32::from_rgb(245, 158, 11),
                            "BOOL" => egui::Color32::from_rgb(168, 85, 247),
                            "NULL" => egui::Color32::from_rgb(239, 68, 68),
                            "ARR" => egui::Color32::from_rgb(56, 189, 248),
                            _ => egui::Color32::from_rgb(99, 102, 241),
                        };
                        let badge_rect = egui::Rect::from_center_size(
                            rect.left_bottom() + egui::vec2(30.0, -16.0) * self.zoom, 
                            egui::vec2(34.0, 15.0) * self.zoom
                        );
                        painter.rect_stroke(badge_rect, 2.0 * self.zoom, egui::Stroke::new(1.0 * self.zoom, badge_c));
                        painter.text(badge_rect.center(), egui::Align2::CENTER_CENTER, &*node.node_type, egui::FontId::proportional(9.0 * self.zoom), badge_c);
                    }

                    if &*node.node_type == "OBJ" || &*node.node_type == "ARR" {
                        let fold_rect = egui::Rect::from_center_size(
                            rect.right_top() + egui::vec2(-15.0, 12.5) * self.zoom, 
                            egui::vec2(20.0, 15.0) * self.zoom
                        );
                        painter.rect_filled(fold_rect, 2.0 * self.zoom, egui::Color32::from_black_alpha(150));
                        painter.rect_stroke(fold_rect, 2.0 * self.zoom, egui::Stroke::new(1.0 * self.zoom, egui::Color32::from_gray(100)));
                        let icon = if node.collapsed { "+" } else { "-" };
                        painter.text(fold_rect.center(), egui::Align2::CENTER_CENTER, icon, egui::FontId::monospace(14.0 * self.zoom), egui::Color32::WHITE);
                    }
                }
            }

            if !self.is_zen_mode && self.nodes.len() > 0 {
                painter.rect_filled(minimap_rect, 8.0, egui::Color32::from_black_alpha(180));
                painter.rect_stroke(minimap_rect, 8.0, egui::Stroke::new(1.0, egui::Color32::from_gray(80)));

                let vp_min = egui::pos2((screen_rect.min.x - self.pan.x) / self.zoom, (screen_rect.min.y - self.pan.y) / self.zoom);
                let vp_max = egui::pos2((screen_rect.max.x - self.pan.x) / self.zoom, (screen_rect.max.y - self.pan.y) / self.zoom);
                let radar_vp_rect = egui::Rect::from_min_max(
                    radar_offset + vp_min.to_vec2() * radar_scale,
                    radar_offset + vp_max.to_vec2() * radar_scale,
                );
                painter.rect_stroke(radar_vp_rect, 2.0, egui::Stroke::new(1.5, egui::Color32::WHITE));

                if is_hovering_minimap && ctx.input(|i| i.pointer.any_down()) {
                    if let Some(pos) = pointer_pos {
                        let target_world = (pos - radar_offset) / radar_scale;
                        self.pan = screen_rect.center().to_vec2() - target_world * self.zoom;
                    }
                }
            }

            let zen_pos = ui.max_rect().left_bottom() + egui::vec2(25.0, -45.0);
            if ui.put(egui::Rect::from_center_size(zen_pos, egui::vec2(45.0, 45.0)), egui::Button::new("🧘").rounding(25.0)).clicked() {
                self.is_zen_mode = !self.is_zen_mode;
            }
        });
    }
}