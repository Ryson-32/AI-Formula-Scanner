<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/tauri';
  import type { Config } from '$lib/types';
  import { currentLang, translateNow } from '$lib/i18n';
  
  // 接收LaTeX字符串作为输入
  export let latex: string = '';
  // 接收渲染引擎作为输入，默认为MathJax
  export let renderEngine: string = 'MathJax';
  // 预览模式：固定高度并自动缩放，避免滚动条
  export let mode: 'preview' | 'full' = 'full';
  // 预览高度（仅在 mode = 'preview' 时生效）
  export let previewHeight: number = 100;
  
  let formulaElement: HTMLElement;
  let contentElement: HTMLElement;
  
  let engineReady = false;
  let lastLatex = '';
  let lastEngine = '';
  let lastSuccessfulHTML = '';

  // 在组件挂载时加载MathJax或KaTeX
  onMount(async () => {
    // 从配置中获取默认渲染引擎
    try {
      const config = await invoke<Config>('get_config');
      renderEngine = config.renderEngine;
    } catch (err) {
      const error = err as Error;
      console.error('Failed to load render engine config:', error);
    }
    
    // 加载渲染引擎
    loadRenderEngine();
    engineReady = true;
    maybeRender();
  });

  // 当输入或引擎变化时尝试渲染（确保依赖被跟踪）
  $: if (engineReady) {
    const __deps = [latex, renderEngine];
    maybeRender();
  }
  
  // 加载渲染引擎
  function loadRenderEngine() {
    if (renderEngine === 'MathJax' && !(window as any).MathJax) {
      // 加载MathJax
      const script = document.createElement('script');
      script.src = 'https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-svg.js';
      script.async = true;
      script.onload = () => {
        requestAnimationFrame(maybeRender);
      };
      document.head.appendChild(script);
    } else if (renderEngine === 'KaTeX' && !(window as any).katex) {
      // 加载KaTeX
      const script = document.createElement('script');
      script.src = 'https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/katex.min.js';
      script.async = true;
      script.onload = () => {
        requestAnimationFrame(maybeRender);
      };
      document.head.appendChild(script);
      
      // 加载KaTeX CSS
      const link = document.createElement('link');
      link.rel = 'stylesheet';
      link.href = 'https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/katex.min.css';
      document.head.appendChild(link);
    }
  }
  
  function engineAvailable() {
    return (renderEngine === 'MathJax' && (window as any).MathJax)
      || (renderEngine === 'KaTeX' && (window as any).katex);
  }
  
  function renderedHasError(element: HTMLElement): boolean {
    if (!element) return true;
    // KaTeX 错误标记
    if (element.querySelector('.katex-error')) return true;
    // MathJax v3 错误标记
    if (element.querySelector('mjx-merror')) return true;
    const text = element.textContent || '';
    // 常见错误文案（KaTeX/MathJax）
    const patterns = [
      'ParseError',
      'Error:',
      'Extra close brace',
      'missing open brace',
      'Missing',
      'Undefined control sequence'
    ];
    return patterns.some((p) => text.includes(p));
  }
  
  // 渲染LaTeX公式（带去抖）
  function maybeRender() {
    if (!engineReady) return;
    if (!engineAvailable()) {
      // 引擎尚未就绪，稍后重试，不更新 last 缓存
      setTimeout(maybeRender, 60);
      return;
    }
    if (latex === lastLatex && renderEngine === lastEngine) return;
    lastLatex = latex;
    lastEngine = renderEngine;
    if (!latex) return;
    try {
      if (renderEngine === 'MathJax' && (window as any).MathJax) {
        // 使用MathJax渲染（优先 tex2svg，避免 typeset 队列导致的延迟/不刷新）
        if (contentElement) {
          const MJ = (window as any).MathJax;
          contentElement.innerHTML = '';
          if (MJ.tex2svg) {
            const node = MJ.tex2svg(latex, { display: true });
            contentElement.appendChild(node);
          } else {
            contentElement.innerHTML = `$$${latex}$$`;
            if (MJ.typeset) MJ.typeset([contentElement]);
          }
          if (renderedHasError(contentElement)) {
            if (lastSuccessfulHTML) {
              contentElement.innerHTML = lastSuccessfulHTML;
            } else {
              contentElement.innerHTML = '';
              contentElement.textContent = latex || translateNow('formulas.placeholder', $currentLang);
              contentElement.classList.add('latex-fallback');
            }
          } else {
            lastSuccessfulHTML = contentElement.innerHTML;
          }
        }
      } else if (renderEngine === 'KaTeX' && (window as any).katex) {
        // 使用KaTeX渲染
        if (contentElement) {
          (window as any).katex.render(latex, contentElement, {
            displayMode: true,
            throwOnError: false
          });
          if (renderedHasError(contentElement)) {
            if (lastSuccessfulHTML) {
              contentElement.innerHTML = lastSuccessfulHTML;
            } else {
              contentElement.innerHTML = '';
              contentElement.textContent = latex || translateNow('formulas.placeholder', $currentLang);
              contentElement.classList.add('latex-fallback');
            }
          } else {
            lastSuccessfulHTML = contentElement.innerHTML;
          }
        }
      }
      // 一帧后进行自适应缩放，确保获取到排版后的尺寸
      requestAnimationFrame(fitToContainer);
    } catch (err) {
      const error = err as Error;
      console.error('Error rendering formula:', error);
      if (!contentElement) return;
      // 失败时优先回退到上一次成功渲染，否则显示原始 LaTeX 文本
      if (lastSuccessfulHTML) {
        contentElement.innerHTML = lastSuccessfulHTML;
      } else {
        contentElement.textContent = latex || translateNow('formulas.placeholder', $currentLang);
        contentElement.classList.add('latex-fallback');
      }
    }
  }

  function fitToContainer() {
    if (mode !== 'preview') {
      if (contentElement) contentElement.style.transform = 'none';
      return;
    }
    if (!formulaElement || !contentElement) return;
    // 重置缩放以获取自然尺寸
    contentElement.style.transform = 'none';
    const cw = formulaElement.clientWidth;
    const ch = formulaElement.clientHeight;
    const iw = contentElement.scrollWidth || contentElement.offsetWidth;
    const ih = contentElement.scrollHeight || contentElement.offsetHeight;
    if (!iw || !ih) return;
    const scale = Math.min(1, cw / iw, ch / ih);
    contentElement.style.transformOrigin = 'center center';
    contentElement.style.transform = `scale(${scale})`;
  }
</script>

<div class="formula-renderer {mode === 'preview' ? 'preview' : 'full'}" bind:this={formulaElement} style={mode === 'preview' ? `height:${previewHeight}px` : ''}>
  <div class="formula-content" bind:this={contentElement}></div>
  {#if !latex}
    <div class="placeholder">{translateNow('formulas.placeholder', $currentLang)}</div>
  {/if}
</div>

<style>
  .formula-renderer {
    width: 100%;
    min-height: 80px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-base);
    background-color: var(--bg-secondary);
    border-radius: var(--border-radius-card);
    border: 1px solid var(--border-primary);
    box-shadow: var(--card-shadow);
    color: var(--text-default);
    overflow: hidden; /* 预览不出现滚动条 */
  }

  .formula-renderer.preview {
    min-height: 0; /* 使用固定高度 */
  }

  .formula-content {
    color: var(--text-default);
  }
  :global(.formula-renderer svg),
  :global(.formula-renderer svg *),
  :global(.formula-renderer .katex),
  :global(.formula-renderer .katex *) {
    color: var(--text-default) !important;
    fill: var(--text-default) !important;
    stroke: var(--text-default) !important;
  }
  
  .placeholder {
    color: var(--text-muted);
    font-style: italic;
    font-size: var(--font-size-body);
  }
  .latex-fallback {
    white-space: pre-wrap;
    font-family: var(--font-family-code, ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace);
    font-size: var(--font-size-body);
    background: #fff;
    color: #111;
    border-radius: var(--border-radius-card);
    padding: var(--spacing-sm);
  }
  
  :global(.error) {
    color: var(--status-error);
    font-size: var(--font-size-small);
    background-color: var(--status-error-bg);
    padding: var(--spacing-xs) var(--spacing-sm);
    border-radius: var(--border-radius-btn);
  }
  
  /* 确保公式居中显示 */
  :global(.formula-renderer .MathJax),
  :global(.formula-renderer .katex) {
    margin: 0 auto;
  }
</style>