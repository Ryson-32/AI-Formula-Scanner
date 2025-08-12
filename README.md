<!-- markdownlint-disable MD033 MD041 -->
<div align="center">
  <h1>AI Formula Scanner </h1>
  <p>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache%202.0-blue.svg" alt="License"></a>
    <img src="https://img.shields.io/badge/platform-Windows-lightgrey.svg" alt="Platform: Windows">
    <img src="https://img.shields.io/badge/version-0.1.0-green.svg" alt="Version 0.1.0">
  </p>
  <p><strong><a href="README.md">English</a> | <a href="README-CN.md">中文</a></strong></p>
</div>
<!-- markdownlint-enable MD033 MD041 -->

AI Formula Scanner is a desktop AI formula recognition, analysis, and management tool. It supports region screenshots or image imports, one-click LaTeX extraction, and provides structured intelligent analysis and verification (with confidence scores). Built-in features include history and favorites, original image comparison, hotkeys, and bilingual interface.

## Screenshots 🖼️

<img width="1282" height="832" alt="image" src="https://github.com/user-attachments/assets/cd163948-e901-4b63-9db2-1f94042f8afc" />
<img width="1282" height="832" alt="image" src="https://github.com/user-attachments/assets/4364d52c-bdaf-42da-8853-0107b01eb238" />
<img width="1282" height="832" alt="image" src="https://github.com/user-attachments/assets/2d1a739e-c927-4a88-aabe-271955f04c9e" />

## How It Works ⚙️

### Three-Stage Recognition Pipeline

The application uses 3 LLM calls for comprehensive analysis:

1. **LaTeX Extraction**: Extract LaTeX text from image only
2. **Intelligent Analysis**: Concurrent with the first call. Generate title, summary, variable table, glossary, and suggestions based on the image
3. **Verification & Confidence**: Starts after the first call returns LaTeX. Compare "extracted LaTeX + original image" to produce status (ok/warning/error), issue list, coverage, and confidence score (0-100)

### Processing Strategy

- **Concurrent Execution**: 1st call (LaTeX) and 2nd call (analysis) are triggered concurrently
- **Sequential Verification**: 3rd call (verification) starts after obtaining LaTeX
- **Real-time Feedback**: Recognition process is pushed to the interface with stage indicators, failed stages can be retried individually
- **Local Storage**: Recognition records (including original image path, LaTeX, analysis results, and confidence) are saved locally, supporting search, sorting, favorites, and detail drawer view
- **Model Calls**: Recognition relies on online LLM services (such as Gemini). First-time use requires entering a valid API Key in "Settings" and verifying connection through "Test"

## Notes 📝

### Model Recommendations 🤖

Note: Currently only supports Google Gemini API. ⚠️

**Recommended model choices and typical processing times** (reference values, depending on network and image complexity):

- **Gemini-2.5-flash**: Optimal balance of accuracy and speed. Common formulas take about 10s, complex scenarios (including verification) overall experience about 20s
- **Gemini-2.5-flash-lite**: Faster and more economical, but slightly less accurate on complex formulas, can be used as a lightweight alternative

## Usage Workflow 🧭

### 1. Initial Setup

- Open the app → Go to "Settings" page
- Enter API Key, click "Test" to ensure connectivity
- Adjust as needed: language (Chinese/English), screenshot hotkey, formula rendering engine (MathJax/KaTeX), and default LaTeX wrapper format

### 2. Start Recognition (choose one)

- **Screenshot Recognition**: Click "Screenshot Recognition" or use hotkey `Ctrl + Shift + A` to drag a box, release to start processing
- **Import Image**: Click "Import Image", select PNG/JPG/JPEG files to recognize

### 3. View and Edit

- **Progress Indicator**: Shows LaTeX → Analysis → Verification in sequence (can "retry" any stage)
- **Basic Page**: Preview formula, copy or edit LaTeX, expand original image comparison
- **Analysis Page**: View summary, variables (including symbols/units), glossary, suggestions, and verification report with confidence score

### 4. History and Favorites

- Recognition results are automatically saved to database
- In "History/Favorites" you can search, sort, view details (drawer style), support favorites and deletion

## Deployment Instructions 🛠️

### Development Environment Setup 🧰

#### Prerequisites 📦

- Node.js 18+
- Rust 1.70+
- Git

#### Clone Project ⬇️

```bash
git clone https://github.com/AI-Formula-Scanner/AI-Formula-Scanner.git
cd AI-Formula-Scanner
```

#### Install Dependencies 📥

```bash
# Install frontend dependencies
npm install

# Install Tauri CLI (if not already installed)
npm install -g @tauri-apps/cli
```

#### Run in Development Mode ▶️

```bash
# Start development server
npm run tauri dev
```

## Known Issues 🐞

- Performance issues when there are many history records
- Current version displays "analysis" results only after verification results are returned

## License 📄

This project is released under the Apache License 2.0. You are free to use, copy, modify, and distribute this software under the terms of the license, while receiving explicit patent grants and contributor patent grants.
