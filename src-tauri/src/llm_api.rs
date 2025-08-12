// Handles all communication with the LLM API

use crate::data_models::Analysis;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use std::time::Duration;
use tokio::time::sleep;

/// Configuration for LLM service
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_key: String,
    pub api_base_url: String,
    pub model_name: String,
    pub request_timeout_seconds: u64,
    pub max_retries: u32,
    pub max_output_tokens: u32,
}

/// Generic LLM client trait for different providers
#[async_trait]
pub trait LlmClient: Send + Sync {
    // 已移除 perform_recognition 方法，使用更具体的方法

    /// Gets the verification result (confidence score + report) for a given LaTeX string
    async fn get_verification_result(
        &self,
        prompt: &str,
        latex: &str,
    ) -> Result<crate::data_models::VerificationResult, anyhow::Error>;

    /// Verifies latex vs image and returns structured verification
    async fn verify_latex_against_image(
        &self,
        latex: &str,
        image_base64: &str,
        language: &str,
    ) -> Result<crate::data_models::Verification, anyhow::Error>;

    /// Extracts only LaTeX from the given image
    async fn extract_latex(
        &self,
        prompt: &str,
        image_base64: &str,
    ) -> Result<String, anyhow::Error>;

    /// Generates analysis (title, summary, variables, terms, suggestions)
    async fn generate_analysis(
        &self,
        prompt: &str,
        image_base64: &str,
    ) -> Result<(String, Analysis), anyhow::Error>;

    // 已移除 get_confidence_score_with_image，使用 get_verification_result_with_image

    /// Gets verification result (confidence score + report) using both LaTeX and original image
    async fn get_verification_result_with_image(
        &self,
        prompt: &str,
        latex: &str,
        image_base64: &str,
    ) -> Result<crate::data_models::VerificationResult, anyhow::Error>;

    /// Generic content generation method
    async fn generate_content(&self, prompt: &str) -> Result<String, anyhow::Error>;
}

#[derive(Debug)]
pub struct ApiClient {
    client: Client,
    config: LlmConfig,
}

// --- Gemini API Request Structures ---

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum GeminiPart {
    Text { text: String },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: GeminiInlineData
    },
}

#[derive(Serialize)]
struct GeminiInlineData {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
}

#[derive(Serialize)]
struct GeminiGenerationConfig {
    temperature: f32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
}

// --- Gemini API Response Structures ---

#[derive(Serialize, Deserialize, Debug)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiCandidate {
    content: GeminiResponseContent,
    #[serde(rename = "finishReason", default)]
    finish_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiResponsePart {
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RecognitionContent {
    latex: String,
    title: String,
    analysis: Analysis,
}

#[derive(Serialize, Deserialize, Debug)]
struct ConfidenceScoreContent {
    confidence_score: u8,
}

#[derive(Serialize, Deserialize, Debug)]
struct VerificationResultContent {
    confidence_score: u8,
    verification_report: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LatexOnlyContent {
    latex: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnalysisOnlyContent {
    title: String,
    analysis: Analysis,
}

impl ApiClient {
    /// Creates a new ApiClient instance with configuration.
    pub fn new(config: LlmConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    fn new_with_config(mut config: LlmConfig, base_url: String) -> Self {
        config.api_base_url = base_url;
        Self::new(config)
    }

    /// Helper method to send request with retry logic
    async fn send_request_with_retry(&self, request_body: &GeminiRequest) -> Result<String> {
        let mut attempts = 0;
        loop {
            match self.send_request(request_body).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    let msg = e.to_string();
                    let is_retryable_http = msg.contains("status 429")
                        || msg.contains("status 500")
                        || msg.contains("status 502")
                        || msg.contains("status 503")
                        || msg.contains("status 504");
                    let is_retryable_transport = msg.contains("Failed to send request")
                        || msg.to_lowercase().contains("timeout")
                        || msg.to_lowercase().contains("timed out")
                        || msg.to_lowercase().contains("connection reset")
                        || msg.to_lowercase().contains("temporarily unavailable");
                    let is_context_canceled = msg.to_lowercase().contains("context canceled")
                        || msg.contains("status 499");

                    let should_retry = (is_retryable_http || is_retryable_transport) && !is_context_canceled;

                    if should_retry && attempts < self.config.max_retries {
                        attempts += 1;
                        // Exponential backoff with small pseudo-jitter without extra deps
                        let base_secs = 2u64.pow(attempts);
                        let jitter_ms = (attempts as u64 * 137) % 1000;
                        let delay = Duration::from_secs(base_secs) + Duration::from_millis(jitter_ms);
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "[LLM] Retry #{}, reason='{}', waiting {:?}",
                            attempts, msg, delay
                        );
                        sleep(delay).await;
                        continue;
                    }

                    return Err(e);
                }
            }
        }
    }

    /// Helper method to clean LLM response (remove markdown markers)
    fn clean_response(&self, response: &str) -> String {
        response
            .replace("```json", "")
            .replace("```", "")
            .trim()
            .to_string()
    }

    fn build_verification_prompt(latex: &str, language: &str) -> String {
        let lang_note = if language == "zh-CN" {
            "Output language: Simplified Chinese for 'issues[*].message'. Keys remain English.".to_string()
        } else {
            "Output language: English for 'issues[*].message'. Keys remain English.".to_string()
        };
        format!(
            "You are a strict verifier. Compare the provided LaTeX with the image. Do NOT fix the LaTeX; only point out mismatches. Return a strict JSON: {{\n  \"status\": \"error|warning|ok\",\n  \"issues\": [{{\"category\": \"missing_term|extra_term|symbol_mismatch|notation_mismatch|layout_mismatch|other\", \"message\": \"...\"}}],\n  \"coverage\": {{\"symbols_matched\": n, \"symbols_total\": n, \"terms_matched\": n, \"terms_total\": n}}\n}}.\nRules:\n- status=error if ANY mismatch that changes math meaning (missing/extra term, wrong symbol, wrong power/subscript, different operator).\n- status=warning for layout/formatting-only differences (line breaks, spacing) that do not change math.\n- status=ok only if visually and semantically equivalent.\n- Be concise but precise.\n{}\nLaTeX to verify:\n{}",
            lang_note, latex)
    }

    // 已删除 internal_perform_recognition 方法

    async fn internal_extract_latex(
        &self,
        prompt: &str,
        image_base64: &str,
    ) -> Result<String, anyhow::Error> {
        let request_body = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![
                    GeminiPart::Text { text: prompt.to_string() },
                    GeminiPart::InlineData { inline_data: GeminiInlineData { mime_type: "image/png".to_string(), data: image_base64.to_string() }},
                ],
            }],
            generation_config: GeminiGenerationConfig {
                temperature: 0.2,
                max_output_tokens: self.config.max_output_tokens,
            },
        };

        let response_text = self.send_request_with_retry(&request_body).await?;
        let content_str = match serde_json::from_str::<GeminiResponse>(&response_text) {
            Ok(api_response) => {
                api_response
                    .candidates
                    .get(0)
                    .and_then(|c| c.content.parts.get(0))
                    .map(|p| p.text.clone())
                    .ok_or_else(|| anyhow!("Gemini returned no text for latex extraction"))?
            }
            Err(_) => return Err(anyhow!("Failed to parse Gemini response for latex extraction")),
        };
        let clean = self.clean_response(&content_str);
        // 首选严格 JSON 解析
        match serde_json::from_str::<LatexOnlyContent>(&clean) {
            Ok(v) => Ok(v.latex),
            Err(_e) => {
                // 容错：尝试宽松提取 \"latex\" 字段字符串（修复结尾多余 ] 等常见错误）
                if let Some(decoded) = Self::try_relaxed_extract_latex(&clean) {
                    return Ok(decoded);
                }
                Err(anyhow!("Failed to parse latex-only content: {}", clean))
            }
        }
    }

    async fn internal_generate_analysis(
        &self,
        prompt: &str,
        image_base64: &str,
    ) -> Result<(String, Analysis), anyhow::Error> {
        let request_body = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![
                    GeminiPart::Text { text: prompt.to_string() },
                    GeminiPart::InlineData { inline_data: GeminiInlineData { mime_type: "image/png".to_string(), data: image_base64.to_string() }},
                ],
            }],
            generation_config: GeminiGenerationConfig {
                temperature: 0.5,
                max_output_tokens: self.config.max_output_tokens,
            },
        };
        let response_text = self.send_request_with_retry(&request_body).await?;
        let content_str = match serde_json::from_str::<GeminiResponse>(&response_text) {
            Ok(api_response) => {
                api_response
                    .candidates
                    .get(0)
                    .and_then(|c| c.content.parts.get(0))
                    .map(|p| p.text.clone())
                    .ok_or_else(|| anyhow!("Gemini returned no text for analysis"))?
            }
            Err(_) => return Err(anyhow!("Failed to parse Gemini response for analysis")),
        };
        let clean = self.clean_response(&content_str);
        // 容错：有些模型会误返回 {"latex": "..."} 到分析提示，尝试兜底
        if clean.contains("\"latex\"") && !clean.contains("\"analysis\"") {
            return Ok(("Untitled formula".to_string(), Analysis { summary: String::new(), variables: Vec::new(), terms: Vec::new(), suggestions: Vec::new() }));
        }
        let analysis: AnalysisOnlyContent = serde_json::from_str(&clean)
            .with_context(|| format!("Failed to parse analysis content: {}", clean))?;
        Ok((analysis.title, analysis.analysis))
    }

    /// 宽松提取：从形如 {"latex": "..."} 的文本中，稳健解析出 JSON 字符串值
    /// 处理一些模型常见输出瑕疵（如末尾多了一个 ] 落在引号之外）
    fn try_relaxed_extract_latex(clean: &str) -> Option<String> {
        let key = "\"latex\"";
        let mut start = clean.find(key)?;
        start += key.len();
        // 寻找冒号
        let mut colon = None;
        for (i, ch) in clean[start..].char_indices() {
            if ch == ':' { colon = Some(start + i); break; }
        }
        let colon = colon?;
        // 冒号后第一个引号作为字符串起点
        let mut qstart = None;
        for (i, ch) in clean[colon+1..].char_indices() {
            if ch == '"' { qstart = Some(colon + 1 + i); break; }
            if !ch.is_whitespace() && ch != '"' { continue; }
        }
        let qstart = qstart?;
        // 扫描 JSON 字符串，考虑转义
        let bytes = clean.as_bytes();
        let mut i = qstart + 1;
        let mut escaped = false;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if c == '"' && !escaped {
                // [qstart, i] 是带引号的 JSON 字符串
                let s = &clean[qstart..=i];
                if let Ok(decoded) = serde_json::from_str::<String>(s) {
                    return Some(decoded);
                } else {
                    return None;
                }
            }
            if c == '\\' && !escaped { escaped = true; } else { escaped = false; }
            i += 1;
        }
        None
    }

    // 已删除 internal_get_confidence_score 方法

    /// Internal method for getting verification result (confidence + report)
    async fn internal_get_verification_result(
        &self,
        prompt: &str,
        latex: &str,
    ) -> Result<crate::data_models::VerificationResult, anyhow::Error> {
        let request_body = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![
                    GeminiPart::Text {
                        text: format!("{}\n\nLaTeX to evaluate: {}", prompt, latex),
                    },
                ],
            }],
            generation_config: GeminiGenerationConfig {
                temperature: 0.2,
                max_output_tokens: self.config.max_output_tokens,
            },
        };

        let response_text = self.send_request_with_retry(&request_body).await?;

        let content_str = match serde_json::from_str::<GeminiResponse>(&response_text) {
            Ok(api_response) => {
                let maybe_text = api_response
                    .candidates
                    .get(0)
                    .and_then(|c| c.content.parts.get(0))
                    .map(|p| p.text.clone());
                if let Some(text) = maybe_text {
                    text
                } else {
                    return Err(anyhow!("Gemini returned no text for verification"));
                }
            }
            Err(_) => return Err(anyhow!("Failed to parse Gemini response for verification")),
        };

        let clean_content = self.clean_response(&content_str);
        let verification_content: VerificationResultContent = serde_json::from_str(&clean_content)
            .with_context(|| format!("Failed to parse verification content from API: {}", clean_content))?;

        Ok(crate::data_models::VerificationResult {
            confidence_score: verification_content.confidence_score,
            verification_report: verification_content.verification_report,
        })
    }

    async fn internal_verify_latex_against_image(
        &self,
        latex: &str,
        image_base64: &str,
        language: &str,
    ) -> Result<crate::data_models::Verification, anyhow::Error> {
        let prompt = Self::build_verification_prompt(latex, language);
        let request_body = GeminiRequest {
            contents: vec![GeminiContent { parts: vec![
                GeminiPart::Text { text: prompt },
                GeminiPart::InlineData { inline_data: GeminiInlineData { mime_type: "image/png".into(), data: image_base64.to_string() }},
            ]}],
            generation_config: GeminiGenerationConfig { temperature: 0.2, max_output_tokens: self.config.max_output_tokens },
        };
        let response_text = self.send_request_with_retry(&request_body).await?;
        let content_str = match serde_json::from_str::<GeminiResponse>(&response_text) {
            Ok(api_response) => api_response.candidates.get(0).and_then(|c| c.content.parts.get(0)).map(|p| p.text.clone()).ok_or_else(|| anyhow!("Gemini returned no text for verification"))?,
            Err(_) => return Err(anyhow!("Failed to parse Gemini response for verification")),
        };
        let clean = self.clean_response(&content_str);
        let v: crate::data_models::Verification = serde_json::from_str(&clean).with_context(|| format!("Failed to parse verification: {}", clean))?;
        Ok(v)
    }

    // 已删除 internal_get_confidence_score_with_image 方法

    /// Internal method for getting verification result with image
    async fn internal_get_verification_result_with_image(
        &self,
        prompt: &str,
        latex: &str,
        image_base64: &str,
    ) -> Result<crate::data_models::VerificationResult, anyhow::Error> {
        let request_body = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![
                    GeminiPart::Text { text: format!("{}\n\nLaTeX to evaluate: {}", prompt, latex) },
                    GeminiPart::InlineData { inline_data: GeminiInlineData { mime_type: "image/png".to_string(), data: image_base64.to_string() }},
                ],
            }],
            generation_config: GeminiGenerationConfig {
                temperature: 0.2,
                max_output_tokens: self.config.max_output_tokens,
            },
        };

        let response_text = self.send_request_with_retry(&request_body).await?;
        let content_str = match serde_json::from_str::<GeminiResponse>(&response_text) {
            Ok(api_response) => {
                api_response
                    .candidates
                    .get(0)
                    .and_then(|c| c.content.parts.get(0))
                    .map(|p| p.text.clone())
                    .ok_or_else(|| anyhow!("Gemini returned no text for verification with image"))?
            }
            Err(_) => return Err(anyhow!("Failed to parse Gemini response for verification with image")),
        };

        let clean_content = self.clean_response(&content_str);
        let verification_content: VerificationResultContent = serde_json::from_str(&clean_content)
            .with_context(|| format!("Failed to parse verification content from API: {}", clean_content))?;

        Ok(crate::data_models::VerificationResult {
            confidence_score: verification_content.confidence_score,
            verification_report: verification_content.verification_report,
        })
    }

    fn canonical_models_base(&self) -> String {
        let b = self.config.api_base_url.trim_end_matches('/');
        if b.contains("/models") {
            b.to_string()
        } else if b.contains("/v1beta") || b.contains("/v1") {
            format!("{}/models", b)
        } else {
            format!("{}/v1beta/models", b)
        }
    }

    /// Generic function to send a request to the Gemini API.
    async fn send_request(&self, request_body: &GeminiRequest) -> Result<String> {
        // 自动补全代理前缀缺失的版本与 models 段，提高兼容性
        let base = self.canonical_models_base();
        let mut url = format!("{}/{}:generateContent", base, self.config.model_name);
        if !self.config.api_key.is_empty() {
            url.push_str(&format!("?key={}", self.config.api_key));
        }

        // 打印请求摘要（不泄露密钥，不输出图片原始数据）
        #[cfg(debug_assertions)]
        {
            let masked_url = url.split('?').next().unwrap_or(&url).to_string();
            let mut parts_desc: Vec<String> = Vec::new();
            for content in &request_body.contents {
                for part in &content.parts {
                    match part {
                        GeminiPart::Text { text } => parts_desc.push(format!("text({} chars)", text.len())),
                        GeminiPart::InlineData { inline_data } => {
                            parts_desc.push(format!("image({} bytes)", inline_data.data.len()))
                        }
                    }
                }
            }
            eprintln!(
                "[LLM] Request -> url={} parts=[{}] maxOutputTokens={} temperature={}",
                masked_url,
                parts_desc.join(", "),
                request_body.generation_config.max_output_tokens,
                request_body.generation_config.temperature
            );
        }

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(request_body)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        let status = response.status();
        let text = response
            .text()
            .await
            .context("Failed to read response text")?;

        #[cfg(debug_assertions)]
        {
            let masked_url = url.split('?').next().unwrap_or(&url).to_string();
            let snippet: String = text.chars().take(4000).collect();
            eprintln!(
                "[LLM] Response <- url={} status={} len={} bodySnippet={}",
                masked_url,
                status.as_u16(),
                text.len(),
                snippet
            );
        }

        if !status.is_success() {
            return Err(anyhow!(
                "API request failed with status {}: {}",
                status,
                text
            ));
        }

        Ok(text)
    }
}

/// Implementation of LlmClient trait for ApiClient
#[async_trait]
impl LlmClient for ApiClient {
    // 已移除 perform_recognition 实现

    async fn get_verification_result(
        &self,
        prompt: &str,
        latex: &str,
    ) -> Result<crate::data_models::VerificationResult, anyhow::Error> {
        self.internal_get_verification_result(prompt, latex).await
    }

    async fn verify_latex_against_image(
        &self,
        latex: &str,
        image_base64: &str,
        language: &str,
    ) -> Result<crate::data_models::Verification, anyhow::Error> {
        self.internal_verify_latex_against_image(latex, image_base64, language).await
    }

    async fn extract_latex(
        &self,
        prompt: &str,
        image_base64: &str,
    ) -> Result<String, anyhow::Error> {
        self.internal_extract_latex(prompt, image_base64).await
    }

    async fn generate_analysis(
        &self,
        prompt: &str,
        image_base64: &str,
    ) -> Result<(String, Analysis), anyhow::Error> {
        self.internal_generate_analysis(prompt, image_base64).await
    }

    // 已移除 get_confidence_score_with_image 实现

    async fn get_verification_result_with_image(
        &self,
        prompt: &str,
        latex: &str,
        image_base64: &str,
    ) -> Result<crate::data_models::VerificationResult, anyhow::Error> {
        self.internal_get_verification_result_with_image(prompt, latex, image_base64).await
    }

    async fn generate_content(&self, prompt: &str) -> Result<String, anyhow::Error> {
        let request_body = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart::Text {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: GeminiGenerationConfig {
                temperature: 0.7,
                max_output_tokens: self.config.max_output_tokens,
            },
        };

        let response_text = self.send_request_with_retry(&request_body).await?;

        let content = match serde_json::from_str::<GeminiResponse>(&response_text) {
            Ok(api_response) => {
                let maybe_text = api_response
                    .candidates
                    .get(0)
                    .and_then(|c| c.content.parts.get(0))
                    .map(|p| p.text.clone());
                if let Some(text) = maybe_text {
                    text
                } else {
                    let v: serde_json::Value = serde_json::from_str(&response_text)
                        .with_context(|| format!("Failed to parse Gemini API response JSON: {}", response_text))?;
                    let finish_reason = v
                        .get("candidates")
                        .and_then(|c| c.get(0))
                        .and_then(|c0| c0.get("finishReason"))
                        .and_then(|fr| fr.as_str())
                        .unwrap_or("unknown");
                    return Err(anyhow!(
                        "Gemini returned no text (finishReason: {}). Raw: {}",
                        finish_reason,
                        response_text
                    ));
                }
            }
            Err(_) => {
                let v: serde_json::Value = serde_json::from_str(&response_text)
                    .with_context(|| format!("Failed to parse Gemini API response JSON: {}", response_text))?;
                let finish_reason = v
                    .get("candidates")
                    .and_then(|c| c.get(0))
                    .and_then(|c0| c0.get("finishReason"))
                    .and_then(|fr| fr.as_str())
                    .unwrap_or("unknown");
                return Err(anyhow!(
                    "Gemini returned no text (finishReason: {}). Raw: {}",
                    finish_reason,
                    response_text
                ));
            }
        };

        Ok(self.clean_response(&content))
    }
}

// 测试已移除，因为相关方法已重构
