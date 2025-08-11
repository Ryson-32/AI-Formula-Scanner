<script lang="ts">
  import { onMount, afterUpdate } from 'svelte';

  // 接收验证报告数据
  export let verification: any = null;
  export let renderEngine: string = 'MathJax';

  let containerElement: HTMLElement;

  // 处理LaTeX代码的函数
  function processLatexInText(text: string): string {
    if (!text) return '';

    // 更全面的LaTeX模式匹配，包括下标、上标和LaTeX命令
    const patterns = [
      // LaTeX命令：\command{...}
      /\\[a-zA-Z]+(?:\s*\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\})*(?:\s*\[[^\]]*\])?/g,
      // 下标表达式：f_{b,x}、u_i 等
      /[a-zA-Z]+_\{[^}]+\}/g,
      // 简单下标：f_x、u_i 等
      /[a-zA-Z]+_[a-zA-Z0-9]+/g,
      // 上标表达式：x^{n+1} 等
      /[a-zA-Z]+\^\{[^}]+\}/g,
      // 简单上标：x^2 等
      /[a-zA-Z]+\^[a-zA-Z0-9]+/g
    ];

    let result = text;

    // 依次应用所有模式
    patterns.forEach(pattern => {
      result = result.replace(pattern, (match) => {
        // 避免重复包装已经处理过的内容
        if (match.startsWith('\\(') && match.endsWith('\\)')) {
          return match;
        }
        return `\\(${match}\\)`;
      });
    });

    return result;
  }

  // 渲染LaTeX公式
  function renderMath() {
    if (!containerElement) return;

    try {
      if (renderEngine === 'MathJax' && (window as any).MathJax) {
        const MJ = (window as any).MathJax;
        if (MJ.typeset) {
          MJ.typeset([containerElement]);
        }
      } else if (renderEngine === 'KaTeX' && (window as any).katex) {
        // KaTeX需要手动处理每个公式
        const mathElements = containerElement.querySelectorAll('.math-inline');
        mathElements.forEach((element) => {
          const latex = element.textContent?.replace(/^\$|\$$/g, '') || '';
          try {
            (window as any).katex.render(latex, element as HTMLElement, {
              displayMode: false,
              throwOnError: false
            });
          } catch (e) {
            console.warn('KaTeX render error:', e);
          }
        });
      }
    } catch (error) {
      console.error('Math rendering error:', error);
    }
  }

  onMount(() => {
    // 延迟渲染，确保DOM已经更新
    setTimeout(renderMath, 100);
  });

  afterUpdate(() => {
    // 内容更新后重新渲染
    setTimeout(renderMath, 50);
  });
</script>

{#if verification}
  <div class="verification-content" bind:this={containerElement}>
    <div class="verification-status {(verification.status || '').toLowerCase()}">
      状态: {verification.status}
    </div>

    {#if verification.issues && verification.issues.length > 0}
      <div class="verification-issues">
        <h5>发现的问题:</h5>
        <ul>
          {#each verification.issues as issue}
            <li class="issue {issue.category}">
              <div class="issue-content">
                {@html processLatexInText(issue.message)}
              </div>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if verification.coverage}
      <div class="verification-coverage">
        <h5>覆盖率统计:</h5>
        <div class="coverage-stats">
          <div>符号匹配: {verification.coverage.symbols_matched}/{verification.coverage.symbols_total}</div>
          <div>术语匹配: {verification.coverage.terms_matched}/{verification.coverage.terms_total}</div>
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .verification-content {
    font-size: var(--font-size-body);
    line-height: 1.6;
  }
  
  .verification-status {
    padding: var(--spacing-sm);
    border-radius: var(--border-radius-btn);
    margin-bottom: var(--spacing-md);
    font-weight: var(--font-weight-medium);
  }
  
  .verification-status.ok {
    background-color: var(--status-success-bg);
    color: var(--status-success);
    border: 1px solid var(--status-success);
  }
  
  .verification-status.warning {
    background-color: var(--status-warning-bg);
    color: var(--status-warning);
    border: 1px solid var(--status-warning);
  }
  
  .verification-status.error {
    background-color: var(--status-error-bg);
    color: var(--status-error);
    border: 1px solid var(--status-error);
  }
  
  .verification-issues {
    margin-bottom: var(--spacing-md);
  }
  
  .verification-issues h5 {
    margin: 0 0 var(--spacing-sm) 0;
    color: var(--text-header);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
  }
  
  .verification-issues ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  
  .issue {
    margin-bottom: var(--spacing-sm);
    padding: var(--spacing-sm);
    border-radius: var(--border-radius-btn);
    border-left: 3px solid var(--border-secondary);
    background-color: var(--bg-hover);
  }
  
  .issue.symbol_mismatch {
    border-left-color: var(--status-error);
  }
  
  .issue.missing_term {
    border-left-color: var(--status-warning);
  }
  
  .issue.extra_term {
    border-left-color: var(--status-warning);
  }
  
  .issue.notation_mismatch {
    border-left-color: var(--status-info);
  }
  
  .issue.layout_mismatch {
    border-left-color: var(--status-info);
  }
  
  .issue.other {
    border-left-color: var(--border-secondary);
  }
  
  .issue-content {
    line-height: 1.6;
    word-wrap: break-word;
  }
  
  /* LaTeX渲染样式 */
  .issue-content :global(.MathJax) {
    display: inline !important;
    vertical-align: baseline !important;
  }
  
  .issue-content :global(mjx-container) {
    display: inline !important;
    vertical-align: baseline !important;
    margin: 0 2px !important;
  }
  
  .issue-content :global(.katex) {
    display: inline !important;
    vertical-align: baseline !important;
    font-size: 1em !important;
    margin: 0 2px !important;
  }
  
  .verification-coverage {
    margin-top: var(--spacing-md);
  }
  
  .verification-coverage h5 {
    margin: 0 0 var(--spacing-sm) 0;
    color: var(--text-header);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
  }
  
  .coverage-stats {
    display: flex;
    gap: var(--spacing-md);
    flex-wrap: wrap;
  }
  
  .coverage-stats div {
    padding: var(--spacing-xs) var(--spacing-sm);
    background-color: var(--bg-hover);
    border-radius: var(--border-radius-btn);
    font-size: var(--font-size-sm);
  }
</style>
