use eframe::egui;
use serde_json::Value;
use std::collections::HashMap;

#[derive(PartialEq, Clone, Copy)]
pub enum DiffStatus { Normal, Added, Removed, Modified }

pub struct Node {
    pub label: Box<str>,
    pub value: String,
    pub pos: egui::Pos2,
    pub node_type: Box<str>,
    pub matches_search: bool,
    pub collapsed: bool,
    pub visible: bool,
    pub status: DiffStatus,
    pub path: String,
    pub raw_val: Value,
    pub is_secret: bool,
}

#[derive(Default, Clone)]
pub struct FieldStats {
    pub total: usize,
    pub types: HashMap<String, usize>,
    pub null_or_empty: usize,
}

pub struct JRayPro {
    pub json_input: String,
    pub json_input_b: String,
    pub active_tab: usize,
    pub is_diff_mode: bool,
    pub search_query: String,
    pub search_results_idx: Vec<usize>,
    pub current_search_match: usize,
    pub nodes: Vec<Node>,
    pub connections: Vec<(usize, usize)>,
    pub pan: egui::Vec2,
    pub zoom: f32,
    pub status_msg: String,
    pub is_zen_mode: bool,
    pub dragged_node: Option<usize>,
    
    pub is_huge_file: bool,
    pub raw_full_json: Option<String>,
    pub raw_full_json_b: Option<String>, 
    
    pub show_code_gen: bool,
    pub code_gen_lang: usize,
    pub generated_code: String,
    pub show_profiler: bool,
    pub profiler_reports: Vec<String>,
    
    pub array_limits: HashMap<String, usize>, 
    pub loading_state: u8, 
    pub pending_path: Option<String>,
    pub decoded_payload: Option<String>,

    // ✨ RADAR: Variabili per lo Stream Live API
    pub api_url: String,
    pub api_interval: f32,
    pub is_api_live: bool,
    pub last_api_fetch: Option<std::time::Instant>,
    pub api_receiver: Option<std::sync::mpsc::Receiver<String>>,
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
            dragged_node: None,
            is_huge_file: false,
            raw_full_json: None,
            raw_full_json_b: None,
            show_code_gen: false,
            code_gen_lang: 0,
            generated_code: "".to_string(),
            show_profiler: false,
            profiler_reports: Vec::new(),
            array_limits: HashMap::new(),
            loading_state: 0,
            pending_path: None,
            decoded_payload: None,
            // ✨ RADAR: Valori di default
            api_url: "http://api.open-notify.org/iss-now.json".to_string(), // Link di test geniale
            api_interval: 2.0, // Aggiorna ogni 2 secondi
            is_api_live: false,
            last_api_fetch: None,
            api_receiver: None,
        }
    }
}