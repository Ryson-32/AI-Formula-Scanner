<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/tauri';
  import { open } from '@tauri-apps/api/dialog';
  import { readBinaryFile } from '@tauri-apps/api/fs';
  import { clipboard } from '@tauri-apps/api';
  import type { Config } from '$lib/types';
  import { currentLang, translateNow } from '$lib/i18n';
  import { recognitionStore } from '$lib/recognitionStore';

  import FormulaRenderer from './FormulaRenderer.svelte';
  import LatexEditor from './LatexEditor.svelte';
  import VerificationReportRenderer from './VerificationReportRenderer.svelte';
  import { historyStore } from '$lib/historyStore';
  import { RotateCcw, Image as ImageIcon, Star as StarIcon, Bell, AlertTriangle, AlertCircle } from 'lucide-svelte';

  // --- LLM 三阶段指示灯状态 ---
  type PhaseStatus = 'idle' | 'pending' | 'done' | 'error';
  let phase: { latex: PhaseStatus; analysis: PhaseStatus; verify: PhaseStatus } = {
    latex: 'idle',
    analysis: 'idle',
    verify: 'idle'
  };
  function resetPhaseForStart() {
    phase = { latex: 'pending', analysis: 'pending', verify: 'idle' };
    persistPhase();
  }
  // 将当前 phase 持久化，跨页恢复
  function persistPhase() {
    try { localStorage.setItem('phaseState', JSON.stringify(phase)); } catch {}
  }

  // 使用全局识别状态（跨路由保留）
  let originalImageSrc = '';
  let isOriginalExpanded = false;
  let activeTab: 'basic' | 'analysis' = 'basic';

  // 分析页面折叠状态
  let isVariablesExpanded = false;
  let isTermsExpanded = false;
  let isSuggestionsExpanded = false;
  let isVerificationExpanded = false;

  // 获取配置
  let apiKey = '';
  let defaultEngine = '';
  let renderEngine = 'MathJax';
  let defaultLatexFormat = 'raw';
  let screenshotShortcut = 'CommandOrControl+Shift+A';

  // 记录最后的操作类型，用于重试功能
  let lastOperation: 'file' | 'region' | null = null;

  // 指示灯是否应该显示（一旦开始调用就一直显示）
  let showPhaseStatus = false;

  // 展示用标签
  $: engineDisplay = renderEngine || 'MathJax';
  $: formatDisplay = (() => {
    const keyMap: Record<string, string> = {
      raw: 'settings.display.format.raw',
      single_dollar: 'settings.display.format.single',
      double_dollar: 'settings.display.format.double',
      equation: 'settings.display.format.equation',
      bracket: 'settings.display.format.bracket'
    };
    const k = keyMap[defaultLatexFormat] ?? 'settings.display.format.raw';
    return translateNow(k, $currentLang);
  })();
  // 已移除旧版 customPrompt 使用
  let customPrompt = '';
  let titleSizeClass = 'title-md';

  // 格式化快捷键显示
  function formatShortcut(shortcut: string): string {
    return shortcut
      .replace('CommandOrControl', 'Ctrl')
      .replace('Shift', 'Shift')
      .replace('+', '+');
  }

  // 动态显示快捷键
  $: shortcutDisplay = formatShortcut(screenshotShortcut);

  // 计算建议的最高等级（error > warning > info）
  function computeHighestLevel(suggestions: Array<{ type: string }>): 0 | 1 | 2 | 3 {
    let lvl: 0 | 1 | 2 | 3 = 0;
    for (const s of suggestions || []) {
      const t = (s.type || '').toLowerCase();
      if (t === 'error') lvl = 3;
      else if (t === 'warning' && lvl < 2) lvl = 2;
      else if (t === 'info' && lvl < 1) lvl = 1;
      if (lvl === 3) break;
    }
    return lvl;
  }

  // 综合状态计算（第二次LLM的建议 + 第三次LLM的验证状态）
  function computeOverallStatus(suggestions: Array<{ type: string }>, verification?: any): { level: 0 | 1 | 2 | 3, label: string, icon: string } {
    const suggestionLevel = computeHighestLevel(suggestions);
    let verificationLevel: 0 | 1 | 2 | 3 = 0;

    // 根据verification状态确定等级
    if (verification?.status === 'error') verificationLevel = 3;
    else if (verification?.status === 'warning') verificationLevel = 2;
    else if (verification?.status === 'ok') verificationLevel = 0;

    // 取最高等级
    const finalLevel = Math.max(suggestionLevel, verificationLevel) as 0 | 1 | 2 | 3;

    let label = '';
    let icon = '';

    switch (finalLevel) {
      case 3:
        label = 'error';
        icon = '❌';
        break;
      case 2:
        label = 'warning';
        icon = '⚠️';
        break;
      case 1:
        label = 'info';
        icon = 'ℹ️';
        break;
      default:
        label = 'ok';
        icon = '✔️';
        break;
    }

    return { level: finalLevel, label, icon };
  }

  $: overallStatus = computeOverallStatus(
    $recognitionStore.result?.analysis?.suggestions ?? [],
    $recognitionStore.result?.verification
  );

  let unlistenProgress: (() => void) | undefined;
  let unlistenRegionCapture: (() => void) | undefined;

  onMount(async () => {
    try {
      const config = await invoke<Config>('get_config');
      apiKey = config.apiKey;
      defaultEngine = config.defaultEngine;
      renderEngine = (config as any).renderEngine ?? (config as any).render_engine ?? 'MathJax';
      defaultLatexFormat = (config as any).defaultLatexFormat ?? (config as any).default_latex_format ?? 'raw';
      screenshotShortcut = (config as any).screenshotShortcut ?? (config as any).screenshot_shortcut ?? 'CommandOrControl+Shift+A';
      // customPrompt 已弃用
    } catch (err) {
      const error = err as Error;
      console.error('Failed to load config:', error);
      recognitionStore.setError(`${translateNow('history.load_failed', $currentLang)}: ${error.message}`);
    }

    // 若从历史页进入，加载选中的历史项到结果视图
    try {
      const cached = localStorage.getItem('selectedItem');
      if (cached) {
        const raw = JSON.parse(cached);
        const item = normalizeResult(raw);
        // 不修改原始字段，保持文件路径；显示由 originalImageSrc 响应式生成
        recognitionStore.setResult(item as any);
        localStorage.removeItem('selectedItem');
      }
    } catch (e) {
      // ignore parse errors
    }

    // 读取原始图片折叠记忆
    try {
      const saved = localStorage.getItem('originalImageExpanded');
      if (saved !== null) isOriginalExpanded = saved === 'true';
    } catch {}

    // 挂载时尝试从本地恢复指示灯状态
    try {
      const saved = localStorage.getItem('phaseState');
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed && typeof parsed === 'object' && parsed.latex && parsed.analysis && parsed.verify) {
          phase = parsed;
        }
      }
    } catch {}

    // 监听后端阶段事件，实时更新 UI + 指示灯
    try {
      const { listen } = await import('@tauri-apps/api/event');
      unlistenProgress = await listen('recognition_progress', (e: any) => {
        const p = e?.payload as any;
        if (!p || typeof p !== 'object') return;
        if (p.stage === 'latex' && p.latex) {
          recognitionStore.patch({ id: p.id, latex: p.latex, created_at: p.created_at ?? '', original_image: p.original_image ?? '', model_name: p.model_name });
          // 第一阶段完成：latex=done，verify 开始等待
          phase.latex = 'done';
          if (phase.verify === 'idle') phase.verify = 'pending';
          persistPhase();
        } else if (p.stage === 'analysis' && p.analysis) {
          
          recognitionStore.patch({ title: p.title ?? '', analysis: p.analysis });
          phase.analysis = 'done';
          persistPhase();
          // 分析完成后，自动切换到分析tab以显示结果
          if (activeTab === 'basic') {
            activeTab = 'analysis';
          }
          
        } else if (p.stage === 'confidence' && typeof p.confidence_score === 'number') {
          const patch: any = { confidence_score: p.confidence_score };
          if (p.verification) patch.verification = p.verification;
          if (p.verification_report && !$recognitionStore.result?.verification) {
            // 回退路径兜底：若没有结构化 verification，但有文字报告，存到结果中以供渲染
            (patch as any).verification_report = p.verification_report;
          }
          recognitionStore.patch(patch);
          recognitionStore.setLoading(false);
          phase.verify = 'done';
          persistPhase();
        }
      });

      // 监听区域截图完成事件
      unlistenRegionCapture = await listen('region-capture-completed', async (event: any) => {
        const imagePath = event.payload as string;

        // 开始识别流程
        await startRecognitionFromImagePath(imagePath);
      });
    } catch {}
  });

  // 使用 onDestroy 清理事件监听器
  onDestroy(() => {
    if (unlistenProgress) {
      unlistenProgress();
    }
    if (unlistenRegionCapture) {
      unlistenRegionCapture();
    }
  });

  function assetUrlToFsPath(urlStr: string): string {
    try {
      if (/^asset:/.test(urlStr) || /^https?:\/\/asset\.localhost/i.test(urlStr)) {
        const u = new URL(urlStr.replace(/^asset:\/\//, 'https://'));
        let p = decodeURIComponent(u.pathname);
        if (/^\/[A-Za-z]:/.test(p)) p = p.slice(1);
        if (p.includes('/')) p = p.replace(/\//g, '\\');
        return p;
      }
    } catch {}
    return urlStr;
  }

  async function toDisplaySrc(path: string | undefined): Promise<string> {
    if (!path) return '';
    if (/^(data:|https?:|tauri:)/i.test(path)) return path;
    const fsPath = assetUrlToFsPath(path);
    try {
      return await invoke<string>('read_image_as_data_url', { image_path: fsPath, imagePath: fsPath });
    } catch (e) {
      console.error('read_image_as_data_url failed:', e, 'path=', fsPath);
      return '';
    }
  }

  // 统一归一化后端/缓存返回的字段命名（camelCase/snake_case 兼容）
  function normalizeResult(raw: any) {
    if (!raw) return raw;
    return {
      id: raw.id,
      latex: raw.latex,
      title: raw.title,
      analysis: raw.analysis,
      is_favorite: raw.is_favorite ?? raw.isFavorite ?? false,
      created_at: raw.created_at ?? raw.createdAt ?? '',
      confidence_score: raw.confidence_score ?? raw.confidenceScore ?? 0,
      original_image: raw.original_image ?? raw.originalImage ?? '',
      model_name: raw.model_name ?? raw.modelName,
      verification: raw.verification
    } as any;
  }


  // 响应式更新图片源
  $: if ($recognitionStore.result?.original_image) {
    toDisplaySrc($recognitionStore.result.original_image).then(src => {
      originalImageSrc = src;
    });
  }

  // 采用行内原图展示（不使用弹窗）
  function toggleOriginalInline() {
    if (!originalImageSrc && $recognitionStore.result?.original_image) {
      toDisplaySrc($recognitionStore.result.original_image).then((src) => {
        originalImageSrc = src;
        isOriginalExpanded = !isOriginalExpanded;
        try { localStorage.setItem('originalImageExpanded', String(isOriginalExpanded)); } catch {}
      });
    } else {
      isOriginalExpanded = !isOriginalExpanded;
      try { localStorage.setItem('originalImageExpanded', String(isOriginalExpanded)); } catch {}
    }
  }

  // 日期格式化
  function formatDate(dateString: string): string {
    const date = new Date(dateString);
    if (isNaN(date.getTime())) return '—';
    const locale = $currentLang === 'en' ? 'en-US' : 'zh-CN';
    return date.toLocaleString(locale, {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    });
  }

  async function ensureConfigLoaded() {
    if (apiKey && defaultEngine) return;
    try {
      const config = await invoke<Config>('get_config');
      apiKey = config.apiKey;
      defaultEngine = config.defaultEngine;
      renderEngine = (config as any).renderEngine ?? (config as any).render_engine ?? 'MathJax';
      defaultLatexFormat = (config as any).defaultLatexFormat ?? (config as any).default_latex_format ?? 'raw';
      screenshotShortcut = (config as any).screenshotShortcut ?? (config as any).screenshot_shortcut ?? 'CommandOrControl+Shift+A';
    } catch (err) {
      // ignore here; caller will handle error state
    }
  }

  // 根据标题长度自适应字号（最多两行展示）
  $: {
    const len = $recognitionStore.result?.title?.length ?? 0;
    if (len <= 28) titleSizeClass = 'title-xl';
    else if (len <= 48) titleSizeClass = 'title-lg';
    else if (len <= 72) titleSizeClass = 'title-md';
    else titleSizeClass = 'title-sm';
  }



  // 文件导入识别
  async function recognizeFromFile() {
    await ensureConfigLoaded();
    if (!apiKey) {
      recognitionStore.setError(translateNow('recognition.error.config_missing', $currentLang));
      return;
    }

    try {
      // 打开文件选择器
      const selected = await open({
        multiple: false,
        filters: [{
          name: '图片',
          extensions: ['png', 'jpg', 'jpeg']
        }]
      });

      if (selected === null) {
        // 用户取消了选择
        return;
      }

      // 记录操作类型用于重试
      lastOperation = 'file';

      // 分阶段：先清空并进入loading，并将1/2阶段置为并发 pending
      recognitionStore.setResult({ id: '', latex: '', title: '', analysis: { summary: '', variables: [], terms: [], suggestions: [] }, is_favorite: false, created_at: '', confidence_score: 0, original_image: '' } as any);
      recognitionStore.start();
      resetPhaseForStart(); persistPhase();

      // 调用后端识别（传递文件路径，由后端读取）
      const filePath = selected as string;
      const result = await invoke('recognize_from_file', {
        filePath
      });
      const item = normalizeResult(result as any);
      // 事件驱动优先；无事件时兜底补丁（测试/非Tauri环境）
      if (!$recognitionStore.result?.latex) {
        recognitionStore.patch({
          id: item.id,
          latex: item.latex,
          created_at: item.created_at,
          original_image: item.original_image,
          model_name: item.model_name
        } as any);
      }
      historyStore.refresh();
    } catch (err) {
      const error = err as Error;
      console.error('File recognition failed:', error);
      if (phase.verify === 'pending') phase.verify = 'error';
      if (phase.analysis === 'pending') phase.analysis = 'error';
      if (phase.latex === 'pending') phase.latex = 'error';
      persistPhase();
      const msg = String(error.message || error);
      const m1 = msg.match(/status (\d{3})/i);
      const m2 = msg.match(/code[:=]?(\s*)(\d{3})/i);
      const code = m1 ? m1[1] : (m2 ? m2[2] : undefined);
      const fr = msg.match(/finishReason:\s*([A-Z_]+)/)?.[1];
      if (fr) {
        if (fr === 'MAX_TOKENS') {
          recognitionStore.setError(`${translateNow('recognition.finish_reason.max_tokens', $currentLang)}: ${msg}`);
        } else if (fr === 'STOP') {
          recognitionStore.setError(`${translateNow('recognition.finish_reason.stop', $currentLang)}: ${msg}`);
        } else {
          recognitionStore.setError(`${translateNow('recognition.error.finish_reason', $currentLang).replace('{reason}', fr)}: ${msg}`);
        }
      } else if (code) {
        recognitionStore.setError(`${translateNow('recognition.file.error_failed_code', $currentLang).replace('{code}', code)}: ${msg}`);
      } else {
        recognitionStore.setError(`${translateNow('recognition.file.error_failed', $currentLang)}: ${msg}`);
      }
    }
  }

  // 更新LaTeX
  function updateLatex(event: CustomEvent<{ latex: string }>) {
    if ($recognitionStore.result) {
      recognitionStore.updateLatex(event.detail.latex);
    }
  }

  function handleTitleInput(e: Event) {
    const el = e.target as HTMLElement;
    const t = (el && (el.innerText ?? el.textContent)) || '';
    recognitionStore.patch({ title: t });
  }

  async function handleTitleBlur(e: Event) {
    const el = e.target as HTMLElement;
    const t = (el && (el.innerText ?? el.textContent)) || '';
    if ($recognitionStore.result && $recognitionStore.result.id) {
      try {
        await invoke('update_history_title', { id: $recognitionStore.result.id, title: t });
      } catch {}
    }
  }

  // 切换收藏状态
  async function toggleFavorite() {
    if (!$recognitionStore.result) return;

    try {
      await invoke('update_favorite_status', {
        id: $recognitionStore.result.id,
        isFavorite: !$recognitionStore.result.is_favorite
      });

      recognitionStore.patch({ is_favorite: !$recognitionStore.result.is_favorite });
      // 同步全局缓存
      historyStore.updateItem($recognitionStore.result.id, { is_favorite: !$recognitionStore.result.is_favorite });
    } catch (err) {
      const msg = typeof err === 'string'
        ? err
        : (err && typeof err === 'object' && 'message' in err && typeof (err as any).message === 'string')
          ? (err as any).message
          : (() => { try { return JSON.stringify(err); } catch { return String(err); } })();
      console.error('Failed to update favorite status:', err);
      recognitionStore.setError(`${translateNow('history.update_failed', $currentLang)}: ${msg}`);
    }
  }

  // 复制LaTeX（严格复制文本框内容，不做任何加工）
  async function copyLatex() {
    if (!$recognitionStore.result) return;

    try {
      const rawLatex = $recognitionStore.result.latex ?? '';
      await clipboard.writeText(rawLatex);
      const { showToast } = await import('$lib/toast');
      showToast(translateNow('recognition.copy_latex_success', $currentLang), 'success');
    } catch (err) {
      const error = err as Error;
      console.error('Failed to copy LaTeX:', error);
      recognitionStore.setError(`${translateNow('recognition.copy_latex_failed', $currentLang)}: ${error.message}`);
    }
  }

  // 复制图片
  // 复制图片功能已移除（精简按钮区）

  // 保存到历史
  async function saveToHistory() {
    if (!$recognitionStore.result) return;

    try {
      await invoke('save_to_history', { item: $recognitionStore.result });
      historyStore.add($recognitionStore.result as any);
      const { showToast } = await import('$lib/toast');
      showToast(translateNow('recognition.save_history_success', $currentLang), 'success');
    } catch (err) {
      const error = err as Error;
      console.error('Failed to save to history:', error);
      recognitionStore.setError(`${translateNow('recognition.save_history_failed', $currentLang)}: ${error.message}`);
    }
  }

  // —— 指示灯重试：按阶段重试 ——
  async function retryPhase(which: 'latex' | 'analysis' | 'verify') {
    if (which === 'latex') {
      // 根据最后的操作类型进行重试
      if (lastOperation === 'region') {
        await recognizeFromRegion();
      } else if (lastOperation === 'file') {
        await recognizeFromFile();
      }
      return;
    }
    if (!$recognitionStore.result) return;

    if (which === 'analysis') {
      // 使用当前图片 base64 重跑分析
      if (!$recognitionStore.result.original_image) return;
      phase.analysis = 'pending'; persistPhase();
      try {
        const img = $recognitionStore.result.original_image.replace('data:image/png;base64,', '');
        const result = await invoke('retry_analysis_phase', { imageBase64: img, image_base64: img });
        const [title, analysis] = result as [string, any];
        recognitionStore.patch({ title, analysis });
        phase.analysis = 'done'; persistPhase();
      } catch (err) {
        phase.analysis = 'error'; persistPhase();
      }
      return;
    }

    if (which === 'verify') {
      if (!$recognitionStore.result.latex || !$recognitionStore.result.original_image) return;
      phase.verify = 'pending'; persistPhase();
      try {
        const img = $recognitionStore.result.original_image.replace('data:image/png;base64,', '');
        const result = await invoke('retry_verification_phase', { latex: $recognitionStore.result.latex, imageBase64: img, image_base64: img });
        const [verificationResult, verification] = result as [any, any];
        recognitionStore.patch({ confidence_score: verificationResult.confidence_score, verification });
        phase.verify = 'done'; persistPhase();
      } catch (err) {
        phase.verify = 'error'; persistPhase();
      }
      return;
    }
  }

  // 置信度检查逻辑统一到 retryPhase('verify')

  // 区域选择截图识别
  async function recognizeFromRegion() {
    await ensureConfigLoaded();
    if (!apiKey) {
      recognitionStore.setError(translateNow('recognition.error.config_missing', $currentLang));
      return;
    }

    try {
      // 打开遮罩窗口进行区域选择
      await invoke('open_overlays_for_all_displays');

      // 记录操作类型用于重试
      lastOperation = 'region';

    } catch (err) {
      const error = err as Error;
      console.error('Region selection failed:', error);
      recognitionStore.setError(`区域选择失败: ${error.message}`);
    }
  }

  // 从图片路径开始识别流程
  async function startRecognitionFromImagePath(imagePath: string) {
    try {

      // 设置初始状态
      recognitionStore.setResult({ id: '', latex: '', title: '', analysis: { summary: '', variables: [], terms: [], suggestions: [] }, is_favorite: false, created_at: '', confidence_score: 0, original_image: '' } as any);
      recognitionStore.start();
      resetPhaseForStart(); // 使用统一的状态初始化函数
      showPhaseStatus = true;

      // 调用后端开始识别
      await invoke('recognize_from_file', {
        filePath: imagePath
      });
      

    } catch (err) {
      const error = err as Error;
      console.error('Recognition failed:', error);
      recognitionStore.setError(`识别失败: ${error.message}`);
    }
  }
</script>

<div class="recognition-view">
  {#if $recognitionStore.errorMessage}
    <div class="error-message">
      <p>{$recognitionStore.errorMessage}</p>
      <button class="close-button" on:click={() => recognitionStore.clearError()}>&times;</button>
    </div>
  {/if}

  <div class="recognition-controls">
    <button class="btn btn-primary" on:click={recognizeFromRegion}>
      {translateNow('recognition.region_capture', $currentLang)} ({shortcutDisplay})
    </button>
    <button class="btn btn-secondary" on:click={recognizeFromFile}>
      {translateNow('recognition.import', $currentLang)}
    </button>
    <!-- 识别进行中不再显示加载提示语 -->
    <div class="phase-status" role="status" aria-live="polite" title={translateNow('recognition.progress', $currentLang)}>
      <div class="phase-item">
        <div class="phase" data-state={phase.latex}>
          <span class="dot" aria-hidden="true"></span>
          <span class="phase-text">LaTeX</span>
        </div>
        <button class="retry-icon" on:click={() => retryPhase('latex')} title={translateNow('recognition.retry_latex', $currentLang)}>
          <RotateCcw size={14} color="#000" />
        </button>
      </div>
      <div class="sep">›</div>
      <div class="phase-item">
        <div class="phase" data-state={phase.analysis}>
          <span class="dot" aria-hidden="true"></span>
          <span class="phase-text">{translateNow('recognition.analysis', $currentLang)}</span>
        </div>
        <button class="retry-icon" on:click={() => retryPhase('analysis')} title={translateNow('recognition.retry_analysis', $currentLang)}>
          <RotateCcw size={14} color="#000" />
        </button>
      </div>
      <div class="sep">›</div>
        <div class="phase-item">
          <div class="phase" data-state={phase.verify}>
            <span class="dot" aria-hidden="true"></span>
            <span class="phase-text">{translateNow('recognition.verification', $currentLang)}（beta）</span>
          </div>
          <button class="retry-icon" on:click={() => retryPhase('verify')} title={translateNow('recognition.retry_verify', $currentLang)}>
            <RotateCcw size={14} color="#000" />
          </button>
        </div>
    </div>
  </div>

  

  {#if $recognitionStore.result}
    <div class="card recognition-result">
      <!-- 统一标题栏：左侧 tabs，中间标题，右侧模型/时间，最右警告 -->
      <div class="title-bar">
        <div class="tabs-left">
          <button class="tab" class:active={activeTab==='basic'} on:click={() => activeTab='basic'}>
            {translateNow('recognition.tab.basic', $currentLang)}
          </button>
          <button class="tab" class:active={activeTab==='analysis'} on:click={() => activeTab='analysis'}>
            {translateNow('recognition.tab.analysis', $currentLang)}
          </button>
        </div>

        <div class="title-center">
          <button class="image-button" title={translateNow('recognition.original_image', $currentLang)} on:click={toggleOriginalInline}>
            <ImageIcon size={18} />
          </button>
          <button
            class="favorite-button"
            on:click={toggleFavorite}
            class:active={$recognitionStore.result.is_favorite}
            title={$recognitionStore.result.is_favorite ? translateNow('common.favorite.remove', $currentLang) : translateNow('common.favorite.add', $currentLang)}
          >
            <span class="icon">
              <StarIcon size={18} fill={$recognitionStore.result.is_favorite ? 'currentColor' : 'none'} />
            </span>
          </button>
          <h2 class="formula-title {titleSizeClass}" contenteditable
              on:input={handleTitleInput}
              on:blur={handleTitleBlur}>
            {$recognitionStore.result.title}
          </h2>
        </div>

        <div class="meta-col">
          {#if $recognitionStore.result.model_name}
            <span class="badge model-badge" title="{$recognitionStore.result.model_name}">{$recognitionStore.result.model_name}</span>
          {/if}
          {#if formatDate($recognitionStore.result.created_at) !== '—'}
            <span class="time-text">{formatDate($recognitionStore.result.created_at)}</span>
          {/if}
        </div>

        <div class="warn-right">
          {#if overallStatus.level > 0 || $recognitionStore.result.verification}
            <button
              type="button"
              class="status-icon {overallStatus.label}"
              title={overallStatus.label}
              aria-label={overallStatus.label}
              on:click={() => activeTab='analysis'}
            >
              {#if overallStatus.label === 'error'}
                <AlertCircle size={18} color="var(--status-error)" />
              {:else if overallStatus.label === 'warning'}
                <AlertTriangle size={18} color="var(--status-warning)" />
              {:else if overallStatus.label === 'info'}
                <Bell size={18} color="var(--status-info)" />
              {:else}
                ✔️
              {/if}
            </button>
          {/if}
        </div>
      </div>

      {#if activeTab === 'basic'}
        <!-- 原始图片 + 预览 + LaTeX（原图显示在公式预览上方） -->
        {#if isOriginalExpanded && originalImageSrc}
          <div class="original-inline">
            <img src={originalImageSrc} alt={translateNow('recognition.image_alt', $currentLang)} />
          </div>
        {/if}
        <div class="result-preview">
          <div class="formula-preview">
            {#key $recognitionStore.result.latex}
              <FormulaRenderer latex={$recognitionStore.result.latex} mode="preview" previewHeight={120} />
            {/key}
          </div>
        </div>

        <div class="bottom-section basic">
          <div class="latex-editor-container">
            <div class="latex-header">
              <h3 class="latex-title">
                {translateNow('recognition.latex_code', $currentLang)}
                <span class="pill-group" aria-label="current render engine and format">
                  <span class="pill engine" title="渲染引擎">{engineDisplay}</span>
                  <span class="pill sep">·</span>
                  <span class="pill format" title="LaTeX 格式">{formatDisplay}</span>
                </span>
              </h3>
              <div class="latex-actions">
                <button class="btn btn-secondary btn-compact" on:click={copyLatex} title={translateNow('recognition.copy_latex', $currentLang)}>
                  {translateNow('recognition.copy_latex', $currentLang)}
                </button>
              </div>
            </div>
            <LatexEditor latex={$recognitionStore.result.latex} on:update={updateLatex} />
          </div>

          <!-- 操作按钮行已并入 LaTeX 标题右侧，减少空间占用 -->
        </div>
      {:else}
        <!-- 分析页面：公式预览 + 可折叠的分析内容 -->
        {#if isOriginalExpanded && originalImageSrc}
          <div class="original-inline">
            <img src={originalImageSrc} alt={translateNow('recognition.image_alt', $currentLang)} />
          </div>
        {/if}

        <!-- 公式预览保持不变 -->
        <div class="result-preview">
          <div class="formula-preview">
            {#key $recognitionStore.result.latex}
              <FormulaRenderer latex={$recognitionStore.result.latex} mode="preview" previewHeight={120} />
            {/key}
          </div>
        </div>

        <!-- 分析内容区域 -->
        <div class="analysis-sections">
          <!-- 分析摘要 -->
          {#if $recognitionStore.result.analysis.summary}
            <div class="analysis-section">
              <div class="section-header-static">
                <span class="section-title">{translateNow('recognition.analysis', $currentLang)}</span>
              </div>
              <div class="section-content">
                <div class="analysis-summary">
                  {$recognitionStore.result.analysis.summary}
                </div>
              </div>
            </div>
          {/if}

          <!-- 变量部分 -->
          {#if $recognitionStore.result.analysis.variables && $recognitionStore.result.analysis.variables.length > 0}
            <div class="analysis-section">
              <button
                class="section-header"
                on:click={() => isVariablesExpanded = !isVariablesExpanded}
                aria-expanded={isVariablesExpanded}
              >
                <span class="section-title">{translateNow('main.result.variables', $currentLang)} ({$recognitionStore.result.analysis.variables.length})</span>
                <span class="expand-icon">{isVariablesExpanded ? '▼' : '▶'}</span>
              </button>
              {#if isVariablesExpanded}
                <div class="section-content">
                  <div class="variables-grid">
                    {#each $recognitionStore.result.analysis.variables as variable}
                      <div class="variable-item">
                        <!-- 第一行：变量描述 -->
                        <div class="variable-description">{variable.description}</div>

                        <!-- 第二行：符号、单位标签、单位值 -->
                        <div class="variable-details">
                          <div class="variable-symbol">
                            <FormulaRenderer latex={variable.symbol} mode="preview" previewHeight={80} />
                          </div>
                          {#if variable.unit}
                            <div class="unit-label">{translateNow('main.result.unit', $currentLang)}</div>
                            <div class="variable-unit">
                              <FormulaRenderer latex={variable.unit} mode="preview" previewHeight={72} />
                            </div>
                          {/if}
                        </div>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          {/if}

          <!-- 术语部分 -->
          {#if $recognitionStore.result.analysis.terms && $recognitionStore.result.analysis.terms.length > 0}
            <div class="analysis-section">
              <button
                class="section-header"
                on:click={() => isTermsExpanded = !isTermsExpanded}
                aria-expanded={isTermsExpanded}
              >
                <span class="section-title">{translateNow('main.result.terms', $currentLang)} ({$recognitionStore.result.analysis.terms.length})</span>
                <span class="expand-icon">{isTermsExpanded ? '▼' : '▶'}</span>
              </button>
              {#if isTermsExpanded}
                <div class="section-content">
                  <div class="terms-list">
                    {#each $recognitionStore.result.analysis.terms as term}
                      <div class="term-item">
                        <div class="term-name">
                          <FormulaRenderer latex={term.name} mode="preview" previewHeight={72} />
                        </div>
                        <div class="term-description">{term.description}</div>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          {/if}

          <!-- 建议部分 -->
          {#if $recognitionStore.result.analysis.suggestions && $recognitionStore.result.analysis.suggestions.length > 0}
            <div class="analysis-section">
              <button
                class="section-header"
                on:click={() => isSuggestionsExpanded = !isSuggestionsExpanded}
                aria-expanded={isSuggestionsExpanded}
              >
                <span class="section-title">{translateNow('main.result.suggestions', $currentLang)} ({$recognitionStore.result.analysis.suggestions.length})</span>
                <span class="expand-icon">{isSuggestionsExpanded ? '▼' : '▶'}</span>
              </button>
              {#if isSuggestionsExpanded}
                <div class="section-content">
                  <div class="suggestions-list">
                    {#each $recognitionStore.result.analysis.suggestions as suggestion}
                      <div class="suggestion-item {suggestion.type}">
                        <div class="suggestion-type">{suggestion.type}</div>
                        <div class="suggestion-message">{suggestion.message}</div>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          {/if}

          <!-- 验证报告部分 -->
          {#if $recognitionStore.result.verification || $recognitionStore.result.verification_report}
            <div class="analysis-section">
              <button
                class="section-header"
                on:click={() => isVerificationExpanded = !isVerificationExpanded}
                aria-expanded={isVerificationExpanded}
              >
                <span class="section-title">{translateNow('main.result.verification_report', $currentLang)}</span>
                <span class="expand-icon">{isVerificationExpanded ? '▼' : '▶'}</span>
              </button>
              {#if isVerificationExpanded}
                <div class="section-content">
                  {#if $recognitionStore.result.verification}
                    <VerificationReportRenderer verification={$recognitionStore.result.verification} />
                  {:else}
                    <div class="verification-content">
                      <div class="verification-status warning">状态: 未提供结构化明细</div>
                      <div class="verification-issues">
                        <h5>报告:</h5>
                        <ul>
                          <li class="issue">
                            <div class="issue-content">{$recognitionStore.result.verification_report}</div>
                          </li>
                        </ul>
                      </div>
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {:else if $recognitionStore.isLoading}
    <div class="loading-indicator card">
      <p>{translateNow('recognition.loading', $currentLang)}</p>
    </div>
  {:else}
    <div class="empty-state card">
      <p>{translateNow('recognition.start_hint', $currentLang)}</p>
    </div>
  {/if}
</div>

<!-- 弹窗预览已移除，改为行内展示 originalImage -->

<style>
  .recognition-view {
    width: 100%;
    padding: 0; /* 页面的左右留白由外层 .main-page 控制 */
    display: flex;
    flex-direction: column;
    gap: var(--spacing-lg);
  }

  .error-message {
    background-color: var(--status-error-bg);
    color: var(--status-error);
    padding: var(--spacing-base);
    border-radius: var(--border-radius-btn);
    margin-bottom: var(--spacing-base);
    display: flex;
    justify-content: space-between;
    align-items: center;
    border: 1px solid var(--status-error);
  }

  .close-button {
    background: none;
    border: none;
    color: var(--status-error);
    cursor: pointer;
    font-size: var(--font-size-lg);
    line-height: 1;
  }

  .recognition-controls {
    display: flex;
    gap: var(--spacing-base);
    align-items: center;
  }

  /* 按钮样式 */
  .btn {
    border-radius: var(--border-radius-btn);
    padding: var(--spacing-sm) var(--spacing-base);
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  /* 主按钮 */
  .btn:not(.btn-secondary) {
    background-color: var(--primary);
    color: white;
    border: none;
  }

  .btn:not(.btn-secondary):hover {
    background-color: var(--primary-hover);
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }

  /* 主要按钮（强调） */
  .btn.btn-primary {
    background: linear-gradient(135deg, var(--primary), var(--primary-hover));
    color: white;
    border: none;
    box-shadow: 0 2px 8px rgba(0, 122, 204, 0.3);
  }

  .btn.btn-primary:hover {
    background: linear-gradient(135deg, var(--primary-hover), var(--primary));
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 122, 204, 0.4);
  }

  /* 次要按钮 */
  .btn.btn-secondary {
    background-color: var(--bg-secondary);
    color: var(--text-default);
    border: 2px solid var(--border-primary);
  }

  .btn-compact {
    padding: 4px 10px;
    font-size: 13px;
    line-height: 1.3;
    border-width: 1px;
  }

  .btn.btn-secondary:hover {
    background-color: var(--bg-hover);
    border-color: var(--border-hover);
  }

  /* 行内状态提示，跟随导入按钮右侧，尺寸随内容自适应 */
  .inline-status {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border: 1px solid var(--border-primary);
    border-radius: var(--border-radius-btn);
    background: var(--bg-secondary);
    color: var(--text-muted);
    white-space: nowrap;
  }
  /* 三阶段状态条 */
  .phase-status { display:inline-flex; align-items:center; gap:10px; }
  .phase-status .sep { color: var(--text-muted); font-size: var(--font-size-base); }
  .phase-item { display:flex; align-items:center; gap:6px; }
  .phase { display:flex; align-items:center; gap:6px; color: var(--text-muted); }
  .phase .dot { width:10px; height:10px; border-radius:50%; display:inline-block; }
  .phase[data-state="idle"] .dot { background:#cbd5e1; }
  .phase[data-state="pending"] { color: var(--text-default); }
  .phase[data-state="pending"] .dot { background:#fbbf24; animation: pulse 1s ease-in-out infinite alternate; }
  .phase[data-state="done"] { color: var(--text-default); font-weight: var(--font-weight-semibold); }
  .phase[data-state="done"] .dot { background:#16a34a; }
  .phase[data-state="error"] { color: var(--status-error); font-weight: var(--font-weight-semibold); }
  .phase[data-state="error"] .dot { background: var(--status-error); }
  .phase-text { font-size: var(--font-size-base); white-space: nowrap; }
  @keyframes pulse { from { opacity:.6 } to { opacity:1 } }

  .retry-icon {
    background:none;
    border:none;
    padding:0;
    margin:0 0 0 4px;
    cursor:pointer;
    opacity:.7;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 26px;
    width: 26px;
  }
  .retry-icon:hover { opacity:1; }
  .inline-spinner {
    width: 14px; height: 14px; border-radius: 50%;
    border: 2px solid var(--border-secondary, #e5e7eb);
    border-top-color: var(--primary);
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none;
  }

  .recognition-result {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-base);
    border: 1px solid var(--border-primary);
    border-radius: var(--border-radius-card);
    box-shadow: var(--card-shadow);
    overflow: hidden;
  }

  /* 已移除的原图折叠样式（.original-*, .section-toggle, .original-image）清理 */

  .result-preview { overflow: visible; margin: var(--spacing-sm) 0 var(--spacing-md) 0; background-color: transparent; }
  .formula-preview { padding: 0; border: none; border-radius: 0; background: transparent; }

  .title-bar {
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    align-items: center;
    gap: var(--spacing-base);
    padding: var(--spacing-md) var(--spacing-base);
    background-color: var(--bg-secondary);
    border-bottom: 1px solid var(--border-primary);
  }
  .tabs-left { display: inline-flex; gap: 8px; }
  .title-center { display:flex; align-items:center; justify-content:center; gap: var(--spacing-sm); text-align:center; }
  .meta-col { display:flex; flex-direction:column; align-items:flex-end; justify-content:center; gap: 4px; }
  .warn-right { display:flex; align-items:center; justify-content:flex-end; }

  .favorite-button {
    background: none;
    border: none;
    font-size: var(--font-size-h3);
    cursor: pointer;
    color: var(--text-muted);
    margin-right: var(--spacing-sm);
    padding: var(--spacing-xs);
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    transition: background-color 0.2s ease;
  }

  .image-button {
    background: none;
    border: none;
    border-radius: 0;
    padding: 0;
    margin-right: var(--spacing-xs);
    cursor: default;
    color: var(--text-default);
    background-color: transparent;
  }
  .image-button:hover { background-color: transparent; }

  .favorite-button:hover {
    color: var(--status-warning);
    background-color: var(--bg-hover);
  }

  .favorite-button.active .icon {
    color: var(--status-warning);
  }

  .formula-title {
    margin: 0;
    font-weight: var(--font-weight-semibold);
    line-height: 1.25;
    flex-grow: 1;
    color: var(--text-header);
    max-height: calc(2 * 1.25em); /* 限制两行 */
    overflow: hidden;
    display: -webkit-box;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    word-break: break-word;
  }
  .formula-title.title-xl { font-size: var(--font-size-h2); }
  .formula-title.title-lg { font-size: calc(var(--font-size-h2) - 2px); }
  .formula-title.title-md { font-size: var(--font-size-card-title); }
  .formula-title.title-sm { font-size: var(--font-size-base); }

  .status-icon { font-size: var(--font-size-card-title); margin-left: var(--spacing-sm); background: transparent; border: none; padding: 0; }
  .status-icon.warning { color: var(--status-warning); }
  .status-icon.error { color: var(--status-error); }
  .status-icon.ok { color: var(--status-success, #16a34a); }

  .meta {
    margin-left: auto;
    color: var(--text-muted);
    font-size: var(--font-size-small);
    white-space: nowrap;
  }
  .badge { display:inline-flex; align-items:center; padding:2px 8px; border-radius:999px; font-size:12px; line-height:1; white-space:nowrap; }
  .model-badge { background: rgba(99,102,241,0.1); color: var(--primary); border:1px solid var(--primary); }
  .meta-sep { margin: 0 6px; color: var(--border-secondary, #e5e7eb); }
  .time-text { color: var(--text-muted); }

  .formula-preview { padding: 0; background-color: transparent; }

  .bottom-section { display: grid; grid-template-columns: 1fr; gap: var(--spacing-md); padding: 0 var(--spacing-base) var(--spacing-sm); align-items: start; }

  @media (min-width: 1024px) {
    .bottom-section { grid-template-columns: 1.35fr 0.65fr; }
    .latex-editor-container { grid-column: 1; }
    .analysis-container, .confidence-score { grid-column: 2; }
    .action-buttons { grid-column: 1 / -1; }
  }

  .latex-editor-container {
    padding: var(--spacing-sm) calc(var(--spacing-base) + 4px);
    border-radius: var(--border-radius-card);
    border: 1px solid var(--border-secondary, #e5e7eb);
    background-color: var(--bg-main);
    width: 100%;
  }
  .latex-header { display: flex; align-items: center; justify-content: space-between; gap: var(--spacing-base); margin-bottom: var(--spacing-sm); padding-right: 2px; }
  .latex-title { margin: 0; font-size: var(--font-size-h3); font-weight: var(--font-weight-semibold); color: var(--text-header); position: relative; top: 2px; }
  .pill-group { margin-left: 8px; display: inline-flex; gap: 6px; align-items: center; vertical-align: middle; }
  .pill { display: inline-flex; align-items: center; height: 22px; padding: 0 8px; border-radius: 999px; font-size: 12px; line-height: 1; border: 1px solid var(--border-secondary, #e5e7eb); background: var(--bg-secondary); color: var(--text-muted); }
  .pill.engine { text-transform: none; }
  /* format pill uses same base style */
  .pill.sep { padding: 0; width: auto; border: none; background: transparent; color: var(--border-secondary, #e5e7eb); }
  .latex-actions { display: inline-flex; gap: var(--spacing-sm); }
  .bottom-section.basic { grid-template-columns: 1fr !important; }
  .bottom-section.basic .latex-editor-container { grid-column: 1 / -1 !important; width: 100%; }
  /* 统一原始图片卡片与下方 LaTeX 区域视觉宽度：两者都处在 recognition-result 内部并使用相同的左右 padding（var(--spacing-base)） */

  .latex-editor-container h3 {
    margin-top: 0;
    margin-bottom: var(--spacing-base);
    font-size: var(--font-size-h3);
    font-weight: var(--font-weight-semibold);
    color: var(--text-header);
    line-height: var(--line-height-normal);
  }

  .analysis-container {
    width: 100%;
  }

  .analysis-toggle {
    width: 100%;
    text-align: left;
    padding: var(--spacing-md) var(--spacing-base);
    background-color: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: var(--border-radius-btn);
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-weight: var(--font-weight-medium);
    color: var(--text-default);
  }

  .analysis-toggle:hover {
    background-color: var(--bg-hover);
  }

  .analysis-content {
    padding: var(--spacing-base);
    background-color: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-top: none;
    border-radius: 0 0 var(--border-radius-btn) var(--border-radius-btn);
    margin-top: calc(-1 * var(--spacing-xs));
  }

  .analysis-summary {
    margin-top: 0;
    color: var(--text-muted);
    font-size: var(--font-size-base);
    line-height: var(--line-height-relaxed);
  }

  .tabs { display: inline-flex; gap: 8px; padding: 0 var(--spacing-base); margin-top: var(--spacing-sm); }
  .tab { border: 1px solid var(--border-secondary, #e5e7eb); background: var(--bg-secondary); color: var(--text-default); padding: 6px 12px; border-radius: 999px; cursor: pointer; }
  .tab.active { background: var(--primary); color: #fff; border-color: var(--primary); }

  /* 分析页面样式 */
  .analysis-sections {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    margin-top: var(--spacing-md);
    padding: 0 var(--spacing-base);
  }

  .analysis-section {
    border: 1px solid var(--border-primary);
    border-radius: var(--border-radius-card);
    overflow: hidden;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .section-header {
    width: 100%;
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--bg-secondary);
    border: none;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-semibold);
    color: var(--text-header);
    transition: background-color 0.2s ease;
  }

  .section-header-static {
    width: 100%;
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--bg-secondary);
    border: none;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: var(--font-size-body);
    color: var(--text-default);
  }

  .section-header:hover {
    background: var(--bg-hover);
  }

  .section-title {
    font-weight: var(--font-weight-semibold);
  }

  .expand-icon {
    font-size: var(--font-size-sm);
    transition: transform 0.2s ease;
  }

  .section-content {
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--bg-main);
    border-top: 1px solid var(--border-secondary);
  }

  .analysis-summary {
    color: var(--text-default);
    line-height: var(--line-height-normal);
    font-size: var(--font-size-body);
    word-wrap: break-word;
    overflow-wrap: break-word;
  }

  /* 变量样式 */
  .variables-grid {
    display: grid;
    gap: var(--spacing-sm);
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  }

  .variable-item {
    padding: var(--spacing-sm);
    border: 1px solid var(--border-secondary);
    border-radius: var(--border-radius-btn);
    background: var(--bg-secondary);
    transition: all 0.2s ease;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }

  .variable-item:hover {
    border-color: var(--primary);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }

  /* 第一行：变量描述 */
  .variable-description {
    color: var(--text-default);
    font-weight: var(--font-weight-medium);
    line-height: 1.2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: clamp(0.75rem, 2vw, 0.9rem);
  }

  /* 第二行：符号、单位标签、单位值 */
  .variable-details {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    min-height: 44px;
  }

  .variable-symbol {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    max-height: 44px;
    overflow: hidden;
  }

  .variable-symbol :global(.formula-renderer) {
    max-height: 44px;
    min-height: 44px;
    padding: 2px 4px;
    border: none;
    background: transparent;
    box-shadow: none;
  }

  .unit-label {
    color: var(--text-muted);
    font-size: clamp(0.7rem, 1.5vw, 0.8rem);
    flex-shrink: 0;
    margin-left: auto;
  }

  .variable-unit {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    max-height: 40px;
    overflow: hidden;
  }

  .variable-unit :global(.formula-renderer) {
    max-height: 40px;
    min-height: 40px;
    padding: 2px 4px;
    border: none;
    background: transparent;
    box-shadow: none;
  }

  /* 术语样式 */
  .terms-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .term-item {
    padding: var(--spacing-sm);
    border-left: 4px solid var(--primary);
    background: var(--bg-secondary);
    border-radius: 0 var(--border-radius-btn) var(--border-radius-btn) 0;
    border: 1px solid var(--border-secondary);
    border-left: 4px solid var(--primary);
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    gap: var(--spacing-base);
    min-height: 60px;
  }

  .term-item:hover {
    border-color: var(--primary);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    transform: translateX(2px);
  }

  .term-name {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    min-width: 80px;
    max-width: 140px;
    padding: 2px 4px;
    background: var(--bg-main);
    border-radius: var(--border-radius-btn);
    border: 1px solid var(--border-light);
    overflow: hidden;
  }

  .term-name :global(.formula-renderer) {
    border: none;
    background: transparent;
    box-shadow: none;
    padding: 1px;
    min-height: 36px;
    max-height: 36px;
  }

  .term-description {
    color: var(--text-default);
    line-height: var(--line-height-normal);
    font-size: clamp(0.8rem, 2vw, 0.95rem);
    flex: 1;
    word-wrap: break-word;
    overflow-wrap: break-word;
  }

  /* LaTeX 数学公式样式优化 */
  :global(.katex) {
    font-size: 1.2em !important;
    line-height: 1.4 !important;
  }

  :global(.katex-display) {
    margin: var(--spacing-sm) 0 !important;
    font-size: 1.3em !important;
  }

  :global(.katex .base) {
    display: inline-block;
    vertical-align: middle;
  }

  /* 变量和术语中的数学公式 */
  .variable-symbol :global(.katex),
  .variable-unit :global(.katex),
  .term-name :global(.katex) {
    font-size: clamp(0.9rem, 2vw, 1.1rem) !important;
    color: var(--text-default) !important;
  }

  /* 确保数学公式在容器中居中对齐 */
  .variable-symbol,
  .variable-unit,
  .term-name {
    justify-content: flex-start;
    align-items: center;
  }

  /* 数学公式容器的最小高度和自适应 */
  .variable-symbol :global(.katex),
  .variable-unit :global(.katex),
  .term-name :global(.katex) {
    min-height: 32px;
    display: inline-flex;
    align-items: center;
    max-width: 100%;
  }

  /* 变量详情行中的公式自适应 */
  .variable-details .variable-symbol :global(.katex) {
    font-size: clamp(3.2rem, 8vw, 4rem) !important;
  }

  .variable-details .variable-unit :global(.katex) {
    font-size: clamp(2.8rem, 7vw, 3.6rem) !important;
  }

  /* 建议样式 */
  .suggestions-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .suggestion-item {
    padding: var(--spacing-sm);
    border-radius: var(--border-radius-btn);
    border-left: 3px solid;
    display: flex;
    gap: var(--spacing-sm);
    align-items: flex-start;
  }

  .suggestion-item.error {
    background: var(--status-error-bg);
    border-left-color: var(--status-error);
  }

  .suggestion-item.warning {
    background: var(--status-warning-bg);
    border-left-color: var(--status-warning);
  }

  .suggestion-item.info {
    background: var(--status-info-bg);
    border-left-color: var(--status-info);
  }

  .suggestion-type {
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    padding: 2px 6px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.8);
    flex-shrink: 0;
  }

  .suggestion-message {
    flex: 1;
    color: var(--text-default);
    line-height: var(--line-height-relaxed);
  }

  /* 验证报告样式 */
  .verification-content {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }

  .verification-status {
    padding: var(--spacing-sm);
    border-radius: var(--border-radius-btn);
    font-weight: var(--font-weight-semibold);
  }

  .verification-status.error {
    background: var(--status-error-bg);
    color: var(--status-error);
  }

  .verification-status.warning {
    background: var(--status-warning-bg);
    color: var(--status-warning);
  }

  .verification-status.ok {
    background: var(--status-success-bg);
    color: var(--status-success);
  }

  .verification-issues h5,
  /* .verification-coverage h5 { moved to component-specific CSS; keep here for compatibility if coverage is shown inline } */

  .verification-issues ul {
    margin: 0;
    padding-left: var(--spacing-base);
  }

  .verification-issues .issue {
    margin-bottom: var(--spacing-xs);
    color: var(--text-default);
  }

  .coverage-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: var(--spacing-sm);
  }

  .coverage-stats > div {
    padding: var(--spacing-sm);
    background: var(--bg-secondary);
    border-radius: var(--border-radius-btn);
    font-size: var(--font-size-sm);
  }

  .suggestion-item {
    margin-bottom: var(--spacing-sm);
    font-size: var(--font-size-base);
    line-height: var(--line-height-relaxed);
  }

  .suggestion-item.warning {
    color: var(--status-warning);
  }

  .suggestion-item.info {
    color: var(--status-info);
  }

  .suggestion-item.error {
    color: var(--status-error);
  }

  .confidence-score {
    padding: var(--spacing-sm) var(--spacing-base);
    background-color: var(--bg-hover);
    border-radius: var(--border-radius-btn);
    text-align: center;
    font-weight: var(--font-weight-medium);
    color: var(--text-default);
    border: 1px solid var(--border-primary);
  }

  .action-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-sm);
    margin-top: var(--spacing-lg);
    justify-content: center;
  }

  .loading-indicator, .empty-state {
    text-align: center;
    padding: var(--spacing-xl);
    color: var(--text-muted);
    border-radius: var(--border-radius-card);
    border: 1px solid var(--border-primary);
    box-shadow: var(--card-shadow);
    background-color: var(--bg-main);
  }
  .preview-skeleton { height: 120px; border-radius: var(--border-radius-card); background: linear-gradient(90deg, var(--bg-secondary) 25%, var(--bg-hover) 37%, var(--bg-secondary) 63%); background-size: 400% 100%; animation: shimmer 1.4s ease infinite; }
  @keyframes shimmer { 0% { background-position: 100% 0 } 100% { background-position: -100% 0 } }

  /* 原图预览弹层（复用详情遮罩样式） */
  /* 行内原图区域，宽度与下方 LaTeX/预览区域一致 */
  .original-inline { padding: var(--spacing-sm) var(--spacing-base) 0; }
  .original-inline img { display:block; max-width:100%; max-height:260px; object-fit:contain; margin: 0 auto; border:1px solid var(--border-primary); border-radius: var(--border-radius-card); background: var(--bg-secondary); padding: var(--spacing-xs); }
</style>