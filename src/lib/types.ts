export interface Config {
  apiKey: string;
  apiBaseUrl: string;
  provider: string;
  defaultEngine: string;
  latexPrompt: string;
  analysisPrompt: string;
  // 前端统一字段：verificationPrompt（兼容旧字段名 confidencePrompt 由后端 serde alias 处理）
  verificationPrompt: string;
  renderEngine: string;
  autoCalculateConfidence: boolean;
  enableClipboardWatcher: boolean;
  defaultLatexFormat: string;
  requestTimeoutSeconds: number;
  maxRetries: number;
  maxOutputTokens: number;
  language: 'zh-CN' | 'en';
  // window state
  windowWidth: number;
  windowHeight: number;
  windowX?: number | null;
  windowY?: number | null;
  rememberWindowState: boolean;
}

export interface RecognitionResult {
  id: string;
  latex: string;
  title: string;
  analysis: {
    summary: string;
    variables: Array<{ symbol: string; description: string; unit?: string | null }>;
    terms: Array<{ name: string; description: string }>;
    suggestions: Array<{
      type: string;
      message: string;
    }>;
  };
  is_favorite: boolean;
  created_at: string;
  confidence_score: number;
  original_image: string;
  model_name?: string;
  prompt_version?: string;
  verification?: {
    status: 'error' | 'warning' | 'ok' | string;
    issues?: Array<{ category: string; message: string }>;
    coverage?: { symbols_matched: number; symbols_total: number; terms_matched: number; terms_total: number };
  };
  // 当结构化 verification 缺失时，后端可能仅提供文字报告作为兜底
  verification_report?: string;
}