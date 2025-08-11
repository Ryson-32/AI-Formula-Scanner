<script lang="ts">
  import FormulaRenderer from './FormulaRenderer.svelte';
  
  // 接收包含LaTeX的文本
  export let text: string = '';
  // 渲染引擎，默认为MathJax
  export let renderEngine: string = 'MathJax';
  
  // 解析文本，识别LaTeX代码并分割
  function parseTextWithLatex(inputText: string) {
    if (!inputText) return [];

    // 更智能的LaTeX模式匹配
    // 优先匹配更长、更完整的表达式
    const patterns = [
      // 0. 复杂的Unicode数学表达式：∂(ρuw)/∂z, ∂(ρv)/∂t 等
      // 匹配偏导数表达式，包括分子分母
      /∂\([^)]+\)\/∂[a-zA-Z]+/g,

      // 0.1. 简单的偏导数：∂/∂x, ∂/∂y, ∂/∂z 等
      /∂\/∂[a-zA-Z]+/g,

      // 0.2. Unicode数学符号组合：包含多个符号的表达式
      /[∂∇∆∑∏∫αβγδεζηθικλμνξοπρστυφχψωΑΒΓΔΕΖΗΘΙΚΛΜΝΞΟΠΡΣΤΥΦΧΨΩ][^a-zA-Z\s]*[∂∇∆∑∏∫αβγδεζηθικλμνξοπρστυφχψωΑΒΓΔΕΖΗΘΙΚΛΜΝΞΟΠΡΣΤΥΦΧΨΩ]/g,

      // 0.3. 单独的Unicode数学符号：∂, ρ, α, β 等
      /[∂∇∆∑∏∫αβγδεζηθικλμνξοπρστυφχψωΑΒΓΔΕΖΗΘΙΚΛΜΝΞΟΠΡΣΤΥΦΧΨΩ]/g,

      // 1. 完整的数学表达式：包含命令+下标+上标+后续变量的组合
      // 如：\sum_{i=1}^{n} A_i X_i, \int_a^b f(x) dx
      /\\[a-zA-Z]+(?:_\{[^}]*\})?(?:\^\{[^}]*\})?\s*(?:[a-zA-Z]+(?:_\{[^}]*\}|\^\{[^}]*\}|_[a-zA-Z0-9]+|\^[a-zA-Z0-9]+)*\s*)*(?:[a-zA-Z]+(?:_\{[^}]*\}|\^\{[^}]*\}|_[a-zA-Z0-9]+|\^[a-zA-Z0-9]+)*)?/g,

      // 2. 完整的LaTeX命令：\command{...}，支持嵌套大括号
      /\\[a-zA-Z]+(?:\s*\{(?:[^{}]|\{[^{}]*\})*\})*(?:\s*\[[^\]]*\])?/g,

      // 3. 复杂的数学表达式：包含下标和上标的组合
      // 匹配如 \sum_{i=1}^{n}, \int_a^b, \prod_{i=1}^{n} 等
      /\\[a-zA-Z]+(?:_\{[^}]*\})?(?:\^\{[^}]*\})?/g,

      // 4. 变量名带下标/上标：A_i, X_{i+1}, x^2, y^{n+1} 等
      /[a-zA-Z]+(?:_\{[^}]*\}|\^\{[^}]*\}|_[a-zA-Z0-9]+|\^[a-zA-Z0-9]+)+/g,

      // 5. 单独的下标表达式：_{...}
      /_\{[^}]*\}/g,

      // 6. 单独的上标表达式：^{...}
      /\^\{[^}]*\}/g,

      // 7. 简单的下标：_x, _1 等
      /_[a-zA-Z0-9]+/g,

      // 8. 简单的上标：^2, ^n 等
      /\^[a-zA-Z0-9]+/g
    ];

    // 找到所有匹配项并按位置排序
    const matches: Array<{start: number, end: number, content: string}> = [];
    patterns.forEach(pattern => {
      let match;
      const regex = new RegExp(pattern.source, pattern.flags);
      while ((match = regex.exec(inputText)) !== null) {
        matches.push({
          start: match.index,
          end: match.index + match[0].length,
          content: match[0]
        });
      }
    });

    // 按开始位置排序并合并重叠的匹配
    matches.sort((a, b) => a.start - b.start);
    const mergedMatches: Array<{start: number, end: number, content: string}> = [];
    for (const match of matches) {
      const lastMatch = mergedMatches[mergedMatches.length - 1];
      if (lastMatch && match.start <= lastMatch.end) {
        // 合并重叠的匹配，取更长的那个
        if (match.end > lastMatch.end) {
          lastMatch.end = match.end;
          lastMatch.content = inputText.slice(lastMatch.start, lastMatch.end);
        }
      } else {
        mergedMatches.push(match);
      }
    }

    // 构建最终的parts数组
    const parts = [];
    let lastIndex = 0;

    for (const match of mergedMatches) {
      // 添加LaTeX之前的普通文本
      if (match.start > lastIndex) {
        const beforeText = inputText.slice(lastIndex, match.start);
        if (beforeText.trim()) {
          parts.push({ type: 'text', content: beforeText });
        }
      }

      // 添加LaTeX部分
      parts.push({ type: 'latex', content: match.content });
      lastIndex = match.end;
    }

    // 添加最后剩余的普通文本
    if (lastIndex < inputText.length) {
      const remainingText = inputText.slice(lastIndex);
      if (remainingText.trim()) {
        parts.push({ type: 'text', content: remainingText });
      }
    }

    // 如果没有找到LaTeX，返回整个文本作为普通文本
    if (parts.length === 0) {
      parts.push({ type: 'text', content: inputText });
    }

    return parts;
  }
  
  $: parsedParts = parseTextWithLatex(text);
</script>

<div class="latex-text-renderer">
  {#each parsedParts as part}
    {#if part.type === 'latex'}
      <span class="inline-latex">
        <FormulaRenderer
          latex={part.content}
          {renderEngine}
          mode="preview"
          previewHeight={18}
        />
      </span>
    {:else}
      <span class="text-part">{part.content}</span>
    {/if}
  {/each}
</div>

<style>
  .latex-text-renderer {
    display: inline-block;
    line-height: 1.5;
    vertical-align: top;
    width: 100%;
    word-wrap: break-word;
    overflow-wrap: break-word;
  }

  .inline-latex {
    display: inline-block;
    vertical-align: middle;
    margin: 0 2px;
    max-width: 100%;
    overflow-x: auto;
    overflow-y: visible;
  }

  .text-part {
    display: inline;
    vertical-align: middle;
  }

  /* 确保行内LaTeX公式的样式 */
  .inline-latex :global(.formula-renderer) {
    display: inline-block;
    vertical-align: middle;
    max-width: 100%;
  }

  .inline-latex :global(.formula-content) {
    display: inline-block;
    vertical-align: middle;
    font-size: 0.9em;
    max-width: 100%;
    overflow-x: auto;
  }

  /* 调整MathJax渲染的行内公式样式 */
  .inline-latex :global(mjx-container) {
    display: inline-block !important;
    vertical-align: middle !important;
    margin: 0 2px !important;
    padding: 0 !important;
    max-width: 100% !important;
    overflow-x: auto !important;
  }

  .inline-latex :global(mjx-math) {
    display: inline-block !important;
    vertical-align: middle !important;
  }

  /* 调整KaTeX渲染的行内公式样式 */
  .inline-latex :global(.katex) {
    display: inline-block !important;
    vertical-align: middle !important;
    font-size: 0.9em;
    margin: 0 2px !important;
    max-width: 100% !important;
  }

  .inline-latex :global(.katex-html) {
    display: inline-block !important;
    vertical-align: middle !important;
  }

  /* 防止公式过长时的换行处理 */
  .inline-latex :global(.katex-display) {
    margin: 0 !important;
    text-align: left !important;
  }
</style>
