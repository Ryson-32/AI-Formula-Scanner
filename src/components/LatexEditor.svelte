<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { onMount } from 'svelte';
  import { currentLang, translateNow } from '$lib/i18n';
  
  // 接收LaTeX字符串作为输入
  export let latex: string = '';
  // 是否禁用编辑
  export let disabled: boolean = false;

  const STORAGE_KEY = 'latexEditorHeight';
  let textareaEl: HTMLTextAreaElement | null = null;
  let editorHeightPx = 280; // 默认高度 280px（可记忆）
  
  // 创建事件分发器，用于向父组件发送更新事件
  const dispatch = createEventDispatcher<{
    update: { latex: string };
  }>();
  
  // 处理输入变化
  function handleInput(event: Event) {
    const target = event.target as HTMLTextAreaElement;
    latex = target.value;
    dispatch('update', { latex });
  }

  function clampHeight(px: number): number {
    const MIN = 120;
    const MAX = 1000;
    if (!Number.isFinite(px)) return editorHeightPx;
    return Math.max(MIN, Math.min(MAX, Math.round(px)));
  }

  function persistHeightFromElement() {
    if (!textareaEl) return;
    const h = textareaEl.offsetHeight;
    const next = clampHeight(h);
    if (Math.abs(next - editorHeightPx) > 1) {
      editorHeightPx = next;
      try { localStorage.setItem(STORAGE_KEY, String(editorHeightPx)); } catch {}
    }
  }

  onMount(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        const parsed = parseInt(saved, 10);
        if (!Number.isNaN(parsed)) editorHeightPx = clampHeight(parsed);
      }
    } catch {}
  });
</script>

<div class="latex-editor">
  <textarea
    class="editor-textarea"
    placeholder={translateNow('editor.placeholder', $currentLang)}
    value={latex}
    on:input={handleInput}
    {disabled}
    rows="4"
    bind:this={textareaEl}
    on:mouseup={persistHeightFromElement}
    on:touchend={persistHeightFromElement}
    on:blur={persistHeightFromElement}
    style={`height:${editorHeightPx}px`}
  ></textarea>
  
  <!-- 简化：移除四个模板按钮，减少空间占用 -->
</div>

<style>
  .latex-editor {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }
  
  .editor-textarea {
    width: 100%;
    padding: var(--spacing-md);
    border: var(--input-border-width) solid var(--border-primary);
    border-radius: var(--border-radius-btn);
    font-family: var(--font-family-code);
    font-size: var(--font-size-body);
    resize: vertical;
    min-height: 120px;
    background-color: var(--bg-main);
    color: var(--text-default);
  }
  
  .editor-textarea:focus {
    outline: none;
    border-color: var(--focus);
    box-shadow: 0 0 0 2px rgba(102, 126, 234, 0.15);
  }
  
  /* 清理已移除按钮相关样式（.editor-controls / .control-button） */
</style>