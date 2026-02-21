use crate::app::{DiffStatus, FieldStats, JRayPro, Node, LicenseTier};
use eframe::egui;
use jsonpath_rust::JsonPathQuery;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::time::Instant;

impl JRayPro {

    // ✨ X-RAY ALGORITHM: Riconosce i Token ad occhio nudo
    pub fn is_likely_secret(s: &str) -> bool {
        let clean_s = s.trim_matches('"');
        if clean_s.len() < 20 || clean_s.contains(' ') { return false; }
        clean_s.starts_with("ey") || clean_s.split('.').count() == 3
    }

    // ✨ X-RAY ALGORITHM: Decriptatore automatico
    pub fn decode_secret(s: &str) -> String {
        use base64::{Engine as _, engine::general_purpose};
        
        let clean_s = s.trim_matches('"');
        let parts: Vec<&str> = clean_s.split('.').collect();
        let target = if parts.len() == 3 { parts[1] } else { clean_s };
        
        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(target)
            .or_else(|_| general_purpose::STANDARD.decode(target))
            .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(target));
            
        match decoded {
            Ok(bytes) => {
                if let Ok(utf8) = String::from_utf8(bytes) {
                    if let Ok(val) = serde_json::from_str::<Value>(&utf8) {
                        serde_json::to_string_pretty(&val).unwrap_or(utf8)
                    } else {
                        utf8
                    }
                } else {
                    "❌ ERRORE: La decriptazione ha rivelato dati binari (non testo).".into()
                }
            },
            Err(_) => "❌ ERRORE: Chiave crittografica fallita o Base64 corrotto.".into()
        }
    }

    pub fn get_type_info(v: &Value) -> (&str, egui::Color32) {
        match v {
            Value::String(_) => ("STR", egui::Color32::from_rgb(16, 185, 129)),
            Value::Number(_) => ("NUM", egui::Color32::from_rgb(245, 158, 11)),
            Value::Bool(_) => ("BOOL", egui::Color32::from_rgb(168, 85, 247)),
            Value::Null => ("NULL", egui::Color32::from_rgb(239, 68, 68)),
            Value::Array(_) => ("ARR", egui::Color32::from_rgb(56, 189, 248)),
            _ => ("OBJ", egui::Color32::from_rgb(99, 102, 241)),
        }
    }

    pub fn format_json(&mut self) {
        if self.is_huge_file { return; }
        let target_input = if self.active_tab == 0 { &mut self.json_input } else { &mut self.json_input_b };
        if let Ok(value) = serde_json::from_str::<Value>(target_input) {
            if let Ok(pretty) = serde_json::to_string_pretty(&value) {
                *target_input = pretty;
                self.status_msg = "JSON formattato".to_string();
            }
        }
    }

    pub fn build_json_value(&self, idx: usize) -> Value {
        let node = &self.nodes[idx];
        match &*node.node_type {
            "OBJ" => {
                let mut map = serde_json::Map::new();
                for &(p, c) in &self.connections {
                    if p == idx { map.insert(self.nodes[c].label.to_string(), self.build_json_value(c)); }
                }
                Value::Object(map)
            }
            "ARR" => {
                let mut arr = Vec::new();
                let mut children: Vec<usize> = self.connections.iter().filter(|&&(p, _)| p == idx).map(|&(_, c)| c).collect();
                children.sort();
                for c in children { arr.push(self.build_json_value(c)); }
                Value::Array(arr)
            }
            _ => serde_json::from_str(&node.value).unwrap_or_else(|_| Value::String(node.value.clone())),
        }
    }

    pub fn sync_graph_to_json(&mut self) {
        if self.nodes.is_empty() || self.is_huge_file || self.is_diff_mode { return; }
        let root_val = self.build_json_value(0);
        if let Ok(pretty) = serde_json::to_string_pretty(&root_val) {
            if self.active_tab == 0 { self.json_input = pretty; } else { self.json_input_b = pretty; }
            self.status_msg = "Sincronizzazione completata!".to_string();
        }
    }

    pub fn save_file(&mut self) {
        if self.is_huge_file { return; }
        self.sync_graph_to_json();
        if let Some(p) = rfd::FileDialog::new().add_filter("JSON", &["json"]).save_file() {
            let target_input = if self.active_tab == 0 { &self.json_input } else { &self.json_input_b };
            if fs::write(p, target_input).is_ok() { self.status_msg = "💾 File salvato con successo".to_string(); }
        }
    }

    pub fn export_to_svg(&mut self) {
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

            min_x -= 100.0; min_y -= 100.0; max_x += 100.0; max_y += 100.0;
            let width = max_x - min_x; let height = max_y - min_y;

            let mut svg = String::with_capacity(self.nodes.len() * 500);
            svg.push_str(&format!("<svg viewBox=\"{} {} {} {}\" xmlns=\"http://www.w3.org/2000/svg\" style=\"background-color:#09090b; font-family: sans-serif;\">\n", min_x, min_y, width, height));

            for &(s_i, e_i) in &self.connections {
                let n1 = &self.nodes[s_i]; let n2 = &self.nodes[e_i];
                if !n1.visible || !n2.visible { continue; }
                let p1x = n1.pos.x + 220.0; let p1y = n1.pos.y + 32.5;
                let p2x = n2.pos.x; let p2y = n2.pos.y + 32.5;
                svg.push_str(&format!("<path d=\"M {},{} C {},{} {},{} {},{}\" fill=\"none\" stroke=\"#6366f1\" stroke-width=\"1.5\" />\n", p1x, p1y, p1x + 80.0, p1y, p2x - 80.0, p2y, p2x, p2y));
            }

            let escape_xml = |s: &str| -> String { s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;") };

            for n in &self.nodes {
                if !n.visible { continue; }
                let border_col = if self.is_diff_mode {
                    match n.status { DiffStatus::Added => "#22c55e", DiffStatus::Removed => "#ef4444", DiffStatus::Modified => "#eab308", DiffStatus::Normal => "#6366f1" }
                } else {
                    if &*n.node_type == "OBJ" { "#22d3ee" } else if &*n.node_type == "STACK" { "#ec4899" } else { "#6366f1" }
                };

                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"220\" height=\"65\" rx=\"8\" fill=\"#18181b\" stroke=\"{}\" stroke-width=\"1.5\" />\n", n.pos.x, n.pos.y, border_col));
                svg.push_str(&format!("<path d=\"M {},{} a 8 8 0 0 1 8 -8 h 204 a 8 8 0 0 1 8 8 v 17 h -220 z\" fill=\"#000000\" fill-opacity=\"0.4\" />\n", n.pos.x, n.pos.y + 8.0));
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" fill=\"#a5b4fc\" font-size=\"12\" font-weight=\"bold\" text-anchor=\"middle\">{}</text>\n", n.pos.x + 110.0, n.pos.y + 17.0, escape_xml(&n.label)));
                if !n.value.is_empty() { svg.push_str(&format!("<text x=\"{}\" y=\"{}\" fill=\"#9ca3af\" font-size=\"11\" font-family=\"monospace\" text-anchor=\"middle\">{}</text>\n", n.pos.x + 110.0, n.pos.y + 44.5, escape_xml(&n.value))); }
                let badge_col = match &*n.node_type { "STR" => "#10b981", "NUM" => "#f59e0b", "BOOL" => "#a855f7", "NULL" => "#ef4444", "ARR" => "#38bdf8", "STACK" => "#ec4899", _ => "#6366f1" };
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"34\" height=\"15\" rx=\"2\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\" />\n", n.pos.x + 13.0, n.pos.y + 41.5, badge_col));
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"9\" font-weight=\"bold\" text-anchor=\"middle\">{}</text>\n", n.pos.x + 30.0, n.pos.y + 52.0, badge_col, n.node_type));
            }
            svg.push_str("</svg>");
            if fs::write(path, svg).is_ok() { self.status_msg = format!("📸 SVG generato in {:?}", start.elapsed()); }
        }
    }

    pub fn capitalize(s: &str) -> String {
        let mut c = s.chars();
        match c.next() { None => String::new(), Some(f) => f.to_uppercase().collect::<String>() + c.as_str() }
    }

    pub fn generate_types(&mut self) {
        if self.nodes.is_empty() { self.generated_code = "// Nessun dato presente nel grafo.".to_string(); return; }
        
        let target_text = if self.active_tab == 0 {
            if self.json_input.starts_with("/* ⚠️") && self.raw_full_json.is_some() { self.raw_full_json.as_ref().unwrap() } else { &self.json_input }
        } else {
            if self.json_input_b.starts_with("/* ⚠️") && self.raw_full_json_b.is_some() { self.raw_full_json_b.as_ref().unwrap() } else { &self.json_input_b }
        };

        let root_val = serde_json::from_str(target_text).unwrap_or(Value::Null);
        let mut output = String::new();
        match self.code_gen_lang {
            0 => { Self::gen_ts(&root_val, "Root", &mut output); },
            1 => { output.push_str("use serde::{Serialize, Deserialize};\n\n"); Self::gen_rust(&root_val, "Root", &mut output); },
            2 => { output.push_str("from typing import List, Any, Optional\nfrom pydantic import BaseModel\n\n"); Self::gen_py(&root_val, "Root", &mut output); },
            _ => {}
        }
        self.generated_code = output;
    }

    pub fn gen_ts(value: &Value, name: &str, output: &mut String) -> String {
        match value {
            Value::Object(map) => {
                let mut fields = String::new();
                for (k, v) in map { fields.push_str(&format!("  {}: {};\n", k, Self::gen_ts(v, &Self::capitalize(k), output))); }
                output.push_str(&format!("export interface {} {{\n{}}}\n\n", name, fields)); name.to_string()
            }
            Value::Array(arr) => { if arr.is_empty() { "any[]".to_string() } else { format!("{}[]", Self::gen_ts(&arr[0], &format!("{}Item", name), output)) } }
            Value::String(_) => "string".to_string(), Value::Number(_) => "number".to_string(), Value::Bool(_) => "boolean".to_string(), Value::Null => "any".to_string(),
        }
    }

    pub fn gen_rust(value: &Value, name: &str, output: &mut String) -> String {
        match value {
            Value::Object(map) => {
                let mut fields = String::new();
                for (k, v) in map { fields.push_str(&format!("    pub {}: {},\n", k, Self::gen_rust(v, &Self::capitalize(k), output))); }
                output.push_str(&format!("#[derive(Debug, Serialize, Deserialize)]\npub struct {} {{\n{}}}\n\n", name, fields)); name.to_string()
            }
            Value::Array(arr) => { if arr.is_empty() { "Vec<Value>".to_string() } else { format!("Vec<{}>", Self::gen_rust(&arr[0], &format!("{}Item", name), output)) } }
            Value::String(_) => "String".to_string(), Value::Number(n) => if n.is_f64() { "f64".to_string() } else { "i64".to_string() }, Value::Bool(_) => "bool".to_string(), Value::Null => "Option<Value>".to_string(),
        }
    }

    pub fn gen_py(value: &Value, name: &str, output: &mut String) -> String {
        match value {
            Value::Object(map) => {
                let mut fields = String::new();
                for (k, v) in map { fields.push_str(&format!("    {}: {}\n", k, Self::gen_py(v, &Self::capitalize(k), output))); }
                if fields.is_empty() { fields = "    pass\n".to_string(); }
                output.push_str(&format!("class {}(BaseModel):\n{}\n\n", name, fields)); name.to_string()
            }
            Value::Array(arr) => { if arr.is_empty() { "List[Any]".to_string() } else { format!("List[{}]", Self::gen_py(&arr[0], &format!("{}Item", name), output)) } }
            Value::String(_) => "str".to_string(), Value::Number(n) => if n.is_f64() { "float".to_string() } else { "int".to_string() }, Value::Bool(_) => "bool".to_string(), Value::Null => "Optional[Any]".to_string(),
        }
    }

    pub fn run_profiler(&mut self) {
        let start = Instant::now();
        let target_text = if self.active_tab == 0 {
            if self.json_input.starts_with("/* ⚠️") && self.raw_full_json.is_some() { self.raw_full_json.as_ref().unwrap() } else { &self.json_input }
        } else {
            if self.json_input_b.starts_with("/* ⚠️") && self.raw_full_json_b.is_some() { self.raw_full_json_b.as_ref().unwrap() } else { &self.json_input_b }
        };

        let mut stats: HashMap<String, FieldStats> = HashMap::new();
        if let Ok(value) = serde_json::from_str::<Value>(target_text) {
            self.profile_value(&value, &mut stats);
            let mut reports = Vec::new();
            for (key, stat) in stats {
                if stat.total == 0 { continue; }
                if stat.types.len() > 1 {
                    let mut sorted_types: Vec<_> = stat.types.iter().collect();
                    sorted_types.sort_by(|a, b| b.1.cmp(a.1));
                    let mut type_strs = Vec::new();
                    for (t, count) in sorted_types { type_strs.push(format!("un {} in {} oggetti", t, count)); }
                    reports.push(format!("⚠️ ATTENZIONE: Il campo '{}' è {}", key, type_strs.join(", ma è ")));
                }
                if stat.null_or_empty > 0 {
                    let perc = (stat.null_or_empty as f64 / stat.total as f64) * 100.0;
                    if perc >= 1.0 { reports.push(format!("📉 Il campo '{}' è vuoto/null nel {:.1}% dei cases ({} su {} totali).", key, perc, stat.null_or_empty, stat.total)); }
                }
            }
            if reports.is_empty() { reports.push("✅ Nessuna anomalia di struttura rilevata! Il dataset è pulitissimo.".to_string()); } 
            else { reports.sort_by(|a, b| b.starts_with("⚠️").cmp(&a.starts_with("⚠️"))); }
            self.profiler_reports = reports;
            self.status_msg = format!("📊 Profiling AI completato in {:?}", start.elapsed());
            self.show_profiler = true;
        } else { self.status_msg = "❌ Impossibile avviare il Profiler: JSON non valido".to_string(); }
    }

    pub fn profile_value(&self, val: &Value, stats: &mut HashMap<String, FieldStats>) {
        match val {
            Value::Object(map) => {
                for (k, v) in map {
                    let entry = stats.entry(k.clone()).or_insert(FieldStats::default());
                    entry.total += 1;
                    let type_name = match v {
                        Value::String(s) => { if s.trim().is_empty() { entry.null_or_empty += 1; } "Testo (String)" },
                        Value::Number(_) => "Numero (Number)", Value::Bool(_) => "Booleano (Bool)",
                        Value::Null => { entry.null_or_empty += 1; "Vuoto (Null)" },
                        Value::Array(arr) => { if arr.is_empty() { entry.null_or_empty += 1; } "Lista (Array)" },
                        Value::Object(_) => "Oggetto (Object)",
                    };
                    *entry.types.entry(type_name.to_string()).or_insert(0) += 1;
                    self.profile_value(v, stats);
                }
            }
            Value::Array(arr) => { for v in arr { self.profile_value(v, stats); } }
            _ => {}
        }
    }

    pub fn run_diff(&mut self) {
        let start = Instant::now();
        self.nodes.clear(); self.connections.clear(); self.is_diff_mode = true;
        
        let text_a = if self.json_input.starts_with("/* ⚠️") && self.raw_full_json.is_some() { self.raw_full_json.as_ref().unwrap() } else { &self.json_input };
        let text_b = if self.json_input_b.starts_with("/* ⚠️") && self.raw_full_json_b.is_some() { self.raw_full_json_b.as_ref().unwrap() } else { &self.json_input_b };

        let v1 = serde_json::from_str::<Value>(text_a).unwrap_or(Value::Null);
        let v2 = serde_json::from_str::<Value>(text_b).unwrap_or(Value::Null);

        if !self.json_input.starts_with("/* ⚠️") { if let Ok(p1) = serde_json::to_string_pretty(&v1) { self.json_input = p1; } }
        if !self.json_input_b.starts_with("/* ⚠️") { if let Ok(p2) = serde_json::to_string_pretty(&v2) { self.json_input_b = p2; } }

        let mut s_idx: f32 = 0.0;
        self.diff_traverse(Some(&v1), Some(&v2), "root".to_string(), None, 0, &mut s_idx, "$".to_string());
        self.status_msg = format!("⚖️ Diff calcolato in {:?}", start.elapsed());
    }

    pub fn diff_traverse(&mut self, v1: Option<&Value>, v2: Option<&Value>, label: String, p_idx: Option<usize>, d: usize, s_idx: &mut f32, current_path: String) {
        if self.nodes.len() > 150000 { return; }
        let status = match (v1, v2) {
            (Some(a), Some(b)) if a == b => DiffStatus::Normal, (Some(_), Some(_)) => DiffStatus::Modified,
            (None, Some(_)) => DiffStatus::Added, (Some(_), None) => DiffStatus::Removed, (None, None) => return,
        };
        let val_to_show = v2.or(v1).unwrap(); 
        let (t_label, _) = Self::get_type_info(val_to_show);
        let n_idx = self.nodes.len();
        let val_str = if val_to_show.is_object() || val_to_show.is_array() { "".to_string() } else { val_to_show.to_string() };

        let is_sec = if t_label == "STR" { Self::is_likely_secret(&val_str) } else { false };

        self.nodes.push(Node {
            label: label.into_boxed_str(), value: val_str, node_type: t_label.into(),
            pos: egui::pos2(d as f32 * 350.0, *s_idx * 120.0), matches_search: true,
            collapsed: false, visible: true, status, path: current_path.clone(), raw_val: val_to_show.clone(),
            is_secret: is_sec,
        });
        if let Some(pi) = p_idx { self.connections.push((pi, n_idx)); }

        let obj1 = v1.and_then(|v| v.as_object()); let obj2 = v2.and_then(|v| v.as_object());
        if obj1.is_some() || obj2.is_some() {
            let mut keys = std::collections::HashSet::new();
            if let Some(o) = obj1 { keys.extend(o.keys()); } if let Some(o) = obj2 { keys.extend(o.keys()); }
            let mut sorted_keys: Vec<_> = keys.into_iter().collect(); sorted_keys.sort();
            for k in sorted_keys {
                let p = if current_path == "$" { format!("$.{}", k) } else { format!("{}.{}", current_path, k) };
                self.diff_traverse(obj1.and_then(|o| o.get(k)), obj2.and_then(|o| o.get(k)), k.clone(), Some(n_idx), d + 1, s_idx, p);
                *s_idx += 1.0;
            }
        } else {
            let arr1 = v1.and_then(|v| v.as_array()); let arr2 = v2.and_then(|v| v.as_array());
            if arr1.is_some() || arr2.is_some() {
                let len1 = arr1.map_or(0, |a| a.len());
                let len2 = arr2.map_or(0, |a| a.len());
                let max_len = std::cmp::max(len1, len2);
                let limit = self.array_limits.get(&current_path).copied().unwrap_or(5);

                if max_len > limit {
                    for i in 0..limit {
                        let p = format!("{}[{}]", current_path, i);
                        let child_v1 = arr1.and_then(|a| a.get(i));
                        let child_v2 = arr2.and_then(|a| a.get(i));
                        self.diff_traverse(child_v1, child_v2, format!("[{}]", i), Some(n_idx), d + 1, s_idx, p);
                        *s_idx += 1.0;
                    }
                    
                    let rem_len = max_len - limit;
                    let stack_label = format!("[{}..{}]", limit, max_len - 1);
                    let stack_val = format!("🃏 Clicca [+50] per espandere ({} rimasti)", rem_len);
                    let stack_status = if len1 != len2 { DiffStatus::Modified } else { DiffStatus::Normal };
                    
                    self.nodes.push(Node {
                        label: stack_label.into_boxed_str(), value: stack_val, node_type: "STACK".into(),
                        pos: egui::pos2((d + 1) as f32 * 350.0, *s_idx * 120.0), matches_search: true, collapsed: false, visible: true,
                        status: stack_status, path: current_path.clone(), raw_val: Value::Null, is_secret: false,
                    });
                    self.connections.push((n_idx, self.nodes.len() - 1));
                    *s_idx += 1.0;
                    
                } else {
                    for i in 0..max_len {
                        let p = format!("{}[{}]", current_path, i);
                        let child_v1 = arr1.and_then(|a| a.get(i));
                        let child_v2 = arr2.and_then(|a| a.get(i));
                        self.diff_traverse(child_v1, child_v2, format!("[{}]", i), Some(n_idx), d + 1, s_idx, p);
                        *s_idx += 1.0;
                    }
                }
            }
        }
    }

    pub fn generate_graph_from_string(&mut self, text: &str) {
        let start = Instant::now();
        self.nodes.clear(); self.connections.clear(); self.is_diff_mode = false; 
        if let Ok(v) = serde_json::from_str::<Value>(text) {
            let mut s_idx: f32 = 0.0;
            self.traverse(&v, "root".to_string(), None, 0, &mut s_idx, "$".to_string());
            let dummy_center = egui::pos2(0.0, 0.0);
            self.apply_search(dummy_center);
            self.status_msg = format!("Nitro: {} nodi in {:?}", self.nodes.len(), start.elapsed());
        } else { self.status_msg = "ERRORE: JSON non valido".to_string(); }
    }

    pub fn traverse(&mut self, value: &Value, label: String, p_idx: Option<usize>, d: usize, s_idx: &mut f32, current_path: String) {
        if self.nodes.len() > 150000 { return; } 
        let (t_label, _) = Self::get_type_info(value);
        let n_idx = self.nodes.len();
        let val_str = if value.is_object() || value.is_array() { "".to_string() } else { value.to_string() };
        
        let is_sec = if t_label == "STR" { Self::is_likely_secret(&val_str) } else { false };

        self.nodes.push(Node {
            label: label.into_boxed_str(), value: val_str, node_type: t_label.into(),
            pos: egui::pos2(d as f32 * 350.0, *s_idx * 120.0), matches_search: true,
            collapsed: false, visible: true, status: DiffStatus::Normal, path: current_path.clone(), raw_val: value.clone(), 
            is_secret: is_sec,
        });
        if let Some(pi) = p_idx { self.connections.push((pi, n_idx)); }

        if let Some(obj) = value.as_object() {
            for (k, v) in obj { 
                let p = if current_path == "$" { format!("$.{}", k) } else { format!("{}.{}", current_path, k) };
                self.traverse(v, k.clone(), Some(n_idx), d + 1, s_idx, p); *s_idx += 1.0; 
            }
        } else if let Some(arr) = value.as_array() {
            let limit = self.array_limits.get(&current_path).copied().unwrap_or(5);
            if arr.len() > limit {
                for (i, v) in arr.iter().take(limit).enumerate() { 
                    let p = format!("{}[{}]", current_path, i);
                    self.traverse(v, format!("[{}]", i), Some(n_idx), d + 1, s_idx, p); *s_idx += 1.0; 
                }
                
                let rem_len = arr.len() - limit;
                let stack_label = format!("[{}..{}]", limit, arr.len() - 1);
                let stack_val = format!("🃏 Clicca [+50] per espandere ({} rimasti)", rem_len);
                let remaining_vals: Vec<Value> = arr.iter().skip(limit).cloned().collect();
                
                self.nodes.push(Node {
                    label: stack_label.into_boxed_str(), value: stack_val, node_type: "STACK".into(),
                    pos: egui::pos2((d + 1) as f32 * 350.0, *s_idx * 120.0), matches_search: true, collapsed: false, visible: true, status: DiffStatus::Normal,
                    path: current_path.clone(), raw_val: Value::Array(remaining_vals), is_secret: false,
                });
                self.connections.push((n_idx, self.nodes.len() - 1));
                *s_idx += 1.0;

            } else {
                for (i, v) in arr.iter().enumerate() { 
                    let p = format!("{}[{}]", current_path, i);
                    self.traverse(v, format!("[{}]", i), Some(n_idx), d + 1, s_idx, p); *s_idx += 1.0; 
                }
            }
        }
    }

    pub fn update_visibility(&mut self) {
        for n in &mut self.nodes { n.visible = true; } 
        for &(p, c) in &self.connections { if !self.nodes[p].visible || self.nodes[p].collapsed { self.nodes[c].visible = false; } }
    }

    pub fn apply_search(&mut self, view_center: egui::Pos2) {
        let query = self.search_query.trim().to_string();
        self.search_results_idx.clear(); self.current_search_match = 0;
        
        if query.is_empty() {
            for node in &mut self.nodes { node.matches_search = true; node.visible = true; }
            self.update_visibility(); return;
        }

        for node in &mut self.nodes { node.collapsed = false; }
        self.update_visibility();

        if query.starts_with("$.") || query.starts_with("$[") {
            let target_text = if self.active_tab == 0 {
                if self.json_input.starts_with("/* ⚠️") && self.raw_full_json.is_some() { self.raw_full_json.as_ref().unwrap() } else { &self.json_input }
            } else {
                if self.json_input_b.starts_with("/* ⚠️") && self.raw_full_json_b.is_some() { self.raw_full_json_b.as_ref().unwrap() } else { &self.json_input_b }
            };

            if let Ok(value) = serde_json::from_str::<Value>(target_text) {
                match value.path(&query) {
                    Ok(results) => {
                        if let Some(arr) = results.as_array() {
                            if arr.is_empty() {
                                for node in &mut self.nodes { node.matches_search = false; }
                                self.status_msg = "🔍 Nessun risultato".to_string();
                            } else {
                                for (i, node) in self.nodes.iter_mut().enumerate() {
                                    node.matches_search = arr.contains(&node.raw_val);
                                    if &*node.node_type == "STACK" {
                                        if let Some(hidden_arr) = node.raw_val.as_array() {
                                            for hidden_val in hidden_arr {
                                                if arr.contains(hidden_val) { node.matches_search = true; break; }
                                            }
                                        }
                                    }
                                    if node.matches_search && node.visible { self.search_results_idx.push(i); }
                                }
                                self.status_msg = format!("🎯 {} match esatti", self.search_results_idx.len());
                            }
                        }
                    },
                    Err(_) => {
                        for node in &mut self.nodes { node.matches_search = false; }
                        self.status_msg = "❌ JSONPath non valida".to_string();
                    }
                }
            }
        } else {
            let q_lower = query.to_lowercase();
            for (i, node) in self.nodes.iter_mut().enumerate() {
                node.matches_search = node.label.to_lowercase().contains(&q_lower) || node.value.to_lowercase().contains(&q_lower);
                if node.matches_search && node.visible { self.search_results_idx.push(i); }
            }
            self.status_msg = format!("🔍 Trovati {} risultati", self.search_results_idx.len());
        }

        if !self.search_results_idx.is_empty() { self.focus_current_match(view_center); }
    }

    pub fn focus_current_match(&mut self, view_center: egui::Pos2) {
        if self.search_results_idx.is_empty() { return; }
        let target_idx = self.search_results_idx[self.current_search_match];
        let target_pos = self.nodes[target_idx].pos;
        self.zoom = 1.0;
        self.pan = view_center.to_vec2() - (target_pos + egui::vec2(110.0, 32.5)).to_vec2() * self.zoom;
    }

    pub fn next_search_match(&mut self, view_center: egui::Pos2) {
        if self.search_results_idx.is_empty() { return; }
        self.current_search_match = (self.current_search_match + 1) % self.search_results_idx.len();
        self.focus_current_match(view_center);
    }

    pub fn prev_search_match(&mut self, view_center: egui::Pos2) {
        if self.search_results_idx.is_empty() { return; }
        if self.current_search_match == 0 { self.current_search_match = self.search_results_idx.len() - 1; } 
        else { self.current_search_match -= 1; }
        self.focus_current_match(view_center);
    }

    pub fn open_file(&mut self, is_file_b: bool) {
        if let Some(p) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
            if let Ok(metadata) = fs::metadata(&p) {
                let size_mb = metadata.len() as f64 / 1_048_576.0;
                if let Ok(full_text) = fs::read_to_string(p) {

                    let preview_text = if size_mb > 5.0 {
                        self.is_huge_file = true;
                        if !is_file_b { self.generate_graph_from_string(&full_text); }
                        format!("/* ⚠️ FILE ENORME: {:.1} MB ⚠️\n* Sincronizzazione disabilitata.\n*/\n\n{}", size_mb, full_text.chars().take(10000).collect::<String>())
                    } else {
                        self.is_huge_file = false; full_text.clone()
                    };

                    if is_file_b { 
                        self.raw_full_json_b = Some(full_text);
                        self.json_input_b = preview_text; 
                        self.active_tab = 1; 
                        self.status_msg = "File B caricato".to_string(); 
                    } else { 
                        self.raw_full_json = Some(full_text);
                        self.json_input = preview_text; 
                        self.active_tab = 0; 
                        self.status_msg = "File A caricato".to_string(); 
                    }
                }
            }
        }
    }

    // 🛡️ DIFESA 3: HARDWARE FINGERPRINTING ESTREMO
    pub fn get_advanced_hardware_id() -> String {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_all();
        
        let cpu = sys.cpus().first().map(|c| c.brand()).unwrap_or("UnknownCPU");
        let mem = sys.total_memory(); 
        let host = System::host_name().unwrap_or("UnknownHost".to_string());
        let m_uid = machine_uid::get().unwrap_or("UnknownUID".to_string());
        
        let raw_id = format!("{}-{}-{}-{}", m_uid, cpu, mem, host);
        
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(raw_id);
        format!("{:x}", hasher.finalize())
    }

    // 🛡️ DIFESA 1: STRING OBFUSCATION + FIRMA DIGITALE
    pub fn generate_security_signature(data: &str) -> String {
        use sha2::{Sha256, Digest};
        
        // Risolto Errore E0716 usando .to_string() per allocare la stringa decriptata
        let secret_salt = obfstr::obfstr!("JRay_Pr0_Sup3rS3cr3t_K3y_2026!@#_N0_Cr4ck").to_string(); 
        
        let mut hasher = Sha256::new();
        hasher.update(format!("{}|{}", data, secret_salt));
        format!("{:x}", hasher.finalize())
    }

    // ✨ SISTEMA DI LICENZE ULTRA-BLINDATO
    pub fn init_license_system() -> (LicenseTier, i64, String) {
        use chrono::{DateTime, Utc};
        use crate::app::LicenseTier;

        let m_id = Self::get_advanced_hardware_id();
        let proj_dirs = directories::ProjectDirs::from("com", "jray", "jraypro").unwrap();
        let config_dir = proj_dirs.config_dir();
        
        let license_file = config_dir.join("license.key");
        if let Ok(content) = std::fs::read_to_string(&license_file) {
            let parts: Vec<&str> = content.split('|').collect();
            
            if parts.len() == 4 {
                let saved_tier = parts[0];
                let saved_key = parts[1];
                let saved_m_id = parts[2];
                let saved_sig = parts[3];
                
                let expected_payload = format!("{}|{}|{}", saved_tier, saved_key, saved_m_id);
                let expected_sig = Self::generate_security_signature(&expected_payload);

                // 🛡️ DIFESA 2 (ENTANGLEMENT LEGGERO)
                if saved_sig == expected_sig && saved_m_id == m_id {
                    if saved_tier == "PRO" { return (LicenseTier::Pro, 0, m_id); }
                    if saved_tier == "PERSONAL" { return (LicenseTier::Personal, 0, m_id); }
                } else {
                    println!("🚨 MANOMISSIONE O CLONAZIONE RILEVATA! BOOM!");
                }
            }
        }

        // TRIAL SYSTEM BLINDATO
        let trial_file = config_dir.join("vault.bin");

        if !trial_file.exists() {
            std::fs::create_dir_all(config_dir).ok();
            let now = Utc::now().to_rfc3339();
            
            let trial_payload = format!("{}|{}", now, m_id);
            let trial_sig = Self::generate_security_signature(&trial_payload);
            std::fs::write(&trial_file, format!("{}|{}", trial_payload, trial_sig)).ok();
            
            return (LicenseTier::Trial, 14, m_id);
        } else {
            if let Ok(content) = std::fs::read_to_string(&trial_file) {
                let parts: Vec<&str> = content.split('|').collect();
                
                if parts.len() == 3 {
                    let expected_payload = format!("{}|{}", parts[0], parts[1]);
                    let expected_sig = Self::generate_security_signature(&expected_payload);

                    if parts[2] == expected_sig && parts[1] == m_id {
                        if let Ok(first_run) = DateTime::parse_from_rfc3339(parts[0]) {
                            let elapsed = Utc::now().signed_duration_since(first_run.with_timezone(&Utc));
                            let remaining = 14 - elapsed.num_days();
                            if remaining > 0 { return (LicenseTier::Trial, remaining, m_id); }
                        }
                    }
                }
            }
        }
        
        (LicenseTier::Expired, 0, m_id)
    }

    // 🛡️ MOTORE DI ATTIVAZIONE: Dialoga con Lemon Squeezy
    pub fn activate_license_online(&mut self) {
        #[derive(serde::Deserialize, Debug)]
        struct LemonMeta {
            variant_name: String,
        }
        #[derive(serde::Deserialize, Debug)]
        struct LemonResponse {
            activated: bool,
            error: Option<String>,
            meta: Option<LemonMeta>,
        }

        let key = self.license_key.trim().to_string();
        if key.is_empty() {
            self.status_msg = "❌ Inserisci una chiave!".to_string();
            return;
        }

        self.status_msg = "📡 Verificando...".to_string();

        let client = reqwest::blocking::Client::new();
        
        let body = serde_json::json!({
            "license_key": key,
            "instance_name": self.machine_id
        });

        let res = client.post("https://api.lemonsqueezy.com/v1/licenses/activate")
            .json(&body) 
            .send();

        match res {
            Ok(response) => {
                if !response.status().is_success() {
                    println!("Errore Server: {}", response.status());
                }

                match response.json::<LemonResponse>() {
                    Ok(data) => {
                        if data.activated {
                            let variant = data.meta.map(|m| m.variant_name).unwrap_or_else(|| "Personal".to_string());
                            let is_pro = variant.to_lowercase().contains("pro");
                            
                            self.license_tier = if is_pro { crate::app::LicenseTier::Pro } else { crate::app::LicenseTier::Personal };
                            
                            // Salvataggio su file con FIRMA DIGITALE
                            if let Some(proj_dirs) = directories::ProjectDirs::from("com", "jray", "jraypro") {
                                let config_dir = proj_dirs.config_dir();
                                let _ = std::fs::create_dir_all(config_dir);
                                let path = config_dir.join("license.key");
                                
                                let prefix = if is_pro { "PRO" } else { "PERSONAL" };
                                
                                // Costruiamo payload + Firma Anti-Manomissione
                                let payload = format!("{}|{}|{}", prefix, key, self.machine_id);
                                let signature = Self::generate_security_signature(&payload);
                                
                                let final_save = format!("{}|{}", payload, signature);
                                let _ = std::fs::write(path, final_save);
                            }
                            
                            self.status_msg = format!("✅ Attivato: {}", variant);
                        } else {
                            let err_msg = data.error.unwrap_or_else(|| "Chiave non valida".to_string());
                            self.status_msg = format!("❌ {}", err_msg);
                        }
                    },
                    Err(e) => {
                        self.status_msg = "❌ Errore formato risposta".to_string();
                        println!("Dettaglio errore JSON: {:?}", e);
                    }
                }
            },
            Err(e) => {
                self.status_msg = "❌ Errore connessione".to_string();
                println!("Errore Reqwest: {:?}", e);
            },
        }
    }
}