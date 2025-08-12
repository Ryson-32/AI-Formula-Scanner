// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Import modules
mod data_models;
mod fs_manager;
mod llm_api;
mod prompts;
mod capture;

use arboard::Clipboard;
use base64::{engine::general_purpose, Engine as _};
use data_models::{Config, HistoryItem};
use llm_api::{ApiClient, LlmClient};
use screenshots::Screen;
use tauri::{AppHandle, Manager, GlobalShortcutManager};
use serde::Serialize;
#[cfg(debug_assertions)]
use serde_json::json;
use uuid::Uuid;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

// --- Tauri Commands ---

// 旧的提示词构建函数已移至 prompts.rs 模块

fn default_title_for_lang(language: &str) -> String {
    if language == "zh-CN" { "未命名公式".to_string() } else { "Untitled formula".to_string() }
}

fn default_summary_for_lang(language: &str) -> String {
    if language == "zh-CN" { "分析暂不可用，请稍后重试。".to_string() } else { "Analysis is temporarily unavailable. Please try again.".to_string() }
}

#[derive(Serialize, Clone)]
struct RecognitionProgressPayload {
    id: String,
    stage: String, // "latex" | "analysis" | "confidence"
    latex: Option<String>,
    title: Option<String>,
    analysis: Option<data_models::Analysis>,
    confidence_score: Option<u8>,
    created_at: Option<String>,
    original_image: Option<String>,
    model_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification: Option<data_models::Verification>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_version: Option<String>, // "default" | "custom" | "full"
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_report: Option<String>,
}

fn emit_progress(app_handle: &AppHandle, payload: RecognitionProgressPayload) {
    let _ = app_handle.emit_all("recognition_progress", payload);
}

fn compute_verification_result_from_struct(
    verification: &data_models::Verification,
) -> data_models::VerificationResult {
    // 依据 coverage 计算分数；若无 coverage，则按 status 与 issues 数量估算
    let score: u8 = if let Some(cov) = &verification.coverage {
        let symbols_score = if cov.symbols_total > 0 {
            (100.0 * (cov.symbols_matched as f32) / (cov.symbols_total as f32)).round()
        } else {
            100.0
        };
        let terms_score = if cov.terms_total > 0 {
            (100.0 * (cov.terms_matched as f32) / (cov.terms_total as f32)).round()
        } else {
            100.0
        };
        let combined = (0.75 * symbols_score + 0.25 * terms_score).round();
        combined.clamp(0.0, 100.0) as u8
    } else {
        // 无覆盖率时的启发式
        let issues_len = verification.issues.len() as u32;
        match verification.status.as_str() {
            "ok" => 100,
            "warning" => 80u8.saturating_sub((issues_len * 2).min(20) as u8),
            _ => 60u8.saturating_sub((issues_len * 5).min(50) as u8),
        }
    };

    // 生成简要报告
    let report = if verification.status == "ok" && verification.issues.is_empty() {
        "LaTeX 完全匹配原始公式。".to_string()
    } else {
        // 拼接前若干条问题，避免过长
        let mut lines: Vec<String> = Vec::new();
        for (i, issue) in verification.issues.iter().enumerate() {
            if i >= 10 { break; }
            lines.push(format!("- [{}] {}", issue.category, issue.message));
        }
        if verification.issues.len() > 10 {
            lines.push(format!("(其余 {} 条问题已省略)", verification.issues.len() - 10));
        }
        if lines.is_empty() {
            // 无显式问题但状态非 ok
            match verification.status.as_str() {
                "warning" => "存在版式/排版差异，但不影响数学含义。".to_string(),
                _ => "存在与原图不一致的内容，请检查符号、上下标与项是否匹配。".to_string(),
            }
        } else {
            format!("发现以下差异：\n{}", lines.join("\n"))
        }
    };

    data_models::VerificationResult { confidence_score: score, verification_report: report }
}

fn determine_prompt_version(config: &crate::data_models::Config) -> String {
    // 检查实际使用的提示词类型
    // 根据代码逻辑：如果latex_prompt不为空，使用后端默认提示词；否则使用custom_prompt

    // 如果latex_prompt不为空，说明使用的是后端默认提示词（含语言约束的完整版）
    if !config.latex_prompt.is_empty() {
        return "full".to_string();
    }

    // 如果latex_prompt为空但custom_prompt不为空，说明使用自定义提示词
    if !config.custom_prompt.is_empty() {
        return "custom".to_string();
    }

    // 兜底情况
    "default".to_string()
}

#[tauri::command]
async fn test_connection(app_handle: AppHandle) -> Result<String, String> {
    // 每次读取最新配置，避免旧配置缓存
    let config = fs_manager::read_config(&app_handle).map_err(|e| e.to_string())?;
    let client = ApiClient::new(config.to_llm_config());
    client
        .generate_content("ping")
        .await
        .map(|_| "ok".to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_config_dir(app_handle: AppHandle) -> Result<(), String> {
    let dir = app_handle
        .path_resolver()
        .app_data_dir()
        .ok_or_else(|| "Failed to resolve app data dir".to_string())?;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[derive(Serialize)]
struct DefaultPromptsResponse {
    latex_prompt: String,
    analysis_prompt: String,
    verification_prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    latex_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    analysis_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_language: Option<String>,
}

#[tauri::command]
fn get_default_prompts() -> DefaultPromptsResponse {
    let (latex_prompt, analysis_prompt, verification_prompt) = prompts::get_base_prompts_tuple();
    // 默认不含语言约束，但为统一前端“再加工”接口也返回空 Option
    DefaultPromptsResponse { latex_prompt, analysis_prompt, verification_prompt, latex_language: None, analysis_language: None, verification_language: None }
}

#[derive(Serialize)]
struct FullPromptsResponse {
    latex_prompt: String,
    analysis_prompt: String,
    verification_prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    latex_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    analysis_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_language: Option<String>,
}

#[tauri::command]
fn get_full_prompts_with_language(language: String) -> FullPromptsResponse {
    // 现在：LaTeX 只返回基础提示词（不含语言约束），Analysis/Verification 返回“基础+语言”
    let (latex_base, analysis_base, verification_base) = prompts::get_base_prompts_tuple();
    let analysis_language = Some(prompts::PromptManager::get_language_constraint_for(prompts::PromptType::Analysis, &language));
    let verification_language = Some(prompts::PromptManager::get_language_constraint_for(prompts::PromptType::Verification, &language));
    let analysis_prompt = format!("{}\n\n{}", analysis_base, analysis_language.clone().unwrap());
    let verification_prompt = format!("{}\n\n{}", verification_base, verification_language.clone().unwrap());
    FullPromptsResponse { latex_prompt: latex_base, analysis_prompt, verification_prompt, latex_language: None, analysis_language, verification_language }
}

#[derive(Serialize)]
struct PromptParts {
    base: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    format_rule: Option<String>,
    language: String,
    full: String,
}

#[derive(Serialize)]
struct PromptPartsResponse {
    latex: PromptParts,
    analysis: PromptParts,
    verification: PromptParts,
}

#[tauri::command]
fn get_prompt_parts(language: String, default_format: String) -> PromptPartsResponse {
    // LaTeX parts（语言段移除，仅基础+格式规则）
    let latex_base = prompts::PromptManager::get_base_prompt(prompts::PromptType::LaTeX);
    let latex_format = prompts::format_rule_for_latex(&default_format);
    let latex_full = format!("{}{}", latex_base, latex_format);

    // Analysis parts
    let analysis_base = prompts::PromptManager::get_base_prompt(prompts::PromptType::Analysis);
    let analysis_lang = prompts::PromptManager::get_language_constraint_for(prompts::PromptType::Analysis, &language);
    let analysis_full = format!("{}\n\n{}", analysis_base, analysis_lang);

    // Verification parts
    let verification_base = prompts::PromptManager::get_base_prompt(prompts::PromptType::Verification);
    let verification_lang = prompts::PromptManager::get_language_constraint_for(prompts::PromptType::Verification, &language);
    let verification_full = format!("{}\n\n{}", verification_base, verification_lang);

    PromptPartsResponse {
        latex: PromptParts { base: latex_base, format_rule: Some(latex_format), language: String::new(), full: latex_full },
        analysis: PromptParts { base: analysis_base, format_rule: None, language: analysis_lang, full: analysis_full },
        verification: PromptParts { base: verification_base, format_rule: None, language: verification_lang, full: verification_full },
    }
}

#[tauri::command]
async fn recognize_from_screenshot(
    app_handle: AppHandle,
) -> Result<HistoryItem, String> {
    let config = fs_manager::read_config(&app_handle).map_err(|e| e.to_string())?;

    let screens = Screen::all().map_err(|e| e.to_string())?;
    if let Some(screen) = screens.first() {
        let image = screen.capture().map_err(|e| e.to_string())?;
        let png_bytes = image
            .to_png(None)
            .map_err(|e| e.to_string())?;
        let base64_image = general_purpose::STANDARD.encode(&png_bytes);

        let id = Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();
        let model_name = Some(config.default_engine.clone());

        let client = std::sync::Arc::new(ApiClient::new(config.to_llm_config()));

        // 运行期仅使用用户在前端保存的提示词；若为空则直接报错，提示用户去设置页恢复默认或保存
        if config.latex_prompt.trim().is_empty() {
            return Err("LaTeX 提示词未设置。请在设置中填写或点击‘恢复默认提示词’后重试。".to_string());
        }
        if config.analysis_prompt.trim().is_empty() {
            return Err("分析提示词未设置。请在设置中填写或点击‘恢复默认提示词’后重试。".to_string());
        }
        if config.verification_prompt.trim().is_empty() {
            return Err("核查提示词未设置。请在设置中填写或点击‘恢复默认提示词’后重试。".to_string());
        }

        let latex_prompt = {
            let mut p = config.latex_prompt.clone();
            p.push_str(&prompts::format_rule_for_latex(&config.default_latex_format));
            p
        };
        let analysis_prompt = {
            let mut p = config.analysis_prompt.clone();
            let lang = prompts::PromptManager::get_language_constraint_for(prompts::PromptType::Analysis, &config.language);
            p.push_str(&format!("\n\n{}", lang));
            p
        };
        // 第1次和第2次调用同时发出（都只输入图片）
        let latex_task = {
            let c = client.clone();
            let latex_prompt = latex_prompt.clone();
            let img = base64_image.clone();
            tokio::spawn(async move { c.extract_latex(&latex_prompt, &img).await })
        };

        let analysis_task = {
            let c = client.clone();
            let analysis_prompt = analysis_prompt.clone();
            let img = base64_image.clone();
            tokio::spawn(async move { c.generate_analysis(&analysis_prompt, &img).await })
        };

        // 等待第1次调用（LaTeX识别）完成
        let latex = match latex_task.await {
            Ok(Ok(latex)) => latex,
            Ok(Err(e)) => return Err(e.to_string()),
            Err(e) => return Err(format!("LaTeX task failed: {}", e)),
        };
        // 打印第1次返回（LaTeX 提取结果）
        #[cfg(debug_assertions)]
        {
            let payload = json!({ "latex": &latex });
            eprintln!("[LLM][Result][latex][{}] {}", id, payload.to_string());
        }
        let prompt_version = determine_prompt_version(&config);
        emit_progress(&app_handle, RecognitionProgressPayload {
            id: id.clone(), stage: "latex".into(), latex: Some(latex.clone()),
            title: None, analysis: None, confidence_score: None,
            created_at: Some(created_at.clone()),
            original_image: Some(format!("data:image/png;base64,{}", base64_image.clone())),
            model_name: model_name.clone(),
            verification: None,
            prompt_version: Some(prompt_version.clone()),
            verification_report: None,
        });

        // 第3阶段：仅使用用户保存的核查提示词（图像+LaTeX）计算置信度与报告
        let verification_prompt = {
            let mut p = config.verification_prompt.clone();
            let lang = prompts::PromptManager::get_language_constraint_for(prompts::PromptType::Verification, &config.language);
            p.push_str(&format!("\n\n{}", lang));
            p
        };
        let verification_task = {
            let c = client.clone();
            let latex = latex.clone();
            let img = base64_image.clone();
            let verification_prompt = verification_prompt.clone();
            tokio::spawn(async move {
                let vr = c.get_verification_result_with_image(&verification_prompt, &latex, &img)
                    .await
                    .unwrap_or(crate::data_models::VerificationResult { confidence_score: 0, verification_report: "验证失败".to_string() });
                (vr, None)
            })
        };

        // 等待第2次调用（分析）结果
        let (title, analysis) = match analysis_task.await {
            Ok(Ok(v)) => v,
            _ => (
                default_title_for_lang(&config.language),
                crate::data_models::Analysis { summary: default_summary_for_lang(&config.language), variables: Vec::new(), terms: Vec::new(), suggestions: Vec::new() }
            )
        };
        // 打印第2次返回（分析：标题/简介/变量/项/建议）
        #[cfg(debug_assertions)]
        {
            let payload = json!({ "title": &title, "analysis": &analysis });
            eprintln!("[LLM][Result][analysis][{}] {}", id, payload.to_string());
        }
        emit_progress(&app_handle, RecognitionProgressPayload {
            id: id.clone(), stage: "analysis".into(), latex: None,
            title: Some(title.clone()), analysis: Some(analysis.clone()), confidence_score: None,
            created_at: None, original_image: None, model_name: model_name.clone(),
            verification: None,
            prompt_version: Some(prompt_version.clone()),
            verification_report: None,
        });

        // 等待第3次调用（验证）结果
        let (verification_result, verification) = match verification_task.await {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Verification task failed: {}", e);
                (crate::data_models::VerificationResult {
                    confidence_score: 0,
                    verification_report: "验证失败".to_string(),
                }, None)
            }
        };
        // 打印第3次返回（置信度 + 核查）
        #[cfg(debug_assertions)]
        {
            let payload = json!({ "confidence_score": verification_result.confidence_score, "verification_report": &verification_result.verification_report, "verification": &verification });
            eprintln!("[LLM][Result][confidence+verify][{}] {}", id, payload.to_string());
        }
        emit_progress(&app_handle, RecognitionProgressPayload {
            id: id.clone(), stage: "confidence".into(), latex: None,
            title: None, analysis: None, confidence_score: Some(verification_result.confidence_score),
            created_at: None, original_image: None, model_name: model_name.clone(),
            verification: verification.clone(),
            prompt_version: Some(prompt_version.clone()),
            verification_report: Some(verification_result.verification_report.clone()),
        });

        let mut history_item = HistoryItem {
            id: id.clone(),
            latex,
            title,
            analysis,
            is_favorite: false,
            created_at: created_at.clone(),
            confidence_score: verification_result.confidence_score,
            original_image: base64_image.to_string(),
            model_name: model_name.clone(),
            verification,
            verification_report: Some(verification_result.verification_report),
        };

        // 将图片保存为文件（日期前缀），并用文件路径替换原始图片字段
        let date_str = chrono::DateTime::parse_from_rfc3339(&history_item.created_at)
            .map(|dt| dt.format("%Y%m%d_%H%M%S").to_string())
            .unwrap_or_else(|_| chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string());
        let stem = format!("{}_{}", date_str, history_item.id);
        let img_path = fs_manager::save_png_to_pictures(&app_handle, &stem, &png_bytes)
            .map_err(|e| e.to_string())?;
        history_item.original_image = img_path.to_string_lossy().to_string();

        // 持久化保存历史，防止前端页面切换导致结果丢失
        let mut history = fs_manager::read_history(&app_handle).map_err(|e| e.to_string())?;
        history.insert(0, history_item.clone());
        fs_manager::write_history(&app_handle, &history).map_err(|e| e.to_string())?;

        Ok(history_item)
    } else {
        Err("No screens found.".to_string())
    }
}

#[tauri::command]
async fn recognize_from_file(
    app_handle: AppHandle,
    file_path: String,
) -> Result<HistoryItem, String> {
    #[cfg(debug_assertions)]
    {
        eprintln!("🔥 [DEBUG] recognize_from_file called with: {}", file_path);
        eprintln!("🔥 [DEBUG] This function should only be called once per recognition");
    }

    let config = fs_manager::read_config(&app_handle).map_err(|e| e.to_string())?;
    let image_data = std::fs::read(&file_path).map_err(|e| e.to_string())?;
    // 统一转换为 PNG 字节
    let dyn_img = image::load_from_memory(&image_data).map_err(|e| e.to_string())?;
    let mut png_bytes: Vec<u8> = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut png_bytes);
        dyn_img
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
    }
    let base64_image = general_purpose::STANDARD.encode(&png_bytes);

    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    let model_name = Some(config.default_engine.clone());

        let client = std::sync::Arc::new(ApiClient::new(config.to_llm_config()));

    if config.latex_prompt.trim().is_empty() {
        return Err("LaTeX 提示词未设置。请在设置中填写或点击‘恢复默认提示词’后重试。".to_string());
    }
    if config.analysis_prompt.trim().is_empty() {
        return Err("分析提示词未设置。请在设置中填写或点击‘恢复默认提示词’后重试。".to_string());
    }
    if config.verification_prompt.trim().is_empty() {
        return Err("核查提示词未设置。请在设置中填写或点击‘恢复默认提示词’后重试。".to_string());
    }
    let latex_prompt = {
        let mut p = config.latex_prompt.clone();
        p.push_str(&prompts::format_rule_for_latex(&config.default_latex_format));
        p
    };
        let analysis_prompt = {
            let mut p = config.analysis_prompt.clone();
            let lang = prompts::PromptManager::get_language_constraint_for(prompts::PromptType::Analysis, &config.language);
            p.push_str(&format!("\n\n{}", lang));
            p
        };
    // 第1次和第2次调用同时发出（都只输入图片）
    let latex_task = {
        let c = client.clone();
        let latex_prompt = latex_prompt.clone();
        let img = base64_image.clone();
        tokio::spawn(async move { c.extract_latex(&latex_prompt, &img).await })
    };

    let analysis_task = {
        let c = client.clone();
        let analysis_prompt = analysis_prompt.clone();
        let img = base64_image.clone();
        tokio::spawn(async move { c.generate_analysis(&analysis_prompt, &img).await })
    };

    // 等待第1次调用（LaTeX识别）完成
    let latex = match latex_task.await {
        Ok(Ok(latex)) => latex,
        Ok(Err(e)) => return Err(e.to_string()),
        Err(e) => return Err(format!("LaTeX task failed: {}", e)),
    };
    #[cfg(debug_assertions)]
    {
        let payload = json!({ "latex": &latex });
        eprintln!("[LLM][Result][latex][{}] {}", id, payload.to_string());
    }
    let prompt_version = determine_prompt_version(&config);
    emit_progress(&app_handle, RecognitionProgressPayload { id: id.clone(), stage: "latex".into(), latex: Some(latex.clone()), title: None, analysis: None, confidence_score: None, created_at: Some(created_at.clone()), original_image: Some(format!("data:image/png;base64,{}", base64_image.clone())), model_name: model_name.clone(), verification: None, prompt_version: Some(prompt_version.clone()), verification_report: None });

    // 第3次调用：在第1次完成后发出（输入图片+LaTeX）
    let verification_prompt = {
        let mut p = config.verification_prompt.clone();
        let lang = prompts::PromptManager::get_language_constraint_for(prompts::PromptType::Verification, &config.language);
        p.push_str(&format!("\n\n{}", lang));
        p
    };
    let verification_task = {
        let c = client.clone();
        let latex = latex.clone();
        let img = base64_image.clone();
            let verification_prompt = verification_prompt.clone();
        tokio::spawn(async move {
                let vr = c.get_verification_result_with_image(&verification_prompt, &latex, &img)
                    .await
                    .unwrap_or(crate::data_models::VerificationResult { confidence_score: 0, verification_report: "验证失败".to_string() });
                (vr, None)
        })
    };
    // 等待第2次调用（分析）结果
    let (title, analysis) = match analysis_task.await { Ok(Ok(v)) => v, _ => (default_title_for_lang(&config.language), crate::data_models::Analysis { summary: default_summary_for_lang(&config.language), variables: Vec::new(), terms: Vec::new(), suggestions: Vec::new() }) };
    #[cfg(debug_assertions)]
    {
        let payload = json!({ "title": &title, "analysis": &analysis });
        eprintln!("[LLM][Result][analysis][{}] {}", id, payload.to_string());
    }
    emit_progress(&app_handle, RecognitionProgressPayload { id: id.clone(), stage: "analysis".into(), latex: None, title: Some(title.clone()), analysis: Some(analysis.clone()), confidence_score: None, created_at: None, original_image: None, model_name: model_name.clone(), verification: None, prompt_version: Some(prompt_version.clone()), verification_report: None });

    // 等待第3次调用（验证）结果
    let (verification_result, verification) = match verification_task.await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Verification task failed: {}", e);
            (crate::data_models::VerificationResult {
                confidence_score: 0,
                verification_report: "验证失败".to_string(),
            }, None)
        }
    };
    // 若有细粒度核查，则以其计算的分数/报告为准，否则使用回退评分
        let final_verification_result = verification_result.clone();
    #[cfg(debug_assertions)]
    {
        let payload = json!({ "confidence_score": final_verification_result.confidence_score, "verification_report": &final_verification_result.verification_report, "verification": &verification });
        eprintln!("[LLM][Result][confidence+verify][{}] {}", id, payload.to_string());
    }
    emit_progress(&app_handle, RecognitionProgressPayload { id: id.clone(), stage: "confidence".into(), latex: None, title: None, analysis: None, confidence_score: Some(final_verification_result.confidence_score), created_at: None, original_image: None, model_name: model_name.clone(), verification: verification.clone(), prompt_version: Some(prompt_version.clone()), verification_report: Some(final_verification_result.verification_report.clone()) });

    let mut history_item = HistoryItem {
        id: id.clone(),
        latex,
        title,
        analysis,
        is_favorite: false,
        created_at: created_at.clone(),
        confidence_score: final_verification_result.confidence_score,
        original_image: base64_image.to_string(),
        model_name: model_name.clone(),
            verification: None,
        verification_report: Some(final_verification_result.verification_report),
    };

    // 将图片保存为文件（日期前缀），并用文件路径替换原始图片字段
    let date_str = chrono::DateTime::parse_from_rfc3339(&history_item.created_at)
        .map(|dt| dt.format("%Y%m%d_%H%M%S").to_string())
        .unwrap_or_else(|_| chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string());
    let stem = format!("{}_{}", date_str, history_item.id);
    let img_path = fs_manager::save_png_to_pictures(&app_handle, &stem, &png_bytes)
        .map_err(|e| e.to_string())?;
    history_item.original_image = img_path.to_string_lossy().to_string();

    // 持久化保存历史
    let mut history = fs_manager::read_history(&app_handle).map_err(|e| e.to_string())?;
    history.insert(0, history_item.clone());
    fs_manager::write_history(&app_handle, &history).map_err(|e| e.to_string())?;

    Ok(history_item)
}

#[tauri::command]
async fn recognize_from_clipboard(
    app_handle: AppHandle,
) -> Result<HistoryItem, String> {
    let config = fs_manager::read_config(&app_handle).map_err(|e| e.to_string())?;
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    let image = clipboard.get_image().map_err(|e| e.to_string())?;
    
    // Convert Arboard's image data to a dynamic image
    let img_buffer = image::ImageBuffer::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.into_owned(),
    )
    .ok_or("Failed to create image buffer from clipboard data")?;
    
    let dynamic_img = image::DynamicImage::ImageRgba8(img_buffer);

    // Encode to PNG and then to base64
    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    dynamic_img
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode clipboard image: {}", e))?;
    let base64_image = general_purpose::STANDARD.encode(&png_bytes);

    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    let model_name = Some(config.default_engine.clone());

    let client = std::sync::Arc::new(ApiClient::new(config.to_llm_config()));

    if config.latex_prompt.trim().is_empty() {
        return Err("LaTeX 提示词未设置。请在设置中填写或点击‘恢复默认提示词’后重试。".to_string());
    }
    if config.analysis_prompt.trim().is_empty() {
        return Err("分析提示词未设置。请在设置中填写或点击‘恢复默认提示词’后重试。".to_string());
    }
    if config.verification_prompt.trim().is_empty() {
        return Err("核查提示词未设置。请在设置中填写或点击‘恢复默认提示词’后重试。".to_string());
    }
    let latex_prompt = {
        let mut p = config.latex_prompt.clone();
        p.push_str(&prompts::format_rule_for_latex(&config.default_latex_format));
        p
    };
    let analysis_prompt = {
        let mut p = config.analysis_prompt.clone();
        let lang = prompts::PromptManager::get_language_constraint_for(prompts::PromptType::Analysis, &config.language);
        p.push_str(&format!("\n\n{}", lang));
        p
    };
    // 第1次和第2次调用同时发出（都只输入图片）
    let latex_task = {
        let c = client.clone();
        let latex_prompt = latex_prompt.clone();
        let img = base64_image.clone();
        tokio::spawn(async move { c.extract_latex(&latex_prompt, &img).await })
    };

    let analysis_task = {
        let c = client.clone();
        let analysis_prompt = analysis_prompt.clone();
        let img = base64_image.clone();
        tokio::spawn(async move { c.generate_analysis(&analysis_prompt, &img).await })
    };

    // 等待第1次调用（LaTeX识别）完成
    let latex = match latex_task.await {
        Ok(Ok(latex)) => latex,
        Ok(Err(e)) => return Err(e.to_string()),
        Err(e) => return Err(format!("LaTeX task failed: {}", e)),
    };
    let prompt_version = determine_prompt_version(&config);
    emit_progress(&app_handle, RecognitionProgressPayload { id: id.clone(), stage: "latex".into(), latex: Some(latex.clone()), title: None, analysis: None, confidence_score: None, created_at: Some(created_at.clone()), original_image: Some(format!("data:image/png;base64,{}", base64_image.clone())), model_name: model_name.clone(), verification: None, prompt_version: Some(prompt_version.clone()), verification_report: None });

    // 第3次调用：在第1次完成后发出（输入图片+LaTeX）
    let verification_prompt = config.verification_prompt.clone();
    let verification_task = {
        let c = client.clone();
        let latex = latex.clone();
        let img = base64_image.clone();
            let verification_prompt = verification_prompt.clone();
        tokio::spawn(async move {
                let vr = c.get_verification_result_with_image(&verification_prompt, &latex, &img)
                    .await
                    .unwrap_or(crate::data_models::VerificationResult { confidence_score: 0, verification_report: "验证失败".to_string() });
                (vr, None)
        })
    };

    // 等待第2次调用（分析）结果
    let (title, analysis) = match analysis_task.await { Ok(Ok(v)) => v, _ => (default_title_for_lang(&config.language), crate::data_models::Analysis { summary: default_summary_for_lang(&config.language), variables: Vec::new(), terms: Vec::new(), suggestions: Vec::new() }) };
    emit_progress(&app_handle, RecognitionProgressPayload { id: id.clone(), stage: "analysis".into(), latex: None, title: Some(title.clone()), analysis: Some(analysis.clone()), confidence_score: None, created_at: None, original_image: None, model_name: model_name.clone(), verification: None, prompt_version: Some(prompt_version.clone()), verification_report: None });

    // 等待第3次调用（验证）结果
    let (verification_result, verification) = match verification_task.await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Verification task failed: {}", e);
            (crate::data_models::VerificationResult {
                confidence_score: 0,
                verification_report: "验证失败".to_string(),
            }, None)
        }
    };
    emit_progress(&app_handle, RecognitionProgressPayload { id: id.clone(), stage: "confidence".into(), latex: None, title: None, analysis: None, confidence_score: Some(verification_result.confidence_score), created_at: None, original_image: None, model_name: model_name.clone(), verification: verification.clone(), prompt_version: Some(prompt_version.clone()), verification_report: Some(verification_result.verification_report.clone()) });

    let mut history_item = HistoryItem {
        id: id.clone(),
        latex,
        title,
        analysis,
        is_favorite: false,
        created_at: created_at.clone(),
        confidence_score: verification_result.confidence_score,
        original_image: base64_image.to_string(),
        model_name: model_name.clone(),
        verification,
        verification_report: Some(verification_result.verification_report),
    };

    // 将图片保存为文件（日期前缀），并用文件路径替换原始图片字段
    let date_str = chrono::DateTime::parse_from_rfc3339(&history_item.created_at)
        .map(|dt| dt.format("%Y%m%d_%H%M%S").to_string())
        .unwrap_or_else(|_| chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string());
    let stem = format!("{}_{}", date_str, history_item.id);
    let img_path = fs_manager::save_png_to_pictures(&app_handle, &stem, &png_bytes)
        .map_err(|e| e.to_string())?;
    history_item.original_image = img_path.to_string_lossy().to_string();

    // 持久化保存历史
    let mut history = fs_manager::read_history(&app_handle).map_err(|e| e.to_string())?;
    history.insert(0, history_item.clone());
    fs_manager::write_history(&app_handle, &history).map_err(|e| e.to_string())?;

    Ok(history_item)
}

#[tauri::command]
async fn recognize_from_image_base64(
    app_handle: AppHandle,
    image_base64: String,
) -> Result<HistoryItem, String> {
    let config = fs_manager::read_config(&app_handle).map_err(|e| e.to_string())?;

    // 输入已是 base64 的 PNG 数据
    let base64_image = image_base64;
    let png_bytes = match base64::engine::general_purpose::STANDARD.decode(&base64_image) {
        Ok(bytes) => bytes,
        Err(e) => return Err(format!("Failed to decode base64 image: {}", e)),
    };

    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    let model_name = Some(config.default_engine.clone());

    let client = std::sync::Arc::new(ApiClient::new(config.to_llm_config()));

    let latex_prompt = if !config.latex_prompt.is_empty() {
        let mut p = config.latex_prompt.clone();
        p.push_str(&prompts::format_rule_for_latex(&config.default_latex_format));
        p
    } else {
        config.custom_prompt.clone()
    };
    let analysis_prompt = if !config.analysis_prompt.is_empty() {
        let mut p = config.analysis_prompt.clone();
        let lang = prompts::PromptManager::get_language_constraint_for(prompts::PromptType::Analysis, &config.language);
        p.push_str(&format!("\n\n{}", lang));
        p
    } else {
        config.custom_prompt.clone()
    };

    // 第1次和第2次调用同时发出（都只输入图片）
    let latex_task = {
        let c = client.clone();
        let latex_prompt = latex_prompt.clone();
        let img = base64_image.clone();
        tokio::spawn(async move { c.extract_latex(&latex_prompt, &img).await })
    };

    let analysis_task = {
        let c = client.clone();
        let analysis_prompt = analysis_prompt.clone();
        let img = base64_image.clone();
        tokio::spawn(async move { c.generate_analysis(&analysis_prompt, &img).await })
    };

    // 等待第1次调用（LaTeX识别）完成
    let latex = match latex_task.await {
        Ok(Ok(latex)) => latex,
        Ok(Err(e)) => return Err(e.to_string()),
        Err(e) => return Err(format!("LaTeX task failed: {}", e)),
    };
    let prompt_version = determine_prompt_version(&config);
    emit_progress(&app_handle, RecognitionProgressPayload { id: id.clone(), stage: "latex".into(), latex: Some(latex.clone()), title: None, analysis: None, confidence_score: None, created_at: Some(created_at.clone()), original_image: Some(format!("data:image/png;base64,{}", base64_image.clone())), model_name: model_name.clone(), verification: None, prompt_version: Some(prompt_version.clone()), verification_report: None });

    // 第3次调用：在第1次完成后发出（输入图片+LaTeX），优先细粒度核查
    let verification_prompt = {
        let mut p = config.verification_prompt.clone();
        let lang = prompts::PromptManager::get_language_constraint_for(prompts::PromptType::Verification, &config.language);
        p.push_str(&format!("\n\n{}", lang));
        p
    };
    let verification_task = {
        let c = client.clone();
        let latex = latex.clone();
        let img = base64_image.clone();
            let verification_prompt = verification_prompt.clone();
        tokio::spawn(async move {
                let vr = c.get_verification_result_with_image(&verification_prompt, &latex, &img)
                    .await
                    .unwrap_or(crate::data_models::VerificationResult { confidence_score: 0, verification_report: "验证失败".to_string() });
                (vr, None)
        })
    };

    // 等待第2次调用（分析）结果
    let (title, analysis) = match analysis_task.await {
        Ok(Ok(v)) => v,
        _ => (
            default_title_for_lang(&config.language),
            crate::data_models::Analysis { summary: default_summary_for_lang(&config.language), variables: Vec::new(), terms: Vec::new(), suggestions: Vec::new() }
        )
    };
    emit_progress(&app_handle, RecognitionProgressPayload { id: id.clone(), stage: "analysis".into(), latex: None, title: Some(title.clone()), analysis: Some(analysis.clone()), confidence_score: None, created_at: None, original_image: None, model_name: model_name.clone(), verification: None, prompt_version: Some(prompt_version.clone()), verification_report: None });

    // 等待第3次调用（验证）结果
    let (verification_result, verification) = match verification_task.await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Verification task failed: {}", e);
            (crate::data_models::VerificationResult {
                confidence_score: 0,
                verification_report: "验证失败".to_string(),
            }, None)
        }
    };
    emit_progress(&app_handle, RecognitionProgressPayload { id: id.clone(), stage: "confidence".into(), latex: None, title: None, analysis: None, confidence_score: Some(verification_result.confidence_score), created_at: None, original_image: None, model_name: model_name.clone(), verification: verification.clone(), prompt_version: Some(prompt_version.clone()), verification_report: Some(verification_result.verification_report.clone()) });

    let mut history_item = HistoryItem {
        id: id.clone(),
        latex,
        title,
        analysis,
        is_favorite: false,
        created_at: created_at.clone(),
        confidence_score: verification_result.confidence_score,
        original_image: base64_image.to_string(),
        model_name: model_name.clone(),
        verification,
        verification_report: Some(verification_result.verification_report),
    };

    // 将图片保存为文件，并替换为路径
    let date_str = chrono::DateTime::parse_from_rfc3339(&history_item.created_at)
        .map(|dt| dt.format("%Y%m%d_%H%M%S").to_string())
        .unwrap_or_else(|_| chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string());
    let stem = format!("{}_{}", date_str, history_item.id);
    let img_path = fs_manager::save_png_to_pictures(&app_handle, &stem, &png_bytes)
        .map_err(|e| e.to_string())?;
    history_item.original_image = img_path.to_string_lossy().to_string();

    // 持久化保存历史
    let mut history = fs_manager::read_history(&app_handle).map_err(|e| e.to_string())?;
    history.insert(0, history_item.clone());
    fs_manager::write_history(&app_handle, &history).map_err(|e| e.to_string())?;

    Ok(history_item)
}
#[tauri::command]
fn copy_image_to_clipboard(image_path: String) -> Result<(), String> {
    // 读取图片并复制到系统剪贴板
    let bytes = std::fs::read(&image_path).map_err(|e| e.to_string())?;
    let dyn_img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let rgba = dyn_img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let img_data = arboard::ImageData {
        width: w as usize,
        height: h as usize,
        bytes: std::borrow::Cow::Owned(rgba.into_raw()),
    };
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_image(img_data).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_image_as_data_url(image_path: String) -> Result<String, String> {
    let bytes = std::fs::read(&image_path).map_err(|e| e.to_string())?;
    let mime = if image_path.to_ascii_lowercase().ends_with(".jpg")
        || image_path.to_ascii_lowercase().ends_with(".jpeg")
    {
        "image/jpeg"
    } else if image_path.to_ascii_lowercase().ends_with(".gif") {
        "image/gif"
    } else {
        // default to png
        "image/png"
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{};base64,{}", mime, encoded))
}

struct HistoryCacheState {
    last_mtime: Option<SystemTime>,
    data: Vec<HistoryItem>,
}

static HISTORY_CACHE: OnceLock<Arc<Mutex<HistoryCacheState>>> = OnceLock::new();

fn init_cache_if_needed() -> Arc<Mutex<HistoryCacheState>> {
    HISTORY_CACHE
        .get_or_init(|| {
            Arc::new(Mutex::new(HistoryCacheState {
                last_mtime: None,
                data: Vec::new(),
            }))
        })
        .clone()
}

#[tauri::command]
fn get_history(app_handle: AppHandle) -> Result<Vec<HistoryItem>, String> {
    let cache = init_cache_if_needed();
    let history_path = fs_manager::get_history_path(&app_handle).map_err(|e| e.to_string())?;
    let mtime = std::fs::metadata(&history_path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    {
        let cache_guard = cache.lock().unwrap();
        if let Some(last) = cache_guard.last_mtime {
            if last == mtime {
                return Ok(cache_guard.data.clone());
            }
        }
    }

    let data = fs_manager::read_history(&app_handle).map_err(|e| e.to_string())?;
    {
        let mut cache_guard = cache.lock().unwrap();
        cache_guard.last_mtime = Some(mtime);
        cache_guard.data = data.clone();
    }
    Ok(data)
}

#[tauri::command]
fn save_to_history(app_handle: AppHandle, item: HistoryItem) -> Result<(), String> {
    let mut history = fs_manager::read_history(&app_handle).map_err(|e| e.to_string())?;
    history.insert(0, item);
    fs_manager::write_history(&app_handle, &history).map_err(|e| e.to_string())?;
    // 更新缓存
    let cache = init_cache_if_needed();
    let mut cache_guard = cache.lock().unwrap();
    cache_guard.data = history;
    cache_guard.last_mtime = std::fs::metadata(
        &fs_manager::get_history_path(&app_handle).map_err(|e| e.to_string())?
    ).and_then(|m| m.modified()).ok();
    Ok(())
}

#[tauri::command]
fn delete_history_item(app_handle: AppHandle, id: String) -> Result<(), String> {
    let mut history = fs_manager::read_history(&app_handle).map_err(|e| e.to_string())?;
    let before_len = history.len();
    history.retain(|item| item.id != id);
    if history.len() == before_len {
        return Err(format!("Item with ID '{}' not found", id));
    }
    fs_manager::write_history(&app_handle, &history).map_err(|e| e.to_string())?;
    let cache = init_cache_if_needed();
    let mut cache_guard = cache.lock().unwrap();
    cache_guard.data = history;
    cache_guard.last_mtime = std::fs::metadata(
        &fs_manager::get_history_path(&app_handle).map_err(|e| e.to_string())?
    ).and_then(|m| m.modified()).ok();
    Ok(())
}

#[tauri::command]
fn update_history_title(
    app_handle: AppHandle,
    id: String,
    title: String,
) -> Result<(), String> {
    let mut history = fs_manager::read_history(&app_handle).map_err(|e| e.to_string())?;
    if let Some(item) = history.iter_mut().find(|item| item.id == id) {
        item.title = title;
        fs_manager::write_history(&app_handle, &history).map_err(|e| e.to_string())?;
        // 更新缓存
        let cache = init_cache_if_needed();
        let mut cache_guard = cache.lock().unwrap();
        cache_guard.data = history;
        cache_guard.last_mtime = std::fs::metadata(
            &fs_manager::get_history_path(&app_handle).map_err(|e| e.to_string())?
        ).and_then(|m| m.modified()).ok();
        Ok(())
    } else {
        Err(format!("Item with ID '{}' not found", id))
    }
}

#[tauri::command]
fn update_favorite_status(
    app_handle: AppHandle,
    id: String,
    // 兼容前端传参：同时支持 snake_case 与 camelCase
    #[allow(non_snake_case)]
    is_favorite: Option<bool>,
    #[allow(non_snake_case)]
    isFavorite: Option<bool>,
) -> Result<(), String> {
    let is_favorite = is_favorite.or(isFavorite).ok_or_else(|| "missing is_favorite/isFavorite".to_string())?;
    let mut history = fs_manager::read_history(&app_handle).map_err(|e| e.to_string())?;
    if let Some(item) = history.iter_mut().find(|item| item.id == id) {
        item.is_favorite = is_favorite;
        fs_manager::write_history(&app_handle, &history).map_err(|e| e.to_string())?;
        let cache = init_cache_if_needed();
        let mut cache_guard = cache.lock().unwrap();
        cache_guard.data = history;
        cache_guard.last_mtime = std::fs::metadata(
            &fs_manager::get_history_path(&app_handle).map_err(|e| e.to_string())?
        ).and_then(|m| m.modified()).ok();
        Ok(())
    } else {
        Err(format!("Item with ID '{}' not found", id))
    }
}

#[tauri::command]
fn get_config(app_handle: AppHandle) -> Result<Config, String> {
    fs_manager::read_config(&app_handle).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(app_handle: AppHandle, config: Config) -> Result<(), String> {
    fs_manager::write_config(&app_handle, &config).map_err(|e| e.to_string())
}

#[tauri::command]
fn register_global_shortcut(app_handle: AppHandle, shortcut: String) -> Result<(), String> {
    // 先取消注册所有现有的快捷键
    app_handle.global_shortcut_manager().unregister_all().map_err(|e| e.to_string())?;

    // 注册新的快捷键
    let app_handle_for_shortcut = app_handle.clone();
    app_handle.global_shortcut_manager().register(&shortcut, move || {
        let app_handle = app_handle_for_shortcut.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(_e) = capture::open_overlays_for_all_displays(app_handle).await {
                #[cfg(debug_assertions)]
                eprintln!("Failed to open overlays from shortcut: {}", _e);
            }
        });
    }).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn get_confidence_score(
    app_handle: AppHandle,
    latex: String,
) -> Result<u8, String> {
    let config = fs_manager::read_config(&app_handle).map_err(|e| e.to_string())?;
    let client = ApiClient::new(config.to_llm_config());
    let verification_prompt = prompts::get_verification_prompt(&config.language);
    let verification_result = client
        .get_verification_result(&verification_prompt, &latex)
        .await
        .map_err(|e| e.to_string())?;
    Ok(verification_result.confidence_score)
}

#[tauri::command]
async fn retry_analysis_phase(
    app_handle: AppHandle,
    image_base64: String,
) -> Result<(String, crate::data_models::Analysis), String> {
    let config = fs_manager::read_config(&app_handle).map_err(|e| e.to_string())?;
    let client = ApiClient::new(config.to_llm_config());
    let analysis_prompt = if !config.analysis_prompt.is_empty() {
        prompts::get_analysis_prompt(&config.language)
    } else {
        config.custom_prompt.clone()
    };

    let result = client
        .generate_analysis(&analysis_prompt, &image_base64)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
async fn retry_verification_phase(
    app_handle: AppHandle,
    latex: String,
    image_base64: String,
) -> Result<(crate::data_models::VerificationResult, Option<crate::data_models::Verification>), String> {
    let config = fs_manager::read_config(&app_handle).map_err(|e| e.to_string())?;
    let client = ApiClient::new(config.to_llm_config());
    let verification_prompt = prompts::get_verification_prompt(&config.language);

    match client.verify_latex_against_image(&latex, &image_base64, &config.language).await {
        Ok(v) => {
            let vr = compute_verification_result_from_struct(&v);
            Ok((vr, Some(v)))
        }
        Err(_) => {
            let fallback = client
                .get_verification_result_with_image(&verification_prompt, &latex, &image_base64)
                .await
                .unwrap_or(crate::data_models::VerificationResult { confidence_score: 0, verification_report: "验证失败".to_string() });
            Ok((fallback, None))
        }
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // 读取配置并应用窗口大小/位置
            let app_handle = app.handle();
            let cfg = fs_manager::read_config(&app_handle).unwrap_or_default();

            // 注册全局快捷键
            let shortcut = cfg.screenshot_shortcut.clone();
            let app_handle_for_shortcut = app_handle.clone();
            if let Err(_e) = app.global_shortcut_manager().register(&shortcut, move || {
                let app_handle = app_handle_for_shortcut.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = capture::open_overlays_for_all_displays(app_handle).await {
                        eprintln!("Failed to open overlays from shortcut: {}", e);
                    }
                });
            }) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to register global shortcut '{}': {}", shortcut, _e);
            }
            if let Some(win) = app.get_window("main") {
                // 设置窗口图标为自定义 ICO（Windows 任务栏与标题栏图标）
                // 设置窗口图标（ICO/PNG 由 tauri-icon 特性支持）
                // 优先使用高质量 PNG 作为窗口图标，避免 ICO 在某些环境渲染异常
                if let Some(png_path) = app.path_resolver().resolve_resource("icons/icon-256.png") {
                    let _ = win.set_icon(tauri::Icon::File(png_path));
                } else if let Some(ico_path) = app.path_resolver().resolve_resource("icons/icon.ico") {
                    let _ = win.set_icon(tauri::Icon::File(ico_path));
                }
                // 设置尺寸
                use tauri::PhysicalSize;
                let _ = win.set_size(PhysicalSize::new(cfg.window_width, cfg.window_height));
                // 设置位置（可选）
                if let (Some(x), Some(y)) = (cfg.window_x, cfg.window_y) {
                    use tauri::PhysicalPosition;
                    let _ = win.set_position(PhysicalPosition::new(x, y));
                }
            }

            // 监听关闭时保存窗口位置与尺寸
            if let Some(win) = app.get_window("main") {
                let app_handle_clone = app_handle.clone();
                let win_clone = win.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        // 读取当前配置，写回窗口状态（仅在 remember_window_state 为 true 时）
                        if let Ok(mut cfg) = fs_manager::read_config(&app_handle_clone) {
                            if cfg.remember_window_state {
                                if let Ok(size) = win_clone.inner_size() {
                                    cfg.window_width = size.width;
                                    cfg.window_height = size.height;
                                }
                                if let Ok(pos) = win_clone.outer_position() {
                                    cfg.window_x = Some(pos.x);
                                    cfg.window_y = Some(pos.y);
                                }
                                let _ = fs_manager::write_config(&app_handle_clone, &cfg);
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            test_connection,
            open_config_dir,
            recognize_from_screenshot,
            recognize_from_file,
            recognize_from_clipboard,
            recognize_from_image_base64,
            get_history,
            save_to_history,
            delete_history_item,
            update_favorite_status,
            update_history_title,
            get_config,
            save_config,
            register_global_shortcut,
            get_confidence_score,
            copy_image_to_clipboard,
            read_image_as_data_url,
            get_default_prompts,
            get_full_prompts_with_language,
            get_prompt_parts,
            retry_analysis_phase,
            retry_verification_phase,
            capture::open_overlays_for_all_displays,
            capture::complete_capture,
            capture::close_all_overlays,
            capture::start_recognition_from_region_capture
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
