<!-- markdownlint-disable MD033 MD041 -->
<div align="center">
  <h1>AI 公式扫描器 </h1>
  <p>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache%202.0-blue.svg" alt="许可证"></a>
    <img src="https://img.shields.io/badge/platform-Windows-lightgrey.svg" alt="平台：Windows">
    <img src="https://img.shields.io/badge/version-0.1.0-green.svg" alt="版本 0.1.0">
  </p>
  <p><strong><a href="README.md">English</a> | <a href="README-CN.md">中文</a></strong></p>
</div>
<!-- markdownlint-enable MD033 MD041 -->

AI Formula Scanner 是一款桌面端 AI 公式识别、分析与管理工具。支持区域截图或导入图片，一键提取 LaTeX，并给出结构化的智能分析与核查（含置信度）。内置历史记录与收藏、原图对照、快捷键与中英界面等特性。

## 软件截图 🖼️

![主界面](https://github.com/user-attachments/assets/343a6ad2-44bc-4188-b215-50c885bdf72c)

![分析界面](https://github.com/user-attachments/assets/05b17536-4465-404f-a59a-fa4de6a2718c)

## 软件原理 ⚙️

### 三阶段识别流水线

应用程序使用 3 次 LLM 调用进行全面分析：

1. **LaTeX 提取**：仅从图片提取 LaTeX 文本
2. **智能分析**：与第一次并发。基于图片生成标题、简介、变量表、术语表与建议
3. **核查与置信度**：在第一次调用返回LaTeX后开始。对比"提取到的 LaTeX + 原图"，产出状态（ok/warning/error）、问题列表、覆盖率与 0–100 的置信度

### 处理策略

- **并发执行**：第1次（LaTeX）与第2次（分析）会并发触发
- **顺序核查**：第3次（核查）在拿到 LaTeX 后开启
- **实时反馈**：识别过程以阶段指示的方式推送到界面，失败可单独重试对应阶段
- **本地留存**：识别记录（含原图路径、LaTeX、分析结果与置信度）保存到本机，支持搜索、排序、收藏与详情抽屉查看
- **模型调用**：识别依赖联网的 LLM 服务（如 Gemini）。首次使用需在"设置"里填入可用的 API Key 并通过"测试"验证连接

## 注意事项 📝

### 模型推荐 🤖

注意：目前只支持 Google Gemini API。⚠️

**建议的模型选择与典型耗时**（参考值，取决于网络与图片复杂度）：

- **Gemini-2.5-flash**：综合准确率与速度较优。常见公式约 10s 左右，复杂场景（含核查）整体体验约 20s
- **Gemini-2.5-flash-lite**：更快更省，但在复杂公式上准确性略逊，可作轻量场景的备选

## 使用流程 🧭

### 1. 初次设置

- 打开应用 → 进入"设置"页
- 填写 API Key，点击"测试"确保连通
- 可按需调整：语言（中/英）、截图快捷键、公式渲染引擎（MathJax/KaTeX）与默认 LaTeX 包裹格式

### 2. 开始识别（任选其一）

- **截图识别**：点击"截图识别"或使用快捷键 `Ctrl + Shift + A` 拉框，松开即可开始处理
- **导入图片**：点击"导入图片"，选择待识别的 PNG/JPG/JPEG 文件

### 3. 查看与编辑

- **进度指示**：依次显示 LaTeX → 分析 → 核查（可对任一阶段"重试"）
- **基础页**：预览公式、复制或编辑 LaTeX、展开原图对照
- **分析页**：查看摘要、变量（含符号/单位）、术语、建议，以及核查报告与置信度

### 4. 历史与收藏

- 识别结果会自动入库
- 在"历史记录/收藏夹"中可搜索、排序、查看详情（抽屉式），支持收藏与删除

## 部署说明 🛠️

### 开发环境搭建 🧰

#### 前置要求 📦

- Node.js 18+
- Rust 1.70+
- Git

#### 克隆项目 ⬇️

```bash
git clone https://github.com/AI-Formula-Scanner/AI-Formula-Scanner.git
cd AI-Formula-Scanner
```

#### 安装依赖 📥

```bash
# 安装前端依赖
npm install

# 安装 Tauri CLI（如果尚未安装）
npm install -g @tauri-apps/cli
```

#### 开发模式运行 ▶️

```bash
# 启动开发服务器
npm run tauri dev
```

## 已知问题 🐞

- 历史记录较多时存在性能问题
- 当前版本会在核查结果返回后才一并展示"分析"结果

## 许可证 📄

本项目采用 Apache License 2.0 发布。你可以在遵守许可证条款的前提下自由使用、复制、修改与分发本软件，同时获得明确的专利授权与贡献者专利授权。
