<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { writable, get } from 'svelte/store';
  import { invoke } from '@tauri-apps/api/tauri';
  import type { Config } from '$lib/types';
  import { currentLang, translateNow, setLanguage, type Lang } from '$lib/i18n';
  import { showToast } from '$lib/toast';

  type UIConfig = Partial<Config> & {
    __lastUsedLatexPrompt?: string;
    __lastUsedAnalysisPrompt?: string;
    __lastUsedVerificationPrompt?: string;
    screenshotShortcut?: string;
  };

  const configStore = writable<UIConfig>({
    provider: 'gemini',
    defaultEngine: 'gemini-2.5-pro',
    // 仅用于显示：上次实际使用的提示词（从 localStorage 注入）
    __lastUsedLatexPrompt: '',
    __lastUsedAnalysisPrompt: '',
    __lastUsedVerificationPrompt: ''
  });
  let debounceTimer: number;
  // 文本域引用（用于记忆高度）
  let latexPromptTextarea: HTMLTextAreaElement;
  let analysisPromptTextarea: HTMLTextAreaElement;
  let confidencePromptTextarea: HTMLTextAreaElement;
  let previewUrl = '';
  
  async function getDefaultPrompts() {
    // 前端仅显示基础提示词
    const res = await invoke('get_default_prompts');
    const { latex_prompt, analysis_prompt, verification_prompt, confidence_prompt } = res as any;
    const defaultCustom = '';
    const ver = verification_prompt ?? confidence_prompt;
    return { defaultCustom, defaultConfidence: ver, defaultLatex: latex_prompt, defaultAnalysis: analysis_prompt };
  }
  
  async function resetPromptsToDefault() {
    const { defaultCustom, defaultConfidence, defaultLatex, defaultAnalysis } = await getDefaultPrompts();
    // 使用 store.update 触发响应式与持久化
    configStore.update((cfg) => ({
      ...(cfg || {}),
      // 兼容旧字段
      customPrompt: defaultCustom,
      // 新三段提示词
      verificationPrompt: defaultConfidence,
      latexPrompt: defaultLatex,
      analysisPrompt: defaultAnalysis,
    }));
    try {
      const cfgNow = get(configStore) as any;
      await invoke('save_config', { config: cfgNow });
      showToast(translateNow('settings.prompt.reset_ok', $currentLang), 'success');
      await tick();
      if (confidencePromptTextarea) {
        confidencePromptTextarea.style.height = 'auto';
        confidencePromptTextarea.style.height = `${confidencePromptTextarea.scrollHeight}px`;
      }
      if (latexPromptTextarea) {
        latexPromptTextarea.style.height = 'auto';
        latexPromptTextarea.style.height = `${latexPromptTextarea.scrollHeight}px`;
      }
      if (analysisPromptTextarea) {
        analysisPromptTextarea.style.height = 'auto';
        analysisPromptTextarea.style.height = `${analysisPromptTextarea.scrollHeight}px`;
      }
    } catch (e) {
      showToast(translateNow('settings.alert.save_failed', $currentLang), 'error');
    }
  }

  function canonicalModelsBase(baseUrl: string): string {
    if (!baseUrl) return '';
    const b = baseUrl.replace(/\/+$/, '');
    if (b.includes('/models')) return b;
    if (b.includes('/v1beta') || b.includes('/v1')) return `${b}/models`;
    return `${b}/v1beta/models`;
  }

  function buildFinalUrl(baseUrl: string, model: string, apiKey?: string): string {
    const base = canonicalModelsBase(baseUrl || '');
    if (!base || !model) return '';
    const keyPart = apiKey ? '?key=***' : '';
    return `${base}/${model}:generateContent${keyPart}`;
  }

  onMount(async () => {
    await loadConfig();
    // 恢复提示词文本框高度
    try {
      const hLatex = localStorage.getItem('promptHeight.latex');
      const hAnalysis = localStorage.getItem('promptHeight.analysis');
      const hConfidence = localStorage.getItem('promptHeight.confidence');
      if (hLatex && latexPromptTextarea) latexPromptTextarea.style.height = hLatex;
      if (hAnalysis && analysisPromptTextarea) analysisPromptTextarea.style.height = hAnalysis;
      if (hConfidence && confidencePromptTextarea) confidencePromptTextarea.style.height = hConfidence;
    } catch {}
  });

  // 从 localStorage 读取上次实际调用的提示词，供右侧展示
  $: (function syncLastUsedPrompts(){
    try {
      const raw = localStorage.getItem('__lastUsedPrompts');
      if (!raw) return;
      const parsed = JSON.parse(raw);
      configStore.update((cfg)=> ({
        ...(cfg||{}),
        __lastUsedLatexPrompt: parsed?.latex_prompt || parsed?.latexPrompt || '',
        __lastUsedAnalysisPrompt: parsed?.analysis_prompt || parsed?.analysisPrompt || '',
        __lastUsedVerificationPrompt: parsed?.verification_prompt || parsed?.verificationPrompt || ''
      } as any));
    } catch {}
  })();

  let hasLoadedConfig = false;
  async function loadConfig() {
    if (hasLoadedConfig) return;
    try {
      const config: Config = await invoke('get_config');
      configStore.set(config);
      hasLoadedConfig = true;
      await tick(); // Wait for DOM to update
      if (confidencePromptTextarea) {
        confidencePromptTextarea.style.height = 'auto';
        confidencePromptTextarea.style.height = `${confidencePromptTextarea.scrollHeight}px`;
      }
      if (latexPromptTextarea) {
        latexPromptTextarea.style.height = 'auto';
        latexPromptTextarea.style.height = `${latexPromptTextarea.scrollHeight}px`;
      }
      if (analysisPromptTextarea) {
        analysisPromptTextarea.style.height = 'auto';
        analysisPromptTextarea.style.height = `${analysisPromptTextarea.scrollHeight}px`;
      }
    } catch (error) {
      console.error('Failed to load config:', error);
    }
  }

  // Ensure config is loaded promptly (also helps in test env)
  loadConfig();

  // Reactive preview of final URL
  $: previewUrl = buildFinalUrl($configStore.apiBaseUrl as string, $configStore.defaultEngine as string, $configStore.apiKey as string);

  const saveConfig = (config: Partial<Config>) => {
    clearTimeout(debounceTimer);
    debounceTimer = window.setTimeout(async () => {
      try {
        await invoke('save_config', { config });
      } catch (error) {
        console.error('Failed to save config:', error);
      }
    }, 500);
  };

  // 手动保存配置
  const handleSaveConfig = async () => {
    try {
      await invoke('save_config', { config: $configStore });
      showToast(translateNow('settings.alert.save_success', $currentLang), 'success');
    } catch (error) {
      console.error('Failed to save config:', error);
      showToast(translateNow('settings.alert.save_failed', $currentLang), 'error');
    }
  };

  // 保存快捷键设置
  const handleSaveShortcut = async () => {
    try {
      await invoke('save_config', { config: $configStore });

      // 重新注册全局快捷键
      if ($configStore.screenshotShortcut) {
        try {
          await invoke('register_global_shortcut', { shortcut: $configStore.screenshotShortcut });
          showToast(translateNow('settings.alert.save_success', $currentLang), 'success');
        } catch (shortcutError) {
          console.error('Failed to register global shortcut:', shortcutError);
          showToast(translateNow('settings.alert.shortcut_failed', $currentLang) || 'Failed to register shortcut', 'warning');
        }
      } else {
        showToast(translateNow('settings.alert.save_success', $currentLang), 'success');
      }
    } catch (error) {
      console.error('Failed to save config:', error);
      showToast(translateNow('settings.alert.save_failed', $currentLang), 'error');
    }
  };

  // 应用窗口大小到当前窗口
  const applyWindowToCurrent = async () => {
    const cfg: Config = await invoke('get_config');
    const width = Number(($configStore.windowWidth ?? cfg.windowWidth) || 1280);
    const height = Number(($configStore.windowHeight ?? cfg.windowHeight) || 800);
    let resized = false;
    // 尝试调整窗口大小（不影响后续保存配置）
    try {
      const { appWindow, PhysicalSize, LogicalSize } = await import('@tauri-apps/api/window');
      // 如果窗口是最大化状态，需先取消最大化才能设置尺寸
      try { await appWindow.unmaximize(); } catch {}
      if (PhysicalSize) {
        await appWindow.setSize(new PhysicalSize(width, height));
        resized = true;
      } else if (LogicalSize) {
        await appWindow.setSize(new LogicalSize(width, height));
        resized = true;
      }
    } catch (err) {
      console.error('setSize failed:', err);
      // 不中断保存流程
    }
    // 保存到配置
    try {
      await invoke('save_config', { config: { ...cfg, windowWidth: width, windowHeight: height } });
    } catch (err) {
      console.error('save_config failed:', err);
      showToast(translateNow('settings.alert.save_failed', $currentLang), 'error');
      return;
    }
    // 根据 setSize 结果分别提示
    if (resized) {
      showToast($currentLang === 'en' ? 'Window size updated successfully!' : '窗口大小修改成功！', 'success');
    } else {
      showToast($currentLang === 'en' ? 'Config saved. But window resize is not allowed by Tauri allowlist. Please restart or check settings.' : '配置已保存，但窗口大小修改未生效（Tauri 权限未开启或环境限制）。请重启或检查设置。', 'warning');
    }
  };

  // 测试连接功能
  let testing = false;

  // 快捷键录制相关
  let isRecording = false;
  let recordedKeys: string[] = [];

  // 格式化快捷键显示
  function formatShortcutDisplay(shortcut: string): string {
    return shortcut
      .replace('CommandOrControl', 'Ctrl')
      .replace('Command', 'Cmd')
      .replace('Control', 'Ctrl')
      .split('+')
      .map(key => key.trim())
      .join(' + ');
  }

  // 开始录制快捷键
  function startRecording() {
    isRecording = true;
    recordedKeys = [];
    // 延迟一点时间让DOM更新，然后聚焦到捕获元素
    setTimeout(() => {
      const overlay = document.querySelector('.key-capture-overlay') as HTMLElement;
      if (overlay) {
        overlay.focus();
      }
    }, 50);
  }

  // 取消录制
  function cancelRecording() {
    isRecording = false;
    recordedKeys = [];
  }

  // 处理按键事件
  function handleKeyDown(event: KeyboardEvent) {
    if (!isRecording) return;

    event.preventDefault();
    event.stopPropagation();

    const keys: string[] = [];
    let modifierCount = 0;
    let hasMainKey = false;

    // 检查修饰键（按固定顺序）
    if (event.ctrlKey || event.metaKey) {
      keys.push('CommandOrControl');
      modifierCount++;
    }
    if (event.altKey) {
      keys.push('Alt');
      modifierCount++;
    }
    if (event.shiftKey) {
      keys.push('Shift');
      modifierCount++;
    }

    // 添加主键（如果不是修饰键）
    const mainKey = event.key;
    if (!['Control', 'Shift', 'Alt', 'Meta', 'Command', 'OS'].includes(mainKey)) {
      // 处理特殊键名
      let keyName = mainKey;
      if (mainKey === ' ') {
        keyName = 'Space';
      } else if (mainKey.length === 1) {
        keyName = mainKey.toUpperCase();
      } else {
        // 保持原始键名（如 F1, Enter, Tab 等）
        keyName = mainKey;
      }
      keys.push(keyName);
      hasMainKey = true;
    }

    // 必须满足以下条件才能完成录制：
    // 1. 至少有一个修饰键
    // 2. 必须有一个主键（非修饰键）
    // 3. 总键数在2-3个之间
    // 支持的组合：
    // - 1个修饰键 + 1个主键 = 2个键 (如 Ctrl+A)
    // - 2个修饰键 + 1个主键 = 3个键 (如 Ctrl+Alt+A)
    if (modifierCount >= 1 && hasMainKey && keys.length >= 2 && keys.length <= 3) {
      const shortcut = keys.join('+');
      $configStore.screenshotShortcut = shortcut;
      isRecording = false;
      recordedKeys = [];
    }
  }
  const testConnection = async () => {
    if (!$configStore.apiKey) {
      showToast(translateNow('settings.alert.key_required', $currentLang), 'warning');
      return;
    }

    try {
      testing = true;
      await invoke('test_connection');
      showToast(translateNow('settings.alert.test_success', $currentLang), 'success');
    } catch (error) {
      console.error('Connection test failed:', error);
      const msg = String(error);
      const m1 = msg.match(/status (\d{3})/i);
      const m2 = msg.match(/code[:=]?(\s*)(\d{3})/i);
      const code = m1 ? m1[1] : (m2 ? m2[2] : undefined);
      if (code) {
        showToast(translateNow('settings.alert.test_failed_code', $currentLang).replace('{code}', code), 'error');
      } else {
        showToast(translateNow('settings.alert.test_failed', $currentLang), 'error');
      }
    } finally {
      testing = false;
    }
  };

  // 自适应文本框高度
  function autosize(event: Event) {
    const el = event.target as HTMLTextAreaElement;
    el.style.height = 'auto';
    el.style.height = `${el.scrollHeight}px`;
  }
  // 记忆用户手动调整的高度
  function rememberHeight(kind: 'latex' | 'analysis' | 'confidence') {
    try {
      let h = '';
      if (kind === 'latex' && latexPromptTextarea) h = getComputedStyle(latexPromptTextarea).height;
      else if (kind === 'analysis' && analysisPromptTextarea) h = getComputedStyle(analysisPromptTextarea).height;
      else if (kind === 'confidence' && confidencePromptTextarea) h = getComputedStyle(confidencePromptTextarea).height;
      if (h) localStorage.setItem(`promptHeight.${kind}`, h);
    } catch {}
  }

  configStore.subscribe(currentConfig => {
    if (currentConfig && Object.keys(currentConfig).length > 0) {
      saveConfig(currentConfig);
    }
  });

  const handleLanguageChange = (e: Event) => {
    const newLang = (e.target as HTMLSelectElement).value as Lang;
    setLanguage(newLang);
    saveConfig({ ...$configStore, language: newLang });
  };

</script>

<div class="settings-view">
  <div class="settings-group">
  <header class="page-header">
    <h1 class="page-title">{translateNow('settings.title', $currentLang)}</h1>
  </header>

  <!-- Language Configuration Top-Level Card -->
  <div class="top-level-card">
    <h2>{translateNow('settings.general.title', $currentLang)}</h2>
    <p class="subtitle">{translateNow('settings.language.desc', $currentLang)}</p>

    <hr class="card-divider" />

    <div class="sub-card">
      <div class="form-grid-ai">
        <div class="form-item" style="display: block; width: 100%;">
          <label for="language">{translateNow('settings.language', $currentLang)}</label>
          <div class="input-with-button">
            <select id="language" bind:value={$configStore.language} on:change={handleLanguageChange} style="flex:1">
              <option value="zh-CN">{translateNow('settings.language.zh', $currentLang)}</option>
              <option value="en">{translateNow('settings.language.en', $currentLang)}</option>
            </select>
            <button class="btn btn-test" on:click={() => invoke('open_config_dir')}>
              {translateNow('settings.actions.open_config_dir', $currentLang)}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Shortcut settings sub-card -->
    <div class="sub-card">
      <h3>{translateNow('settings.shortcut.title', $currentLang)}</h3>
      <div class="form-grid-ai">
        <div class="form-item" style="display: block; width: 100%;">
          <label for="shortcut-display">{translateNow('settings.shortcut.screenshot', $currentLang)}</label>
          <div class="shortcut-setting-row">
            <div class="shortcut-keys" id="shortcut-display">
              {formatShortcutDisplay($configStore.screenshotShortcut || 'CommandOrControl+Shift+A')}
            </div>
            <div class="shortcut-buttons">
              {#if isRecording}
                <div class="recording-status">
                  <span class="recording-text">{translateNow('settings.shortcut.recording', $currentLang)}</span>
                  <div class="recording-indicator"></div>
                </div>
                <button class="btn btn-secondary" on:click={cancelRecording}>
                  {translateNow('common.cancel', $currentLang)}
                </button>
              {:else}
                <button
                  class="btn btn-test"
                  on:click={startRecording}
                >
                  {translateNow('settings.shortcut.modify', $currentLang)}
                </button>
              {/if}
              <button class="btn btn-primary btn-save" on:click={handleSaveShortcut}>{translateNow('settings.actions.save', $currentLang)}</button>
            </div>
          </div>
          {#if isRecording}
            <div
              class="key-capture-overlay"
              on:keydown={handleKeyDown}
              tabindex="0"
              role="button"
              aria-label="Press shortcut keys"
            ></div>
          {/if}
        </div>
      </div>
    </div>

    <!-- Window settings moved into General as a sub-card -->
    <div class="sub-card">
      <h3>{translateNow('settings.window.title', $currentLang)}</h3>
      <p class="subtitle">{translateNow('settings.window.subtitle', $currentLang)}</p>
      <div class="form-grid">
        <div class="form-item">
          <label for="win-width">{translateNow('settings.window.width', $currentLang)}</label>
          <input id="win-width" type="number" min="800" max="4096" bind:value={$configStore.windowWidth} />
        </div>
        <div class="form-item">
          <label for="win-height">{translateNow('settings.window.height', $currentLang)}</label>
          <input id="win-height" type="number" min="600" max="2160" bind:value={$configStore.windowHeight} />
        </div>
      </div>
      <div class="advanced-switches">
        <label class="switch">
          <input type="checkbox" bind:checked={$configStore.rememberWindowState} />
          <span>{translateNow('settings.window.remember', $currentLang)}</span>
        </label>
      </div>
      <div class="card-actions">
        <button class="btn btn-test" on:click={applyWindowToCurrent}>{translateNow('settings.window.apply_now', $currentLang)}</button>
        <button class="btn btn-primary btn-save" on:click={handleSaveConfig}>{translateNow('settings.actions.save', $currentLang)}</button>
      </div>
    </div>
  </div>

  <!-- AI Configuration Top-Level Card -->
  <div class="top-level-card">
    <h2>{translateNow('settings.ai.title', $currentLang)}</h2>
    <p class="subtitle">{translateNow('settings.ai.desc', $currentLang)}</p>
    
    <hr class="card-divider" />

    <!-- API Configuration Sub-Card -->
    <div class="sub-card">
      <h3>{translateNow('settings.api.title', $currentLang)}</h3>
      <div class="form-grid-ai">
        <!-- Provider -->
        <div class="form-item" style="display: block; width: 100%;">
          <label for="provider">{translateNow('settings.api.provider', $currentLang)}</label>
          <select id="provider" bind:value={$configStore.provider}>
            <option value="gemini">Google</option>
            <option value="openai">OpenAI</option>
            <option value="anthropic">Anthropic</option>
          </select>
        </div>

        <!-- API Key -->
        <div class="form-item" style="display: block; width: 100%;">
          <label for="api-key">{translateNow('settings.api.key', $currentLang)}</label>
          <input
            type="password"
            id="api-key"
            placeholder={translateNow('settings.api.key.ph', $currentLang)}
            bind:value={$configStore.apiKey}
          />
        </div>

        <!-- API Base URL -->
        <div class="form-item" style="display: block; width: 100%;">
          <label for="api-base-url">{translateNow('settings.api.base_url', $currentLang)}</label>
          <input
            type="text"
            id="api-base-url"
            placeholder={translateNow('settings.api.base_url.ph', $currentLang)}
            bind:value={$configStore.apiBaseUrl}
          />
          {#if previewUrl}
            <small class="helper-url" title={previewUrl}>{previewUrl}</small>
          {/if}
        </div>

        <!-- Model -->
        <div class="form-item">
          <label for="model">{translateNow('settings.api.model', $currentLang)}</label>
          <div class="input-with-button">
            <input
              type="text"
              id="model"
              placeholder={translateNow('settings.api.model.ph', $currentLang)}
              bind:value={$configStore.defaultEngine}
            />
            <button on:click={testConnection} class="btn btn-test" aria-label={translateNow('settings.actions.test', $currentLang)} disabled={testing}>
              {testing ? translateNow('settings.actions.testing', $currentLang) : translateNow('settings.actions.test', $currentLang)}
            </button>
          </div>
        </div>

      </div>
      <div class="card-actions">
        <button class="btn btn-primary btn-save" on:click={handleSaveConfig}>{translateNow('settings.actions.save', $currentLang)}</button>
      </div>
    </div>

    <!-- Prompt Configuration Sub-Card -->
    <div class="sub-card">
      <h3>{translateNow('settings.prompt.title', $currentLang)}</h3>
      <div class="form-grid-full">
        

        <div class="form-item">
          <label for="latex-prompt" class="label-row">
            <span>{translateNow('settings.prompt.latex_label', $currentLang)}</span>
            {#if $configStore?.__lastUsedLatexPrompt}
              <span class="last-used" title="上次实际调用的提示词">{$configStore.__lastUsedLatexPrompt}</span>
            {/if}
          </label>
          <textarea
            id="latex-prompt"
            class="prompt-textarea"
            placeholder="仅提取 LaTeX 的提示词"
            bind:value={$configStore.latexPrompt}
            on:input={autosize}
            on:mouseup={() => rememberHeight('latex')}
            rows="1"
            bind:this={latexPromptTextarea}
          ></textarea>
        </div>

        <div class="form-item">
          <label for="analysis-prompt" class="label-row">
            <span>{translateNow('settings.prompt.analysis_label', $currentLang)}</span>
            {#if $configStore?.__lastUsedAnalysisPrompt}
              <span class="last-used" title="上次实际调用的提示词">{$configStore.__lastUsedAnalysisPrompt}</span>
            {/if}
          </label>
          <textarea
            id="analysis-prompt"
            class="prompt-textarea"
            placeholder="用于输出标题、简介、变量表、项含义与建议的提示词"
            bind:value={$configStore.analysisPrompt}
            on:input={autosize}
            on:mouseup={() => rememberHeight('analysis')}
            rows="1"
            bind:this={analysisPromptTextarea}
          ></textarea>
        </div>

        <div class="form-item">
          <label for="confidence-prompt" class="label-row">
            <span>{translateNow('settings.prompt.verification_label', $currentLang)}</span>
            {#if $configStore?.__lastUsedVerificationPrompt}
              <span class="last-used" title="上次实际调用的提示词">{$configStore.__lastUsedVerificationPrompt}</span>
            {/if}
          </label>
          <textarea
            id="confidence-prompt"
            class="prompt-textarea"
            placeholder={translateNow('settings.prompt.confidence.ph', $currentLang)}
            bind:value={$configStore.verificationPrompt}
            on:input={autosize}
            on:mouseup={() => rememberHeight('confidence')}
            rows="1"
            bind:this={confidencePromptTextarea}
          ></textarea>
        </div>
      </div>
      <div class="card-actions">
        <button class="btn btn-test" on:click={resetPromptsToDefault}>{translateNow('settings.prompt.reset', $currentLang)}</button>
        <button class="btn btn-primary btn-save" on:click={handleSaveConfig}>{translateNow('settings.actions.save', $currentLang)}</button>
      </div>
    </div>

    <!-- Advanced moved under AI as a sub-card -->
    <div class="sub-card">
      <h3>{translateNow('settings.advanced.title', $currentLang)}</h3>
      <p class="subtitle">{translateNow('settings.advanced.desc', $currentLang)}</p>
      <div class="advanced-grid">
        <div class="advanced-col">
          <div class="form-item">
            <label for="timeout">{translateNow('settings.advanced.timeout', $currentLang)}</label>
            <input type="number" id="timeout" min="5" max="300" placeholder="30" bind:value={$configStore.requestTimeoutSeconds} />
          </div>
        </div>
        <div class="advanced-col">
          <div class="form-item">
            <label for="retries">{translateNow('settings.advanced.retries', $currentLang)}</label>
            <input type="number" id="retries" min="0" max="10" placeholder="3" bind:value={$configStore.maxRetries} />
          </div>
        </div>
        <div class="advanced-col">
          <div class="form-item">
            <label for="max-output-tokens">{translateNow('settings.advanced.max_output_tokens', $currentLang)}</label>
            <input type="number" id="max-output-tokens" min="1" max="32768" placeholder="4096" bind:value={$configStore.maxOutputTokens} />
          </div>
        </div>
      </div>
        <div class="advanced-switches">
          <label class="switch">
            <input type="checkbox" bind:checked={$configStore.autoCalculateConfidence} />
            <span>{translateNow('settings.advanced.auto_conf', $currentLang)}</span>
          </label>
          <label class="switch" title={translateNow('settings.advanced.clipboard_hint', $currentLang)}>
            <input type="checkbox" bind:checked={$configStore.enableClipboardWatcher} disabled />
            <span>{translateNow('settings.advanced.clipboard_pending', $currentLang)}</span>
          </label>
        </div>
      <div class="card-actions">
        <button class="btn btn-primary btn-save" on:click={handleSaveConfig}>{translateNow('settings.actions.save', $currentLang)}</button>
      </div>
    </div>
  </div>

  <!-- Display Configuration Top-Level Card -->
  <div class="top-level-card">
    <h2>{translateNow('settings.display', $currentLang)}</h2>
    <p class="subtitle">{translateNow('settings.display.subtitle', $currentLang)}</p>
    <div class="form-grid">
      <div class="form-item">
        <label for="render-engine">{translateNow('settings.display.render_engine', $currentLang)}</label>
        <select id="render-engine" bind:value={$configStore.renderEngine}>
          <option value="MathJax">{translateNow('settings.display.engine.mathjax', $currentLang)}</option>
          <option value="KaTeX">{translateNow('settings.display.engine.katex', $currentLang)}</option>
        </select>
      </div>

      <div class="form-item">
        <label for="latex-format">{translateNow('settings.display.latex_format', $currentLang)}</label>
        <select id="latex-format" bind:value={$configStore.defaultLatexFormat}>
          <option value="raw">{translateNow('settings.display.format.raw', $currentLang)}</option>
          <option value="single_dollar">{translateNow('settings.display.format.single', $currentLang)}</option>
          <option value="double_dollar">{translateNow('settings.display.format.double', $currentLang)}</option>
          <option value="equation">{translateNow('settings.display.format.equation', $currentLang)}</option>
          <option value="bracket">{translateNow('settings.display.format.bracket', $currentLang)}</option>
        </select>
      </div>
    </div>
  </div>

  <!-- About Section -->
  <div class="top-level-card">
    <h2>{translateNow('about.title', $currentLang)}</h2>

    <div class="about-content">
      <div class="app-info">
        <h3 class="app-name">{translateNow('about.app_name', $currentLang)}</h3>
        <p class="app-version">{translateNow('about.version', $currentLang)}</p>
        <p class="app-description">{translateNow('about.description', $currentLang)}</p>
      </div>

      <div class="tech-stack-section">
        <h4>{translateNow('about.tech_stack', $currentLang)}</h4>
        <div class="tech-tags">
          <span class="tech-tag tauri">Tauri</span>
          <span class="tech-tag svelte">Svelte</span>
          <span class="tech-tag rust">Rust</span>
          <span class="tech-tag typescript">TypeScript</span>
          <span class="tech-tag vite">Vite</span>
        </div>
      </div>
    </div>
  </div>

  </div>
</div>

<style>
  .settings-view {
    padding: 0; /* 统一由 layout 控制左右留白 */
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xl);
    background-color: rgb(248, 250, 252);
    --control-height: 44px; /* 统一控件高度 */
  }

  .page-header { margin-bottom: var(--spacing-lg); }
  .page-title { font-size: var(--font-size-h1); font-weight: var(--font-weight-bold); color: var(--text-header); margin: 0; }

  .top-level-card {
    background-color: rgb(255, 255, 255);
    padding: var(--spacing-xl) var(--spacing-lg);
    border-radius: var(--border-radius-card-lg, 16px);
    box-shadow: var(--card-shadow-lg, 0 8px 16px rgba(0,0,0,0.1));
    border: 1px solid var(--border-primary);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-lg);
  }

  .top-level-card h2 {
    font-size: var(--font-size-h2);
    font-weight: var(--font-weight-bold);
    color: var(--text-header);
    margin: 0;
  }

  .top-level-card .subtitle {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    margin-top: calc(-1 * var(--spacing-md));
    margin-bottom: var(--spacing-xs);
  }

  .card-divider {
    border: none;
    border-top: 1px solid var(--border-secondary, #e5e7eb);
    margin-top: 0;
    margin-bottom: var(--spacing-xs);
  }

  .sub-card {
    background-color: rgb(248, 250, 252); /* Keep original background color */
    padding: var(--spacing-lg);
    border-radius: var(--border-radius-card-lg, 16px); /* Copied from .top-level-card */
    box-shadow: var(--card-shadow-lg, 0 8px 16px rgba(0,0,0,0.1)); /* Copied from .top-level-card */
    border: 1px solid var(--border-primary); /* Copied from .top-level-card */
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }

  .sub-card h3 {
    font-size: var(--font-size-h3);
    font-weight: var(--font-weight-semibold);
    color: var(--text-header);
    margin: 0 0 var(--spacing-sm) 0;
  }

  .form-grid,
  .form-grid-ai,
  .form-grid-full {
    display: grid;
    gap: var(--spacing-lg);
  }

  .form-grid {
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  }
  
  .form-grid-ai {
    /* This is the key change: Force a single column layout */
    grid-template-columns: 1fr;
  }

  .form-grid-full {
    grid-template-columns: 1fr;
  }

  .form-item {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }
  .label-row { display:flex; align-items:center; justify-content: space-between; gap: var(--spacing-sm); }
  .last-used { max-width: 55%; color: var(--text-muted); font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; text-align: right; }


  label {
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-semibold);
    color: var(--text-default);
  }

  input,
  select,
  textarea {
    border-radius: var(--border-radius-btn);
    border: var(--input-border-width) solid var(--border-primary);
    padding: var(--spacing-sm) var(--spacing-base);
    background-color: var(--bg-tertiary);
    width: 100%;
    font-size: var(--font-size-body);
    transition: border-color 0.2s ease, box-shadow 0.2s ease;
  }

  input:focus,
  select:focus,
  textarea:focus {
    outline: none;
    border-color: var(--focus);
    box-shadow: 0 0 0 2px rgba(102, 126, 234, 0.15);
  }

  .input-with-button {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
  }

  .input-with-button input {
    flex-grow: 1;
  }

  .helper-url {
    color: var(--text-muted);
    font-size: var(--font-size-small);
    user-select: text;
    word-break: break-all;
  }

  .btn {
    height: var(--control-height);
    line-height: var(--control-height);
    padding: 0 var(--spacing-base);
    font-size: var(--font-size-label);
    border-radius: var(--border-radius-btn);
    font-weight: var(--font-weight-medium);
    transition: background-color 0.2s ease, transform 0.1s ease;
    cursor: pointer;
    border: none;
  }

  .btn-primary {
    background-color: var(--primary);
    color: var(--text-inverse);
    box-shadow: var(--button-shadow);
  }
  
  .btn-primary:hover {
    background-color: var(--primary-hover);
    transform: translateY(-1px);
    box-shadow: var(--button-shadow-hover);
  }

  /* removed unused .btn-secondary styles */


  .btn-test {
    padding: 0 var(--spacing-base);
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-semibold);
    color: var(--text-default);
    background-color: var(--bg-tertiary);
    border: var(--input-border-width) solid var(--border-primary);
    border-radius: var(--border-radius-btn);
    transition: background-color 0.2s ease, transform 0.1s ease;
    cursor: pointer;
    white-space: nowrap; /* 中文不换行 */
  }

  .btn-test:hover {
    background-color: var(--bg-secondary);
    border-color: var(--border-secondary);
  }

  .btn-save {
    /* Inherit from h3 */
    font-size: var(--font-size-h3);
    font-weight: var(--font-weight-semibold);
    color: rgb(255, 255, 255);
    background-color: var(--primary);
  }

  .card-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-base);
    margin-top: var(--spacing-md);
    flex-wrap: wrap;
  }

  textarea.prompt-textarea {
    font-family: inherit; /* Ensure consistent font with input fields */
    resize: vertical; /* 允许用户手动调节高度 */
    min-height: 80px;
    line-height: 1.5;
  }

  /* cleanup: remove unused selectors */

  /* Advanced layout improvements */
  .advanced-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(240px, 1fr));
    gap: var(--spacing-lg);
    align-items: end;
  }

  .advanced-col .form-item input {
    height: 44px;
    padding: 10px 12px;
  }

  .advanced-switches {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: var(--spacing-base);
    margin-top: var(--spacing-lg);
  }

  .switch {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-base);
    border: 1px solid var(--border-primary);
    border-radius: var(--border-radius-btn);
    background-color: var(--bg-main);
    width: 100%;
    justify-content: flex-start;
  }

  .switch input {
    width: 18px;
    height: 18px;
  }

  .switch span {
    white-space: nowrap;
  }

  @media (max-width: 480px) {
    .switch span { white-space: normal; }
  }

  /* Shortcut Setting Styles */
  .shortcut-setting-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-base);
    justify-content: space-between;
  }

  .shortcut-keys {
    font-family: var(--font-family-code);
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-medium);
    color: var(--text-header);
    background-color: var(--bg-secondary);
    padding: var(--spacing-xs) var(--spacing-sm);
    border-radius: var(--border-radius-sm);
    border: 1px solid var(--border-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    letter-spacing: 0.5px;
    min-width: 160px;
    flex: 1;
    max-width: 200px;
    white-space: nowrap;
  }

  .shortcut-buttons {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    margin-left: auto;
  }

  .recording-status {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    color: var(--status-warning);
    font-size: var(--font-size-sm);
  }

  .recording-text {
    font-weight: var(--font-weight-medium);
    animation: fade-pulse 1.5s infinite;
  }

  @keyframes fade-pulse {
    0%, 100% { opacity: 0.7; }
    50% { opacity: 1; }
  }

  .recording-indicator {
    width: 8px;
    height: 8px;
    background-color: var(--status-warning);
    border-radius: 50%;
    animation: blink-dot 1s infinite;
  }

  @keyframes blink-dot {
    0%, 50% { opacity: 1; }
    51%, 100% { opacity: 0.3; }
  }

  .key-capture-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 9999;
    background-color: rgba(0, 0, 0, 0.1);
    outline: none;
  }

  /* footer-actions removed; use .card-actions inside the card instead */

  /* About Section Styles */
  .about-content {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xl);
  }

  .app-info {
    text-align: left;
  }

  .app-name {
    font-size: var(--font-size-h2);
    font-weight: var(--font-weight-bold);
    color: var(--text-header);
    margin: 0 0 var(--spacing-xs) 0;
  }

  .app-version {
    font-size: var(--font-size-base);
    color: var(--text-muted);
    margin: 0 0 var(--spacing-sm) 0;
  }

  .app-description {
    font-size: var(--font-size-base);
    color: var(--text-default);
    margin: 0;
    line-height: 1.6;
  }

  .tech-stack-section h4 {
    font-size: var(--font-size-h4);
    font-weight: var(--font-weight-semibold);
    color: var(--text-header);
    margin: 0 0 var(--spacing-sm) 0;
  }

  .tech-tags {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-sm);
  }

  .tech-tag {
    display: inline-flex;
    align-items: center;
    padding: var(--spacing-xs) var(--spacing-sm);
    border-radius: var(--border-radius-btn);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    color: white;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .tech-tag.tauri {
    background-color: #24292e;
  }

  .tech-tag.svelte {
    background-color: #ff3e00;
  }

  .tech-tag.rust {
    background-color: #ce422b;
  }

  .tech-tag.typescript {
    background-color: #3178c6;
  }

  .tech-tag.vite {
    background-color: #646cff;
  }
</style>