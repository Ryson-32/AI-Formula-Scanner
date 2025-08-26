/**
 * 统一的数学渲染引擎管理器
 * 避免多个组件同时加载MathJax/KaTeX导致的冲突
 */

type MathEngine = 'MathJax' | 'KaTeX';

interface MathEngineManager {
  isLoading: boolean;
  isLoaded: boolean;
  loadPromise: Promise<void> | null;
}

const engines: Record<MathEngine, MathEngineManager> = {
  MathJax: { isLoading: false, isLoaded: false, loadPromise: null },
  KaTeX: { isLoading: false, isLoaded: false, loadPromise: null }
};

// MathJax配置，避免Package错误
const mathJaxConfig = {
  tex: {
    inlineMath: [['$', '$'], ['\\(', '\\)']],
    displayMath: [['$$', '$$'], ['\\[', '\\]']],
    processEscapes: true,
    processEnvironments: true
  },
  svg: {
    fontCache: 'global'
  },
  startup: {
    ready: () => {
      console.log('MathJax is ready');
      // 确保MathJax完全初始化后再标记为已加载
      engines.MathJax.isLoaded = true;
      engines.MathJax.isLoading = false;
    }
  }
};

/**
 * 加载MathJax引擎
 */
async function loadMathJax(): Promise<void> {
  const engine = engines.MathJax;

  if (engine.isLoaded && (window as any).MathJax) {
    return Promise.resolve();
  }

  if (engine.isLoading && engine.loadPromise) {
    return engine.loadPromise;
  }

  engine.isLoading = true;
  engine.loadPromise = new Promise((resolve, reject) => {
    try {
      // 设置MathJax配置 - 必须在脚本加载前设置
      (window as any).MathJax = {
        ...mathJaxConfig,
        startup: {
          ready: () => {
            console.log('MathJax is ready');
            // 调用默认的ready函数
            (window as any).MathJax.startup.defaultReady();
            // 标记为已加载
            engine.isLoaded = true;
            engine.isLoading = false;
            resolve();
          }
        }
      };

      const script = document.createElement('script');
      script.src = 'https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-svg.js';
      script.async = true;

      script.onerror = (error) => {
        engine.isLoading = false;
        console.error('Failed to load MathJax:', error);
        reject(new Error('Failed to load MathJax'));
      };

      document.head.appendChild(script);
    } catch (error) {
      engine.isLoading = false;
      reject(error);
    }
  });

  return engine.loadPromise;
}

/**
 * 加载KaTeX引擎
 */
async function loadKaTeX(): Promise<void> {
  const engine = engines.KaTeX;
  
  if (engine.isLoaded && (window as any).katex) {
    return Promise.resolve();
  }
  
  if (engine.isLoading && engine.loadPromise) {
    return engine.loadPromise;
  }
  
  engine.isLoading = true;
  engine.loadPromise = new Promise((resolve, reject) => {
    try {
      // 加载KaTeX CSS
      const link = document.createElement('link');
      link.rel = 'stylesheet';
      link.href = 'https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/katex.min.css';
      document.head.appendChild(link);
      
      // 加载KaTeX JS
      const script = document.createElement('script');
      script.src = 'https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/katex.min.js';
      script.async = true;
      
      script.onload = () => {
        engine.isLoaded = true;
        engine.isLoading = false;
        console.log('KaTeX loaded successfully');
        resolve();
      };
      
      script.onerror = (error) => {
        engine.isLoading = false;
        console.error('Failed to load KaTeX:', error);
        reject(new Error('Failed to load KaTeX'));
      };
      
      document.head.appendChild(script);
    } catch (error) {
      engine.isLoading = false;
      reject(error);
    }
  });
  
  return engine.loadPromise;
}

/**
 * 加载指定的数学渲染引擎
 */
export async function loadMathEngine(engine: MathEngine): Promise<void> {
  switch (engine) {
    case 'MathJax':
      return loadMathJax();
    case 'KaTeX':
      return loadKaTeX();
    default:
      throw new Error(`Unknown math engine: ${engine}`);
  }
}

/**
 * 检查引擎是否可用
 */
export function isMathEngineAvailable(engine: MathEngine): boolean {
  switch (engine) {
    case 'MathJax':
      return engines.MathJax.isLoaded && !!(window as any).MathJax && !!(window as any).MathJax.tex2svg;
    case 'KaTeX':
      return engines.KaTeX.isLoaded && !!(window as any).katex;
    default:
      return false;
  }
}

/**
 * 获取引擎加载状态
 */
export function getMathEngineStatus(engine: MathEngine): MathEngineManager {
  return engines[engine];
}


