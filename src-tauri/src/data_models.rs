use serde::{Deserialize, Serialize};
use crate::prompts::{PromptManager, PromptType};

fn default_language() -> String {
    "zh-CN".to_string()
}

fn default_max_output_tokens() -> u32 {
    240000
}

fn default_window_width() -> u32 { 1280 }
fn default_window_height() -> u32 { 800 }
fn default_remember_window_state() -> bool { true }
fn default_screenshot_shortcut() -> String { "CommandOrControl+Shift+A".to_string() }
const PROMPTS_VERSION_CURRENT: u32 = 3;
fn current_prompts_version() -> u32 { PROMPTS_VERSION_CURRENT }
fn default_prompts_version() -> u32 { 0 }

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub api_key: String,
    pub api_base_url: String,
    pub provider: String,
    pub default_engine: String,
    pub custom_prompt: String,
    /// Prompt for LaTeX-only fast extraction
    #[serde(default = "default_latex_prompt")]
    pub latex_prompt: String,
    /// Prompt for analysis (title, summary, variables, terms, suggestions)
    #[serde(default = "default_analysis_prompt")]
    pub analysis_prompt: String,
    /// Prompt for verification (image + LaTeX checking). Previously named confidencePrompt
    #[serde(alias = "confidencePrompt", alias = "confidence_prompt")]
    pub verification_prompt: String,
    pub render_engine: String,
    pub auto_calculate_confidence: bool,
    pub enable_clipboard_watcher: bool,
    pub default_latex_format: String,
    pub request_timeout_seconds: u64,
    pub max_retries: u32,
    /// 最大输出 Token，上限控制模型输出长度
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u32,
    #[serde(default = "default_language")]
    pub language: String,
    /// 窗口默认/记忆尺寸与位置
    #[serde(default = "default_window_width")]
    pub window_width: u32,
    #[serde(default = "default_window_height")]
    pub window_height: u32,
    #[serde(default)]
    pub window_x: Option<i32>,
    #[serde(default)]
    pub window_y: Option<i32>,
    #[serde(default = "default_remember_window_state")]
    pub remember_window_state: bool,
    /// 内置提示词版本号，用于触发自动迁移
    #[serde(default = "default_prompts_version")]
    pub prompts_version: u32,
    /// 截图识别快捷键
    #[serde(default = "default_screenshot_shortcut")]
    pub screenshot_shortcut: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: "".to_string(),
            api_base_url: "https://generativelanguage.googleapis.com/v1beta/models".to_string(),
            provider: "gemini".to_string(),
            default_engine: "gemini-2.5-flash".to_string(),
            custom_prompt: String::new(),
            latex_prompt: default_latex_prompt(),
            analysis_prompt: default_analysis_prompt(),
            verification_prompt: default_verification_prompt(),
            render_engine: "MathJax".to_string(),
            auto_calculate_confidence: false,
            enable_clipboard_watcher: false,
            default_latex_format: "double_dollar".to_string(),
            request_timeout_seconds: 120,
            max_retries: 2,
            max_output_tokens: default_max_output_tokens(),
            language: default_language(),
            window_width: default_window_width(),
            window_height: default_window_height(),
            window_x: None,
            window_y: None,
            remember_window_state: default_remember_window_state(),
            prompts_version: current_prompts_version(),
            screenshot_shortcut: default_screenshot_shortcut(),
        }
    }
}

impl Config {
    /// Convert Config to LlmConfig for the LLM client
    pub fn to_llm_config(&self) -> crate::llm_api::LlmConfig {
        crate::llm_api::LlmConfig {
            api_key: self.api_key.clone(),
            api_base_url: self.api_base_url.clone(),
            model_name: self.default_engine.clone(),
            request_timeout_seconds: self.request_timeout_seconds,
            max_retries: self.max_retries,
            max_output_tokens: self.max_output_tokens,
        }
    }

    /// Returns the current default prompts tuple (latex, analysis, verification)
    pub fn default_prompts_tuple() -> (String, String, String) {
        (default_latex_prompt(), default_analysis_prompt(), default_verification_prompt())
    }

    /// Migrate old/empty prompts to new defaults without touching custom content
    /// Returns true if any field was changed
    pub fn migrate_prompts(&mut self) -> bool {
        let mut changed = false;
        let (def_latex, def_analysis, def_ver) = Self::default_prompts_tuple();

        // 若版本号落后，直接覆盖为当前默认，并更新版本号
        if self.prompts_version < current_prompts_version() {
            self.latex_prompt = def_latex;
            self.analysis_prompt = def_analysis;
            self.verification_prompt = def_ver;
            self.prompts_version = current_prompts_version();
            changed = true;
        } else {
            // 兜底：字段为空时补默认
            if self.latex_prompt.trim().is_empty() { self.latex_prompt = def_latex; changed = true; }
            if self.analysis_prompt.trim().is_empty() { self.analysis_prompt = def_analysis; changed = true; }
            if self.verification_prompt.trim().is_empty() { self.verification_prompt = def_ver; changed = true; }
        }

        changed
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub latex: String,
    pub title: String,
    pub analysis: Analysis,
    pub is_favorite: bool,
    pub created_at: String,
    pub confidence_score: u8,
    pub original_image: String,
    #[serde(default)]
    pub model_name: Option<String>,
    #[serde(default)]
    pub verification: Option<Verification>,
    /// 核查报告，描述LaTeX与原图像的对比结果
    #[serde(default)]
    pub verification_report: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Analysis {
    pub summary: String,
    #[serde(default)]
    pub variables: Vec<VariableInfo>,
    #[serde(default)]
    pub terms: Vec<TermInfo>,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Suggestion {
    #[serde(rename = "type")]
    pub suggestion_type: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VariableInfo {
    pub symbol: String,
    pub description: String,
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TermInfo {
    pub name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerificationIssue {
    pub category: String, // missing_term | extra_term | symbol_mismatch | notation_mismatch | layout_mismatch | other
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerificationCoverage {
    pub symbols_matched: u32,
    pub symbols_total: u32,
    pub terms_matched: u32,
    pub terms_total: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Verification {
    pub status: String, // error | warning | ok
    #[serde(default)]
    pub issues: Vec<VerificationIssue>,
    #[serde(default)]
    pub coverage: Option<VerificationCoverage>,
}

/// 新的验证结果结构，包含置信度和核查报告
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerificationResult {
    pub confidence_score: u8,
    pub verification_report: String,
}

fn default_latex_prompt() -> String { PromptManager::get_base_prompt(PromptType::LaTeX) }

fn default_analysis_prompt() -> String { PromptManager::get_base_prompt(PromptType::Analysis) }

fn default_verification_prompt() -> String { PromptManager::get_base_prompt(PromptType::Verification) }
