<script lang="ts">
  import Sidebar from '../components/Sidebar.svelte';
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/tauri';
  import { setLanguage } from '$lib/i18n';
  import Toast from '../components/Toast.svelte';
  import { recognitionStore } from '$lib/recognitionStore';
  import { historyStore } from '$lib/historyStore';
  import { page } from '$app/stores';

  let sidebarWidth = 220; // px
  let isResizingSidebar = false;
  let lastClientX = 0;

  function applySidebarWidth(width: number) {
    // 将宽度作为 CSS 变量作用在文档根节点，避免作用域差异
    document.documentElement.style.setProperty('--sidebar-width', `${width}px`);
  }

  function startSidebarResize(e: MouseEvent) {
    isResizingSidebar = true;
    lastClientX = e.clientX;
    window.addEventListener('mousemove', onSidebarMouseMove, { passive: true });
    window.addEventListener('mouseup', stopSidebarResize);
  }
  function onSidebarMouseMove(e: MouseEvent) {
    if (!isResizingSidebar) return;
    const delta = e.clientX - lastClientX;
    lastClientX = e.clientX;
    const min = 160; const max = Math.min(480, Math.floor(window.innerWidth * 0.4));
    sidebarWidth = Math.min(Math.max(sidebarWidth + delta, min), max);
    applySidebarWidth(sidebarWidth);
  }
  function stopSidebarResize() {
    if (!isResizingSidebar) return;
    isResizingSidebar = false;
    window.removeEventListener('mousemove', onSidebarMouseMove);
    window.removeEventListener('mouseup', stopSidebarResize);
    try { localStorage.setItem('sidebarWidth', String(sidebarWidth)); } catch {}
  }

  onMount(async () => {
    try {
      const cfg = await invoke('get_config');
      if (cfg && typeof cfg === 'object' && 'language' in (cfg)) {
        const lang = (((cfg as any).language) ?? 'en') as 'zh-CN' | 'en';
        setLanguage(lang);
      }
    } catch (e) {
      setLanguage('en');
    }

    try {
      const saved = localStorage.getItem('sidebarWidth');
      if (saved) sidebarWidth = Math.min(Math.max(parseInt(saved, 10), 160), 480);
    } catch {}
    applySidebarWidth(sidebarWidth);

    // 初始化历史数据store，在后台预加载数据
    try {
      await historyStore.initialize();
    } catch (error) {
      console.error('Failed to initialize history store:', error);
    }

    // 全局监听后端阶段事件：即使不在识别页也能更新结果与相位指示
    try {
      const { listen } = await import('@tauri-apps/api/event');
      await listen('recognition_progress', (e: any) => {
        const p = e?.payload as any;
        if (!p || typeof p !== 'object') return;
        const updPhase = (() => {
          try {
            const saved = localStorage.getItem('phaseState');
            return saved ? JSON.parse(saved) : { latex: 'pending', analysis: 'pending', verify: 'idle' };
          } catch { return { latex: 'pending', analysis: 'pending', verify: 'idle' }; }
        })();

        if (p.stage === 'latex' && p.latex) {
          recognitionStore.patch({ id: p.id, latex: p.latex, created_at: p.created_at ?? '', original_image: p.original_image ?? '', model_name: p.model_name, prompt_version: p.prompt_version });
          updPhase.latex = 'done';
          if (updPhase.analysis === 'idle') updPhase.analysis = 'pending';
          updPhase.verify = 'pending';
        } else if (p.stage === 'analysis' && p.analysis) {
          recognitionStore.patch({ title: p.title ?? '', analysis: p.analysis });
          updPhase.analysis = 'done';
        } else if (p.stage === 'confidence' && typeof p.confidence_score === 'number') {
          const patch: any = { confidence_score: p.confidence_score };
          if (p.verification) patch.verification = p.verification;
          recognitionStore.patch(patch);
          recognitionStore.setLoading(false);
          updPhase.verify = 'done';
        }
        // 记录上次实际使用的提示词版本（用于设置页显示参考）。
        (async () => {
          try {
            const lang = (p.language ?? p.lang ?? 'en') as string;
            const { invoke } = await import('@tauri-apps/api/tauri');
            const full = await invoke('get_full_prompts_with_language', { language: lang });
            localStorage.setItem('__lastUsedPrompts', JSON.stringify(full));
          } catch {}
        })();
        try { localStorage.setItem('phaseState', JSON.stringify(updPhase)); } catch {}
      });
    } catch {}
  });

  onDestroy(() => {
    window.removeEventListener('mousemove', onSidebarMouseMove);
    window.removeEventListener('mouseup', stopSidebarResize);
    // 清理历史数据store的自动刷新
    historyStore.destroy();
  });

  // 调试路由信息
  // 移除路由调试日志
</script>

{#if $page.route.id?.startsWith('/overlay') || $page.url.pathname.startsWith('/overlay')}
  <!-- Overlay页面：全屏显示，无侧边栏 -->
  <div class="overlay-layout">
    <slot />
  </div>
{:else}
  <!-- 普通页面：带侧边栏的布局 -->
  <div class="app-layout">
    <Sidebar />
    <button class="sidebar-resizer" type="button" aria-label="拖拽调整侧栏宽度" on:mousedown={startSidebarResize}></button>
    <main class="main-content">
      <slot />
    </main>
    <Toast />
  </div>
{/if}

<style>
  .overlay-layout {
    width: 100vw;
    height: 100vh;
    margin: 0;
    padding: 0;
    overflow: hidden;
    position: fixed;
    top: 0;
    left: 0;
    z-index: 9999;
  }

  .app-layout {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }

  /* 确保侧栏在布局中不被挤压变窄 */
  .app-layout :global(.sidebar) { flex: 0 0 var(--sidebar-width); min-width: var(--sidebar-width); }

  .main-content {
    flex-grow: 1;
    /* 统一所有页面左右留白，并略微减小间距 */
    padding: var(--spacing-lg) var(--spacing-xl);
    background-color: var(--bg-main);
    overflow-y: auto;
    scrollbar-gutter: stable both-edges;
  }

  /* 侧栏与主区域的可拖拽分割线 */
  .sidebar-resizer {
    flex: 0 0 6px;
    background: transparent;
    position: relative;
    cursor: ew-resize;
    border: none; padding: 0; margin: 0;
  }
  .sidebar-resizer::before {
    content: '';
    position: absolute; top: 0; bottom: 0; left: 2px; width: 1px;
    background: var(--border-primary);
  }
  .sidebar-resizer:hover::before { background: var(--primary); }
</style>