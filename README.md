# J-RAY PRO 🚀

**The Ultimate Visual JSON Graph Editor & Data Profiler**

J-RAY PRO is a blazing-fast, visual node-based JSON editor and data analysis tool built in Rust. It transforms complex, nested JSON data into a beautiful, interactive graphical interface, making it effortless to explore, modify, analyze, and diff massive datasets.

Designed with performance and aesthetics in mind, J-RAY PRO combines the power of raw speed with a sleek, modern UI.

![J-RAY PRO UI](https://via.placeholder.com/800x450.png?text=J-RAY+PRO+Visual+Interface) 

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

---

## � Download & Installation

You can download the latest pre-compiled, standalone executables for your operating system from the [Releases](../../releases) page. 

*Note: Since J-RAY PRO is an independent developer tool, the executables are currently unsigned. Your OS might show a standard security warning on the first run. Here is how to run it safely:*

### 🪟 Windows
1. Download the `J-RAY-PRO-Windows.exe` file.
2. Double-click the `.exe` to run it.
3. **SmartScreen Bypass**: If Windows Defender SmartScreen prevents the app from starting, click **"More info"** (Ulteriori informazioni) and then click **"Run anyway"** (Esegui comunque).

### 🍏 macOS (Apple Silicon)
1. Download and extract the `J-RAY-PRO-Mac-Silicon` file.
2. **Gatekeeper Bypass**: Because the app is not signed with an Apple Developer Certificate, double-clicking it might show an "App is damaged or from an unidentified developer" error.
3. To open it: **Right-click** (or Control-click) the app icon and select **Open**. A new dialog will appear asking if you are sure; click **Open** again. 
*(Alternatively, you can allow it from `System Settings > Privacy & Security` -> "Open Anyway").*

### � Linux (Ubuntu/Debian)
1. Download the `J-RAY-PRO-Linux` binary.
2. Open your terminal and grant execution permissions to the file:
   ```bash
   chmod +x J-RAY-PRO-Linux
   ```
3. Run the application:
   ```bash
   ./J-RAY-PRO-Linux
   ```
   *(Note: Ensure you have standard graphic libraries installed, like libxcb, libxkbcommon, and fontconfig).*

## 🛠️ Build from Source (For Developers)
If you prefer to compile the app yourself, make sure you have Rust installed on your system.

```bash
# Clone the repository
git clone https://github.com/MauryDevIta/J-RAY-PRO.git
cd J-RAY-PRO

# Build and run the project (Release mode is highly recommended for massive files)
cargo run --release
```

## 📜 License & Disclaimers
J-RAY PRO includes built-in Swag & License Management. Please read the End User License Agreement (EULA) and Terms of Service before "flexing" with the PRO engines.

---
*Created and maintained with ❤️ by MauryDevIta*
