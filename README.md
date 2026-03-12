# J-RAY PRO 🚀

**The Ultimate Visual JSON Graph Editor & Data Profiler**

J-RAY PRO is a blazing-fast, visual node-based JSON editor and data analysis tool built in Rust. It transforms complex, nested JSON data into a beautiful, interactive graphical interface, making it effortless to explore, modify, analyze, and diff massive datasets.

Designed with performance and aesthetics in mind, J-RAY PRO combines the power of raw speed with a sleek, modern UI.

![J-RAY PRO UI](https://via.placeholder.com/800x450.png?text=J-RAY+PRO+Visual+Interface) <!-- Feel free to replace with an actual screenshot -->

## ✨ Features

### 🕸️ Interactive Visual Graph
Say goodbye to endless scrolling through raw text. J-RAY PRO translates JSON into a dynamic, connected node graph.
- Beautiful, color-coded badging for data types (Strings, Numbers, Booleans, Nulls, Arrays, Objects).
- Panning, zooming, and drag-and-drop node organization.
- Collapsible objects and arrays to keep your workspace clean.
- "Zen Mode" navigation for maximum focus.

### 🕵️ AI Snoop (Data Profiler)
Instantly understand any dataset with the built-in Profiler.
- Detects the structure of massive JSON files.
- Reports on data consistency (e.g., "Field 'id' is a Number in 500 objects, but is a String in 2").
- Identifies empty or null fields and calculates their exact percentage across the dataset.

### ⚖️ Spying Diff (Visual JSON Diffing)
Compare two JSON files visually side-by-side.
- Automatically highlights **Added** (Green), **Removed** (Red), and **Modified** (Yellow) nodes.
- Highlights structural differences seamlessly within the interconnected graph environment.

### 🪄 Code Maker (Type Generation)
Stop writing boilerplate code by hand. Let J-RAY PRO generate your types instantly from any JSON payload:
- **TypeScript** Interfaces (`export interface Root { ... }`)
- **Rust** Structs (`#[derive(Serialize, Deserialize)] pub struct Root { ... }`)
- **Python** Pydantic Models (`class Root(BaseModel): ... `)

### 🔐 Secret Decoder Ring (Built-in Decryption)
Working with APIs and tokens? 
- J-RAY PRO automatically detects suspected Base64 payloads and JWT tokens.
- Decode secrets directly inside the graph with a single click. No need to open external tools.

### 🔍 Next-Gen Search & Navigation
- Robust text search capabilities across the entire graph.
- Support for **JSONPath** queries (e.g., `$.users[*].id`) to pinpoint exact nodes.
- Built-in "Radar" minimap in the corner of the screen perfectly tracks your position in gigantic node networks.

### 📸 High-Res SVG Export
Need to share your architecture or payload structure with the team? Export your customized node graph directly to an SVG vector file.

### ⚡ Built for Speed & Massive Files
Engineered from the ground up in **Rust** using the `egui` framework, J-RAY PRO easily crushes I/O operations and handles massive datasets smoothly without freezing your system.

## 🛠️ Tech Stack
- **Language**: Rust
- **GUI Framework**: `egui` / `eframe`
- **Serialization**: `serde`, `serde_json`
- **Parallel Processing**: `rayon`
- **Additional Tooling**: `jsonpath-rust` (Querying), `sysinfo` (Hardware IDs at runtime), `similar` (Diffing)

## 📦 How to Build and Run

Make sure you have [Rust](https://www.rust-lang.org/tools/install) installed on your system.

```bash
# Clone the repository
git clone https://github.com/MauryDevIta/J-RAY-PRO.git
cd J-RAY-PRO

# Build and run the project
cargo run --release
```

## 📜 License & Disclaimers
J-RAY PRO includes built-in Swag & License Management. Please read the End User License Agreement (EULA) and Terms of Service before "flexing" with the PRO engines.

---
*Created and maintained with ❤️ by MauryDevIta*
