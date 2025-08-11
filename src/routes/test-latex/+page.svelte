<script lang="ts">
  import LatexTextRenderer from '../../components/LatexTextRenderer.svelte';
  
  // 测试数据，模拟验证报告中的问题
  const testMessages = [
    '左侧第一个方程的第一个项 \\frac{\\partial \\rho u}{\\partial t} 在图像中为 \\frac{\\partial \\rho u}{\\partial t}。',
    'LaTeX 中的第一个矩阵的第二行缺少 \\frac{\\partial \\rho u v}{\\partial x} 项。',
    'LaTeX 中的第一个矩阵的第三行缺少 \\frac{\\partial \\rho u w}{\\partial x} 项。',
    '右侧第一个方程的第一行缺少 \\frac{\\partial}{\\partial y}[\\mu(\\frac{\\partial v}{\\partial x} + \\frac{\\partial u}{\\partial y})] 项。',
    '这是一个包含多个LaTeX符号的测试：\\alpha, \\beta, \\gamma, \\partial, \\rho, \\mu, \\lambda。',
    '复杂的LaTeX表达式：\\frac{\\partial^2 u}{\\partial x^2} + \\frac{\\partial^2 u}{\\partial y^2} = 0',
    '普通文本，没有LaTeX代码。'
  ];
</script>

<div class="test-page">
  <h1>LaTeX文本渲染测试</h1>
  
  <div class="test-section">
    <h2>验证报告问题列表测试</h2>
    <p>以下是模拟验证报告中的问题消息，测试LaTeX代码是否正确渲染：</p>
    
    <ul class="issue-list">
      {#each testMessages as message, index}
        <li class="issue-item">
          <strong>问题 {index + 1}:</strong>
          <LatexTextRenderer text={message} />
        </li>
      {/each}
    </ul>
  </div>
  
  <div class="test-section">
    <h2>单独的LaTeX表达式测试</h2>
    <div class="latex-examples">
      <div class="example">
        <h3>求和表达式</h3>
        <LatexTextRenderer text={'这是一个求和：\\sum_{i=1}^{n} A_i X_i'} />
      </div>

      <div class="example">
        <h3>分数表达式</h3>
        <LatexTextRenderer text={'这是一个分数：\\frac{\\partial \\rho u}{\\partial t}'} />
      </div>

      <div class="example">
        <h3>复杂表达式</h3>
        <LatexTextRenderer text={'复杂的偏导数：\\frac{\\partial}{\\partial x}[\\mu(\\frac{\\partial v}{\\partial x} + \\frac{\\partial u}{\\partial y})]'} />
      </div>

      <div class="example">
        <h3>希腊字母</h3>
        <LatexTextRenderer text={'希腊字母：\\alpha, \\beta, \\gamma, \\delta, \\rho, \\mu, \\lambda, \\partial'} />
      </div>

      <div class="example">
        <h3>混合文本</h3>
        <LatexTextRenderer text={'在方程 \\frac{\\partial u}{\\partial t} = 0 中，变量 \\rho 表示密度，\\mu 表示粘度。'} />
      </div>
    </div>
  </div>
</div>

<style>
  .test-page {
    max-width: 1000px;
    margin: 0 auto;
    padding: 2rem;
    font-family: var(--font-family-body);
  }
  
  h1 {
    color: var(--text-header);
    margin-bottom: 2rem;
  }
  
  .test-section {
    margin-bottom: 3rem;
    padding: 1.5rem;
    border: 1px solid var(--border-primary);
    border-radius: var(--border-radius-card);
    background-color: var(--bg-secondary);
  }
  
  h2 {
    color: var(--text-header);
    margin-bottom: 1rem;
  }
  
  .issue-list {
    list-style: none;
    padding: 0;
  }
  
  .issue-item {
    margin-bottom: 1rem;
    padding: 1rem;
    background-color: var(--bg-main);
    border-radius: var(--border-radius-btn);
    border-left: 3px solid var(--status-error);
  }
  
  .latex-examples {
    display: grid;
    gap: 1.5rem;
  }
  
  .example {
    padding: 1rem;
    background-color: var(--bg-main);
    border-radius: var(--border-radius-btn);
    border: 1px solid var(--border-secondary);
  }
  
  .example h3 {
    margin-top: 0;
    margin-bottom: 0.5rem;
    color: var(--text-header);
    font-size: 1.1rem;
  }
</style>
