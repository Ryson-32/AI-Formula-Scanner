// 统一的提示词管理模块
// 负责管理所有LLM调用的提示词，包括基础版本和语言约束版本

/// 提示词类型枚举
#[derive(Debug, Clone)]
pub enum PromptType {
    LaTeX,
    Analysis,
    Verification, // 原置信度评分，现在改为验证（包含置信度和核查报告）
}

/// 语言类型
#[derive(Debug, Clone)]
pub enum Language {
    Chinese,
    English,
}

impl From<&str> for Language {
    fn from(lang: &str) -> Self {
        match lang {
            "zh-CN" => Language::Chinese,
            _ => Language::English,
        }
    }
}

/// 提示词管理器
pub struct PromptManager;

impl PromptManager {
    /// 获取基础提示词（不含语言约束）
    pub fn get_base_prompt(prompt_type: PromptType) -> String {
        match prompt_type {
            PromptType::LaTeX => Self::base_latex_prompt(),
            PromptType::Analysis => Self::base_analysis_prompt(),
            PromptType::Verification => Self::base_verification_prompt(),
        }
    }

    /// 获取完整提示词（含语言约束）
    pub fn get_full_prompt(prompt_type: PromptType, language: Language) -> String {
        let base = Self::get_base_prompt(prompt_type.clone());
        let constraint = Self::get_language_constraint(prompt_type, language);
        format!("{}\n\n{}", base, constraint)
    }

    /// 获取语言约束
    fn get_language_constraint(prompt_type: PromptType, language: Language) -> String {
        match prompt_type {
            PromptType::LaTeX => Self::latex_language_constraint(language),
            PromptType::Analysis => Self::analysis_language_constraint(language),
            PromptType::Verification => Self::verification_language_constraint(language),
        }
    }

    // === 基础提示词定义 ===

    fn base_latex_prompt() -> String {
        "You are an expert in LaTeX OCR. Task: Given an image of a mathematical formula, EXTRACT THE LaTeX EXACTLY as shown in the image.

Never correct math, never infer intent, never simplify/normalize. Strictly preserve spacing/notation, symbol forms, order of terms, matrix layout, and the distinction between scalars vs vectors/tensors (e.g., boldface, overarrow, blackboard vs italic, uppercase/lowercase, indices). Do not convert a scalar to a vector/tensor or vice versa.

Brackets: Pay extreme attention to bracket KINDS and COUNTS. Use the exact types that appear and do not add/remove/misuse them: parentheses (), square brackets [], curly braces {}, and angle brackets ⟨⟩ when present. Ensure LaTeX grouping braces {} are balanced and minimal (no extra {}).

Command integrity: NEVER drop the leading backslash of LaTeX commands/environments. Always output commands with their backslashes, e.g., \\begin{bmatrix} ... \\end{bmatrix}, \\frac{...}{...}, \\partial, \\alpha. Do not output truncated tokens like \"egin\", \"frac\", etc.

Noise handling: Ignore any non-formula artifacts captured in the screenshot (e.g., Word paragraph marks ↵, tab arrows ↹, UI chrome, page/section labels, figure captions, or reference tags like [1]). Transcribe ONLY the actual formula content. Do NOT add references, citations, or links.

Output only a strict JSON object: {\"latex\": \"...\"}. No Markdown, no comments, no extra text. Ensure JSON validity: escape every backslash in LaTeX for JSON (e.g., \\\\frac).".to_string()
    }

    fn base_analysis_prompt() -> String {
        "You are an expert in mathematics, physics, and technical writing. Based on the provided formula image (DO NOT change the formula), produce a structured analysis JSON with the following fields only: {\"title\": \"...\", \"analysis\": {\"summary\": \"...\", \"variables\": [{\"symbol\": \"...\", \"description\": \"...\", \"unit\": \"?\"}], \"terms\": [{\"name\": \"...\", \"description\": \"...\"}], \"suggestions\": [{\"type\": \"error|warning|info\", \"message\": \"...\"}]}}.

Instructions:
1) Variables: enumerate every symbol that appears (parameters, fields, operators like ∇ optional). For each, give a concise meaning and typical SI unit if applicable. If unit is unknown, use \"?\".
2) Terms: identify each distinct term/expression/sub-expression in the equation(s) (e.g., derivatives, integrals, summations, products, norms, matrix/vector operations, source terms). Provide a one-sentence physical/mathematical meaning for each.
3) Suggestions (three levels):
   - error: Hard mistakes such as dimensional inconsistency, impossible identities, wrong operators, missing brackets causing invalid grammar, or evident OCR mistakes leading to invalid math.
   - warning: Unusual or risky presentation that can hinder readability or typesetting (e.g., extremely long expressions likely to overflow, unconventional notation like uu instead of u^2 though intentionally preserved, ambiguous symbols).
   - info: General improvement advice (naming clarity, add definitions, add context equations or equivalent forms).
4) Scalar vs tensor: Pay special attention to the distinction between scalars and vectors/tensors (e.g., bold/arrow notation, indices). Preserve this distinction in variable descriptions and term explanations; do not convert between them.
5) References: Do NOT add references/citations/links anywhere (e.g., [1], (Smith, 2020)).
6) Output must be a strict JSON object with the exact schema above. No Markdown, no code fences, no extra commentary.".to_string()
    }

    fn base_verification_prompt() -> String {
        "You are a meticulous verification expert. Your task is to carefully compare the provided LaTeX code against the original mathematical formula image and provide both a confidence score and a detailed verification report.

Task: Analyze how accurately the LaTeX code represents the original image by examining:
1) Symbol accuracy: Are all symbols correctly identified and transcribed?
2) Structure fidelity: Do exponents, subscripts, fractions, and groupings match exactly?
3) Operator precision: Are mathematical operators (+, -, ×, ÷, =, etc.) correctly placed?
4) Layout consistency: Does the overall mathematical structure and spacing match?
5) Completeness: Are there any missing or extra elements?
6) Scalar vs tensor distinction: Treat mismatches between scalars and vectors/tensors (e.g., bold/arrow notation, indexing/ordering conveying tensor rank) as meaning-changing errors.

Output a strict JSON object with this exact schema:
{
  \"confidence_score\": 0-100,
  \"verification_report\": \"A concise but thorough report detailing any discrepancies found between the LaTeX and the original image. If perfect match, state 'LaTeX accurately represents the original formula.' If issues found, describe specific problems like 'Missing subscript in variable x', 'Incorrect operator placement', or 'Vector/tensor vs scalar mismatch', etc.\"
}

Be precise and objective in your assessment. No Markdown formatting, no code fences, no extra commentary.".to_string()
    }

    // === 语言约束定义 ===

    fn latex_language_constraint(language: Language) -> String {
        match language {
            Language::Chinese => "Important: Use Simplified Chinese for any error messages or explanations if needed. Keep JSON keys in English.",
            Language::English => "Important: Use English for any error messages or explanations if needed. Keep JSON keys in English.",
        }.to_string()
    }

    fn analysis_language_constraint(language: Language) -> String {
        match language {
            Language::Chinese => "Important: Use Simplified Chinese for the values of 'title', 'analysis.summary', 'analysis.variables[*].description', 'analysis.terms[*].description', and 'analysis.suggestions[*].message'. Keep JSON keys in English.",
            Language::English => "Important: Use English for the values of 'title', 'analysis.summary', 'analysis.variables[*].description', 'analysis.terms[*].description', and 'analysis.suggestions[*].message'. Keep JSON keys in English.",
        }.to_string()
    }

    fn verification_language_constraint(language: Language) -> String {
        match language {
            Language::Chinese => "Important: Use Simplified Chinese for the 'verification_report' content. Keep JSON keys in English.",
            Language::English => "Important: Use English for the 'verification_report' content. Keep JSON keys in English.",
        }.to_string()
    }

    /// 对外暴露：获取指定提示类型与语言的语言约束文案
    pub fn get_language_constraint_for(prompt_type: PromptType, language: &str) -> String {
        let lang = Language::from(language);
        Self::get_language_constraint(prompt_type, lang)
    }
}

// === 便捷函数 ===

/// 获取分析提示词
pub fn get_analysis_prompt(language: &str) -> String {
    PromptManager::get_full_prompt(PromptType::Analysis, Language::from(language))
}

/// 获取验证提示词（原置信度评分）
pub fn get_verification_prompt(language: &str) -> String {
    PromptManager::get_full_prompt(PromptType::Verification, Language::from(language))
}

/// 获取所有基础提示词（用于设置页面显示）
pub fn get_base_prompts_tuple() -> (String, String, String) {
    (
        PromptManager::get_base_prompt(PromptType::LaTeX),
        PromptManager::get_base_prompt(PromptType::Analysis),
        PromptManager::get_base_prompt(PromptType::Verification),
    )
}

/// 依据设置的默认 LaTeX 格式，返回附加到提示词末尾的格式化规则说明。
/// 注意：这仅添加明确说明，真实的包裹仍由模型在 JSON 中返回的 "latex" 值完成。
pub fn format_rule_for_latex(default_format: &str) -> String {
    let rule = match default_format {
        // 不加任何定界符
        "raw" => "\n\nFormatting rule: Return the bare LaTeX body ONLY inside the JSON value without any math delimiters (no $...$, no $$...$$, no \\[...\\], no \\begin{equation}...\\end{equation}). Place the exact LaTeX string in the 'latex' field.",
        // $...$
        "single_dollar" => "\n\nFormatting rule: Wrap the entire LaTeX with $...$ (inline math). The JSON must be {\"latex\": \"$<content>$\"}.",
        // $$...$$
        "double_dollar" => "\n\nFormatting rule: Wrap the entire LaTeX with $$...$$ (display math). The JSON must be {\"latex\": \"$$<content>$$\"}.",
        // \begin{equation}...\end{equation}
        "equation" => "\n\nFormatting rule: Wrap the entire LaTeX with \\begin{equation} ... \\end{equation}. The JSON must be {\"latex\": \"\\begin{equation}<content>\\end{equation}\"}.",
        // \[ ... \]
        "bracket" => "\n\nFormatting rule: Wrap the entire LaTeX with \\[ ... \\] (display math). The JSON must be {\"latex\": \"\\[<content>\\]\"}.",
        // 兜底（与 raw 一致）
        _ => "\n\nFormatting rule: Return the bare LaTeX body ONLY without any math delimiters and put it into the 'latex' field.",
    };
    format!("{}{}", rule, " IMPORTANT: The response MUST be a valid JSON object. Escape every backslash in LaTeX for JSON (e.g., \\\\frac). No Markdown fences.")
}


