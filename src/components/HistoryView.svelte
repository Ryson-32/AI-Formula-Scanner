<script lang="ts">
  import { onMount, tick, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { invoke } from '@tauri-apps/api/tauri';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import FormulaRenderer from './FormulaRenderer.svelte';
  import VerificationReportRenderer from './VerificationReportRenderer.svelte';
  import { clipboard } from '@tauri-apps/api';
  import { showToast } from '$lib/toast';
  import { currentLang, translateNow } from '$lib/i18n';
  import { historyStore } from '$lib/historyStore';
  import { Star as StarIcon } from 'lucide-svelte';
  
  // 定义历史记录数据类型
  type HistoryItem = {
    id: string;
    latex: string;
    title: string;
    analysis: {
      summary: string;
      variables?: Array<{ symbol: string; description: string; unit?: string | null }>;
      terms?: Array<{ name: string; description: string }>;
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
    verification?: any;
    verification_report?: string;
  };
  
  let historyItems: HistoryItem[] = [];
  let filteredItems: HistoryItem[] = [];
  let searchQuery = '';
  let sortBy: 'date_desc' | 'date_asc' | 'title_asc' | 'title_desc' = 'date_desc';
  let isLoading = true;
  let errorMessage = '';
  let isDetailOpen = false;
  let selectedItem: HistoryItem | null = null;
  let lastScrollY = 0;
  let drawerWidth = 980; // 初始更宽，便于查看
  let isResizing = false;
  // 预览懒渲染
  let previewVisible: Record<string, boolean> = {};
  // 详情抽屉：原始图片折叠
  let isDrawerImageExpanded = true;
  let unsubscribeStore: (() => void) | null = null;
  let drawerImageSrc = ''; // 详情抽屉图片源
  let drawerImageLoading = false; // 详情抽屉图片加载中
  let drawerImageError = ''; // 详情抽屉图片加载错误
  // 详情抽屉：变量和术语折叠状态
  let isVariablesExpanded = true;
  let isTermsExpanded = true;
  // 回滚自适应 3x3 卡片的计算逻辑

  function assetUrlToFsPath(urlStr: string): string {
    try {
      // 兼容 asset:// 与 https://asset.localhost 两种形式
      if (/^asset:/.test(urlStr) || /^https?:\/\/asset\.localhost/i.test(urlStr)) {
        const u = new URL(urlStr.replace(/^asset:\/\//, 'https://'));
        let p = decodeURIComponent(u.pathname);
        // Windows: /C:\Users\... => 去掉前导斜杠并将 / 替换为 \
        if (/^\/[A-Za-z]:/.test(p)) p = p.slice(1);
        if (p.includes('/')) p = p.replace(/\//g, '\\');
        return p;
      }
    } catch {}
    return urlStr;
  }

  async function toImgSrc(path: string) {
    if (!path) return '';
    if (/^(data:|https?:|tauri:)/i.test(path)) return path;
    const fsPath = assetUrlToFsPath(path);
    try {
      // 兼容两种参数命名（image_path / imagePath），避免后端二进制版本不一致导致的参数解析错误
      return await invoke<string>('read_image_as_data_url', { image_path: fsPath, imagePath: fsPath });
    } catch (e) {
      return '';
    }
  }

  function inViewport(node: HTMLElement, params: { id: string }) {
    const id = params?.id;
    // 若环境不支持 IntersectionObserver，直接显示
    if (typeof IntersectionObserver === 'undefined') {
      if (id) previewVisible = { ...previewVisible, [id]: true };
      return { destroy() {} };
    }

    // 使用主内容滚动容器作为 root，确保首屏元素立即触发
    const scrollRoot = (typeof document !== 'undefined')
      ? (document.querySelector('main.main-content') as Element | null)
      : null;

    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting && id) {
          previewVisible = { ...previewVisible, [id]: true };
          observer.unobserve(entry.target as Element);
        }
      }
    }, { root: scrollRoot ?? null, rootMargin: '200px 0px', threshold: 0.01 });

    // 延迟一帧，避免初次挂载时布局尚未稳定导致未触发
    const t = setTimeout(() => observer.observe(node), 0);
    return {
      destroy() {
        clearTimeout(t);
        observer.disconnect();
      }
    };
  }
  
  // 导出的函数，以便在测试中调用
  function normalizeItem(raw: any): HistoryItem {
    return {
      id: raw.id,
      latex: raw.latex,
      title: raw.title,
      analysis: raw.analysis,
      is_favorite: raw.is_favorite ?? raw.isFavorite ?? false,
      created_at: raw.created_at ?? raw.createdAt ?? '',
      confidence_score: raw.confidence_score ?? raw.confidenceScore ?? 0,
      original_image: raw.original_image ?? raw.originalImage ?? '',
      model_name: raw.model_name ?? raw.modelName
    };
  }

  export async function loadHistory() {
    try {
      // 确保store已初始化，但不强制刷新（使用缓存数据）
      await historyStore.ensureLoaded();
      const mapped = (historyStore.value as any[]).map(normalizeItem);
      // 异步转换图片为 data URL，避免 asset 证书问题
      for (const it of mapped) {
        if (it.original_image && !/^data:/i.test(it.original_image)) {
          it.original_image = await toImgSrc(it.original_image);
        }
      }
      historyItems = mapped;
      applySort();
      isLoading = false;
    } catch (err) {
      const error = err as Error;
      errorMessage = `${translateNow('history.load_failed', $currentLang)}: ${error.message}`;
      isLoading = false;
    }
  }

  // 从store数据同步到本地状态
  function syncFromStore() {
    try {
      const mapped = (historyStore.value as any[]).map(normalizeItem);
      historyItems = mapped;
      handleSearch();
    } catch (error) {
    }
  }

  // 在组件加载时获取历史记录
  onMount(async () => {
    // 先检查store是否已有数据，如果有就立即显示
    if (historyStore.value && historyStore.value.length > 0) {
      syncFromStore();
      isLoading = false;
    }

    // 然后确保数据是最新的
    await loadHistory();
    // 首屏可见项兜底：确保初次渲染前两屏的项标记为可见，避免观察器未及时触发
    try {
      const firstCards = Array.from(document.querySelectorAll('.history-list .history-item')).slice(0, 8) as HTMLElement[];
      for (const el of firstCards) {
        const id = el?.querySelector('[use\\:inViewport]') ? undefined : undefined; // 占位，直接用列表顺序兜底
      }
      // 根据列表顺序直接打开前两行（假设每行 3-4 列，取前 8 个）
      for (const item of historyItems.slice(0, 8)) {
        previewVisible = { ...previewVisible, [item.id]: true };
      }
    } catch {}
    // 订阅全局缓存变化，自动刷新列表
    unsubscribeStore = historyStore.subscribe(() => {
      syncFromStore();
    });
    // 读取抽屉宽度
    try {
      const saved = localStorage.getItem('historyDrawerWidth');
      if (saved) drawerWidth = Math.min(Math.max(parseInt(saved, 10), 480), 1200);
    } catch {}
    // 读取原图折叠状态
    try {
      const flag = localStorage.getItem('originalImageExpanded');
      if (flag !== null) isDrawerImageExpanded = flag === 'true';
    } catch {}
    // 如果 URL 带有 ?id=，自动打开详情
    const idFromUrl = get(page).url.searchParams.get('id');
    if (idFromUrl) {
      const item = historyItems.find((h) => h.id === idFromUrl);
      if (item) openDetailFromUrl(item);
    }

    // 订阅路由变化，支持浏览器返回键关闭详情并恢复滚动位置
    page.subscribe(async ($page) => {
      const id = $page.url.searchParams.get('id');
      if (id) {
        const item = historyItems.find((h) => h.id === id);
        if (item) openDetailFromUrl(item);
      } else if (isDetailOpen) {
        // 关闭详情并恢复滚动
        isDetailOpen = false;
        selectedItem = null;
        document.body.style.overflow = '';
        await tick();
        window.scrollTo(0, lastScrollY);
      }
    });
  });

  
  // 搜索过滤功能
  function handleSearch() {
    if (!searchQuery.trim()) {
      filteredItems = [...historyItems];
    } else {
      const query = searchQuery.toLowerCase();
      filteredItems = historyItems.filter(item => 
        item.title.toLowerCase().includes(query) || 
        item.latex.toLowerCase().includes(query)
      );
    }
    applySort();
  }
  
  // 格式化日期
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

  function applySort() {
    const items = filteredItems.length ? filteredItems : [...historyItems];
    const sorted = [...items].sort((a, b) => {
      switch (sortBy) {
        case 'date_asc':
          return new Date(a.created_at).getTime() - new Date(b.created_at).getTime();
        case 'title_asc':
          return a.title.localeCompare(b.title);
        case 'title_desc':
          return b.title.localeCompare(a.title);
        case 'date_desc':
        default:
          return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
      }
    });
    filteredItems = sorted;
  }
  
  // 切换收藏状态
  async function toggleFavorite(item: HistoryItem) {
    try {
      await invoke('update_favorite_status', { id: item.id, isFavorite: !item.is_favorite });
      // 本地与全局同时更新
      item.is_favorite = !item.is_favorite;
      historyStore.updateItem(item.id, { is_favorite: item.is_favorite });
      historyItems = [...historyItems];
      handleSearch();
    } catch (err) {
      const msg = typeof err === 'string' 
        ? err 
        : (err && typeof err === 'object' && 'message' in err && typeof (err as any).message === 'string')
          ? (err as any).message
          : (() => { try { return JSON.stringify(err); } catch { return String(err); } })();
      console.error('Failed to update favorite status:', err);
      errorMessage = `更新收藏状态失败: ${msg}`;
    }
  }

  // 删除历史项
  async function deleteItem(item: HistoryItem) {
    try {
      await invoke('delete_history_item', { id: item.id });
      historyStore.remove(item.id);
      historyItems = historyItems.filter(h => h.id !== item.id);
      handleSearch();
    } catch (err) {
      const error = err as Error;
      console.error('Failed to delete history item:', error);
      errorMessage = `${translateNow('history.update_failed', $currentLang)}: ${error.message}`;
    }
  }
  
  // 查看详情
  async function openDetail(item: HistoryItem) {
    lastScrollY = window.scrollY;
    selectedItem = item;
    isDetailOpen = true;
    // 先不渲染原始 file:// 路径，等安全地址准备好再显示
    drawerImageSrc = '';
    drawerImageError = '';
    drawerImageLoading = !!item.original_image;
    try {
      if (item.original_image) {
        drawerImageSrc = await toImgSrc(item.original_image);
      }
    } catch (e) {
      drawerImageError = (e as any)?.message ?? String(e);
    } finally {
      drawerImageLoading = false;
    }
    document.body.style.overflow = 'hidden';
    goto(`/history?id=${encodeURIComponent(item.id)}`, { noScroll: true, keepFocus: true, replaceState: false });
  }

  async function openDetailFromUrl(item: HistoryItem) {
    if (!isDetailOpen) {
      lastScrollY = window.scrollY;
      document.body.style.overflow = 'hidden';
    }
    selectedItem = item;
    isDetailOpen = true;
    drawerImageSrc = '';
    drawerImageError = '';
    drawerImageLoading = !!item.original_image;
    try {
      if (item.original_image) {
        drawerImageSrc = await toImgSrc(item.original_image);
      }
    } catch (e) {
      drawerImageError = (e as any)?.message ?? String(e);
    } finally {
      drawerImageLoading = false;
    }
  }

  async function closeDetail() {
    isDetailOpen = false;
    selectedItem = null;
    drawerImageSrc = ''; // 清空图片源
    drawerImageError = '';
    drawerImageLoading = false;
    document.body.style.overflow = '';
    await goto('/history', { noScroll: true, keepFocus: true, replaceState: false });
    await tick();
    window.scrollTo(0, lastScrollY);
  }

  async function copySelectedLatex() {
    if (!selectedItem) return;
    try {
      await clipboard.writeText(selectedItem.latex || '');
      showToast(translateNow('recognition.copy_latex_success', $currentLang), 'success');
    } catch (e) {
      showToast(translateNow('recognition.copy_latex_failed', $currentLang), 'error');
    }
  }

  // 侧栏是否有内容（用于切换为单列布局）
  $: hasSideContent = !!(selectedItem && (selectedItem.verification || selectedItem.verification_report));

  function handleDrawerTitleInput(e: Event) {
    if (!selectedItem) return;
    const el = e.target as HTMLElement;
    const txt = (el && (el.innerText ?? el.textContent)) || '';
    selectedItem.title = txt;
  }

  async function handleDrawerTitleBlur(e: Event) {
    if (!selectedItem || !selectedItem.id) return;
    const el = e.target as HTMLElement;
    const newTitle = (el && (el.innerText ?? el.textContent)) || '';
    try {
      await invoke('update_history_title', { id: selectedItem.id, title: newTitle });
      historyStore.updateItem(selectedItem.id, { title: newTitle } as any);
    } catch {}
  }

  let lastClientX = 0;
  function startResize(e: MouseEvent) {
    isResizing = true;
    lastClientX = e.clientX;
    e.preventDefault();
    window.addEventListener('mousemove', onMouseMovePassive, { passive: true });
    window.addEventListener('mouseup', stopResize);
  }
  function onMouseMovePassive(e: MouseEvent) {
    if (!isResizing) return;
    const vw = window.innerWidth;
    const max = Math.floor(vw * 0.95);
    const delta = lastClientX - e.clientX;
    lastClientX = e.clientX;
    const proposed = drawerWidth + delta;
    drawerWidth = Math.min(Math.max(proposed, 480), Math.max(600, max));
  }
  function stopResize() {
    if (!isResizing) return;
    isResizing = false;
    window.removeEventListener('mousemove', onMouseMovePassive as any);
    window.removeEventListener('mouseup', stopResize);
    try { localStorage.setItem('historyDrawerWidth', String(drawerWidth)); } catch {}
  }

  onDestroy(() => {
    window.removeEventListener('mousemove', onMouseMovePassive as any);
    window.removeEventListener('mouseup', stopResize);
    if (unsubscribeStore) unsubscribeStore();
  });
</script>

<div class="history-view">

  {#if errorMessage}
    <div class="error-message">
      <p>{errorMessage}</p>
      <button on:click={() => errorMessage = ''}>{translateNow('common.close', $currentLang)}</button>
    </div>
  {/if}
  
  <div class="search-container">
    <input 
      type="text" 
      placeholder={translateNow('history.search_placeholder', $currentLang)} 
      bind:value={searchQuery} 
      on:input={handleSearch}
      class="search-input"
    />
    <select class="sort-select" bind:value={sortBy} on:change={() => { filteredItems = [...filteredItems]; applySort(); }}>
      <option value="date_desc">{translateNow('history.sort.date_desc', $currentLang)}</option>
      <option value="date_asc">{translateNow('history.sort.date_asc', $currentLang)}</option>
      <option value="title_asc">{translateNow('history.sort.title_asc', $currentLang)}</option>
      <option value="title_desc">{translateNow('history.sort.title_desc', $currentLang)}</option>
    </select>
  </div>
  
  {#if isLoading}
    <div class="loading-indicator">
      <p>{translateNow('history.loading', $currentLang)}</p>
    </div>
  {:else if filteredItems.length === 0}
    <div class="empty-state">
      {#if searchQuery.trim()}
        <p>{translateNow('history.no_results', $currentLang)}</p>
      {:else}
        <p>{translateNow('history.empty', $currentLang)}</p>
      {/if}
    </div>
  {:else}
    <div class="history-list">
      {#each filteredItems as item (item.id)}
        <div class="history-item">
          <div class="item-header">
            <button 
              class="favorite-button" 
              on:click={() => toggleFavorite(item)}
              class:active={item.is_favorite}
              title={item.is_favorite ? translateNow('common.favorite.remove', $currentLang) : translateNow('common.favorite.add', $currentLang)}
            >
              <span class="icon">
                <StarIcon size={18} fill={item.is_favorite ? 'currentColor' : 'none'} />
              </span>
            </button>
            <h3 class="item-title">{item.title}</h3>
          </div>
          
          <button type="button" class="item-preview" on:click={() => openDetail(item)} use:inViewport={{ id: item.id }}>
            {#if previewVisible[item.id]}
              <FormulaRenderer latex={item.latex} mode="preview" previewHeight={90} />
            {:else}
              <div class="preview-skeleton" aria-hidden="true"></div>
            {/if}
          </button>
          
          <div class="item-actions">
            <div class="left-actions">
              <button class="action-button danger" on:click={() => deleteItem(item)}>
                {translateNow('history.delete', $currentLang)}
              </button>
            </div>
            <div class="center-meta">
              {#if item.model_name}
                <span class="badge model-badge" title="{item.model_name}">{item.model_name}</span>
              {/if}
              {#if formatDate(item.created_at) !== '—'}
                <span class="time-text">{formatDate(item.created_at)}</span>
              {/if}
            </div>
            <div class="right-actions">
              <button class="action-button" on:click={() => openDetail(item)}>
                {translateNow('history.view_details', $currentLang)}
              </button>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if isDetailOpen && selectedItem}
  <button type="button" class="detail-overlay" on:click={closeDetail} aria-label="Close detail"></button>
  <div class="detail-drawer" role="dialog" aria-modal="true" style={`width:${drawerWidth}px`}>
    <div class="drawer-header">
      <h3 class="drawer-title" contenteditable
          on:input={handleDrawerTitleInput}
          on:blur={handleDrawerTitleBlur}>
        {selectedItem.title}
      </h3>
      <div class="meta" style="margin-left:auto">
        {#if selectedItem.model_name}
          <span class="badge model-badge" title="{selectedItem.model_name}">{selectedItem.model_name}</span>
        {/if}
        {#if formatDate(selectedItem.created_at) !== '—'}
          <span class="meta-sep">•</span>
          <span class="time-text">{formatDate(selectedItem.created_at)}</span>
        {/if}
      </div>
    </div>
    <div class="drawer-body">
      <div class="drawer-grid {hasSideContent ? '' : 'single'}">
        <div class="col main-col">
          <div class="drawer-section">
        <button class="section-toggle" on:click={() => { isDrawerImageExpanded = !isDrawerImageExpanded; try { localStorage.setItem('originalImageExpanded', String(isDrawerImageExpanded)); } catch {} }}>
          {translateNow('recognition.original_image', $currentLang)}
          <span class="toggle-icon">{isDrawerImageExpanded ? '▲' : '▼'}</span>
        </button>
        {#if isDrawerImageExpanded}
          <div class="drawer-image">
            {#if drawerImageLoading}
              <div class="preview-skeleton" aria-hidden="true" style="height:180px"></div>
            {:else if drawerImageSrc}
              <img src={drawerImageSrc} alt="preview" on:error={() => { drawerImageError = '图片加载失败'; drawerImageSrc = ''; }} />
            {:else if drawerImageError}
              <p style="color: var(--status-error)">{drawerImageError}</p>
            {/if}
          </div>
        {/if}
          </div>
          <div class="drawer-section">
            <div class="section-header-row">
              <h4>LaTeX</h4>
              <button class="mini-btn" on:click={copySelectedLatex}>{translateNow('recognition.copy_latex', $currentLang)}</button>
            </div>
            <div class="drawer-latex">
              <FormulaRenderer latex={selectedItem.latex} />
            </div>
          </div>

          <div class="drawer-section">
            <h4>{translateNow('recognition.analysis', $currentLang)}</h4>
            <p>{selectedItem.analysis.summary}</p>

            <!-- 变量部分 -->
            {#if selectedItem.analysis.variables && selectedItem.analysis.variables.length > 0}
              <div class="analysis-section">
                <button
                  class="section-header"
                  on:click={() => isVariablesExpanded = !isVariablesExpanded}
                  aria-expanded={isVariablesExpanded}
                >
                  <span class="section-title">{translateNow('main.result.variables', $currentLang)} ({selectedItem.analysis.variables.length})</span>
                  <span class="expand-icon">{isVariablesExpanded ? '▼' : '▶'}</span>
                </button>
                {#if isVariablesExpanded}
                  <div class="section-content">
                    <div class="variables-grid">
                      {#each selectedItem.analysis.variables as variable}
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
            {#if selectedItem.analysis.terms && selectedItem.analysis.terms.length > 0}
              <div class="analysis-section">
                <button
                  class="section-header"
                  on:click={() => isTermsExpanded = !isTermsExpanded}
                  aria-expanded={isTermsExpanded}
                >
                  <span class="section-title">{translateNow('main.result.terms', $currentLang)} ({selectedItem.analysis.terms.length})</span>
                  <span class="expand-icon">{isTermsExpanded ? '▼' : '▶'}</span>
                </button>
                {#if isTermsExpanded}
                  <div class="section-content">
                    <div class="terms-list">
                      {#each selectedItem.analysis.terms as term}
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

            {#if selectedItem.analysis.suggestions.length > 0}
              <h5>{translateNow('analysis.suggestions', $currentLang)}</h5>
              <ul class="drawer-suggestions">
                {#each selectedItem.analysis.suggestions as s}
                  <li class={s.type}>{s.message}</li>
                {/each}
              </ul>
            {/if}
            {#if selectedItem.confidence_score > 0}
              <div class="confidence-chip">{translateNow('recognition.confidence', $currentLang)}: <strong>{selectedItem.confidence_score}/100</strong></div>
            {/if}
          </div>
        </div>

        <div class="col side-col" class:hidden={!hasSideContent}>
          {#if selectedItem.verification}
            <div class="drawer-section">
              <h4>验证报告（beta）</h4>
              <VerificationReportRenderer verification={selectedItem.verification} />
            </div>
          {:else if selectedItem.verification_report}
            <div class="drawer-section">
              <h4>验证报告（beta）</h4>
              <div class="drawer-plain-report">{selectedItem.verification_report}</div>
            </div>
          {/if}
        </div>
      </div>
    </div>
    <button class="drawer-resizer" type="button" aria-label="拖拽调整宽度" on:mousedown={startResize}></button>
  </div>
{/if}

<style>
  .history-view { width: 100%; padding: 0; }

  /* .view-title removed to avoid duplicated page titles */

  .error-message {
    background-color: var(--status-error-bg);
    color: var(--status-error);
    padding: var(--spacing-base);
    border-radius: var(--border-radius-btn);
    margin-bottom: var(--spacing-base);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .error-message button {
    background-color: transparent;
    color: var(--status-error);
    padding: var(--spacing-xs) var(--spacing-sm);
    font-size: var(--font-size-small);
  }

  .search-container {
    margin-bottom: var(--spacing-lg);
    display: grid;
    grid-template-columns: 1fr 240px;
    gap: var(--spacing-base);
  }

  .search-input {
    width: 100%;
    padding: var(--input-padding-y) var(--input-padding-x);
    border: var(--input-border-width) solid var(--border-primary);
    border-radius: var(--border-radius-btn);
    font-size: var(--font-size-body);
    transition: border-color 0.2s, box-shadow 0.2s;
  }

  .search-input:focus {
    border-color: var(--focus);
    outline: none;
    box-shadow: 0 0 0 2px rgba(102, 126, 234, 0.2);
  }

  .loading-indicator, .empty-state {
    text-align: center;
    padding: var(--spacing-xl);
    background-color: var(--bg-secondary);
    border-radius: var(--border-radius-card);
    color: var(--text-muted);
    font-size: var(--font-size-body);
  }

  .history-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: var(--spacing-lg);
  }

  .history-item {
    background-color: var(--bg-main);
    border: var(--card-border);
    border-radius: var(--border-radius-card);
    box-shadow: var(--card-shadow);
    transition: box-shadow 0.3s ease, transform 0.3s ease;
    display: flex;
    flex-direction: column;
  }

  .history-item:hover {
    box-shadow: var(--card-shadow-hover);
    transform: translateY(-2px);
  }

  .item-header {
    display: flex;
    align-items: center;
    padding: 10px var(--spacing-base);
    border-bottom: 1px solid var(--border-primary);
  }

  .favorite-button {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: var(--text-muted);
    margin-right: var(--spacing-md);
    padding: 0;
    transition: color 0.2s, transform 0.2s;
  }

  .favorite-button:hover {
    transform: scale(1.1);
  }

  .favorite-button.active {
    color: var(--status-warning);
  }

  .item-title {
    font-size: var(--font-size-card-title);
    font-weight: var(--font-weight-semibold);
    color: var(--text-default);
    flex-grow: 1;
    margin: 0;
  }

  .item-date {
    font-size: var(--font-size-small);
    color: var(--text-muted);
  }
  .badge {
    display: inline-flex;
    align-items: center;
    padding: 1px 6px; /* 更紧凑 */
    border-radius: 999px;
    font-size: 11px; /* 更小字号 */
    line-height: 1.2;
    white-space: nowrap;
  }
  .model-badge {
    margin-left: auto;
    background: rgba(99,102,241,0.1);
    color: var(--primary);
    border: 1px solid var(--primary);
  }

  .item-preview {
    padding: 10px var(--spacing-base);
    cursor: pointer;
    flex-grow: 1;
    background: none;
    border: none;
    transition: background-color 0.2s;
  }

  .item-preview:hover {
    background-color: var(--bg-secondary);
  }

  .item-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px var(--spacing-base); /* 压缩上下留白 */
    border-top: 1px solid var(--border-primary);
  }

  .left-actions, .right-actions {
    display: flex;
    gap: 6px; /* 更紧凑 */
  }
  .center-meta { display:flex; flex-direction: column; align-items:center; justify-content:center; gap:2px; }
  .time-text { color: var(--text-muted); font-size: 11px; }
  .time-dot { width: 4px; height: 4px; background: var(--border-secondary, #e5e7eb); border-radius: 50%; }

  .action-button {
    padding: 4px 8px; /* 更紧凑按钮 */
    background-color: transparent;
    color: var(--primary);
    border: 1px solid var(--primary);
    border-radius: var(--border-radius-btn);
    cursor: pointer;
    font-size: 12px;
    font-weight: var(--font-weight-medium);
    transition: background-color 0.2s, color 0.2s, transform 0.1s;
  }

  .action-button:hover {
    background-color: var(--primary);
    color: var(--text-inverse);
    transform: translateY(-1px);
  }

  .action-button.danger {
    border-color: #ef4444;
    color: #ef4444;
  }
  .action-button.danger:hover {
    background-color: #ef4444;
    color: #fff;
  }

  /* Drawer styles */
  .detail-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.35);
    z-index: 999;
  }
  .detail-drawer {
    position: fixed;
    top: 0; right: 0; bottom: 0;
    background: var(--bg-main);
    box-shadow: -8px 0 24px rgba(0,0,0,0.18);
    z-index: 1000;
    display: flex;
    flex-direction: column;
    animation: slideIn .2s ease-out;
  }
  @keyframes slideIn { from { transform: translateX(16px); opacity: .8 } to { transform: translateX(0); opacity: 1 } }
  .drawer-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-md) var(--spacing-lg);
    border-bottom: 1px solid var(--border-primary);
    background: var(--bg-secondary);
  }
  .back-button {
    border: 1px solid var(--border-primary);
    background: var(--bg-main);
    border-radius: var(--border-radius-btn);
    padding: 6px 10px;
    cursor: pointer;
  }
  .drawer-title { margin: 0; font-size: var(--font-size-h3); }
  .drawer-body {
    padding: var(--spacing-lg);
    overflow: auto;
  }
  .drawer-grid {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: var(--spacing-lg);
    align-items: start;
  }
  .drawer-grid.single { grid-template-columns: 1fr; }
  .drawer-grid .col { display: flex; flex-direction: column; gap: var(--spacing-lg); }
  .drawer-grid .main-col { min-width: 0; }
  .drawer-grid .side-col { min-width: 0; }
  .drawer-resizer { position: absolute; top:0; left:-6px; bottom:0; width: 6px; cursor: ew-resize; background: transparent; border: none; padding: 0; }
  .drawer-resizer::before { content:''; position:absolute; top:0; bottom:0; left:2px; width:1px; background: var(--border-primary); }
  .drawer-resizer:hover::before { background: var(--primary); }
  /* removed .resizer-hint (no label needed) */

  .section-toggle {
    width: 100%; text-align: left; padding: var(--spacing-md) var(--spacing-base);
    background-color: var(--bg-secondary); border: 1px solid var(--border-primary);
    border-radius: var(--border-radius-btn); cursor: pointer; display: flex;
    justify-content: space-between; align-items: center; font-weight: var(--font-weight-medium);
    color: var(--text-default);
  }
  .section-toggle:hover { background-color: var(--bg-hover); }

  .preview-skeleton { height: 120px; border-radius: var(--border-radius-card); background: linear-gradient(90deg, var(--bg-secondary) 25%, var(--bg-hover) 37%, var(--bg-secondary) 63%); background-size: 400% 100%; animation: shimmer 1.4s ease infinite; }
  @keyframes shimmer { 0% { background-position: 100% 0 } 100% { background-position: -100% 0 } }
  .drawer-image { text-align: center; }
  .drawer-image img { max-width: 100%; max-height: 220px; object-fit: contain; }
  .drawer-section h4 { margin: 0 0 var(--spacing-sm); }
  .section-header-row { display:flex; align-items:center; justify-content: space-between; gap: var(--spacing-sm); }
  .drawer-section h5 { margin: var(--spacing-sm) 0 var(--spacing-xs); font-weight: var(--font-weight-semibold); color: var(--text-muted); }
  .mini-btn { padding: 4px 8px; font-size: 12px; border: 1px solid var(--border-primary); background: var(--bg-main); color: var(--text-default); border-radius: var(--border-radius-btn); cursor: pointer; }
  .mini-btn:hover { background: var(--bg-secondary); }
  .drawer-latex { padding: var(--spacing-base); border: 1px solid var(--border-primary); border-radius: var(--border-radius-card); }
  .drawer-suggestions { padding-left: 1.2rem; margin: 0; }
  .drawer-suggestions li.warning { color: var(--status-warning); }
  .drawer-suggestions li.info { color: var(--status-info); }
  .drawer-suggestions li.error { color: var(--status-error); }
  /* 分析部分样式 */
  .analysis-section {
    margin-bottom: var(--spacing-lg);
  }

  .section-header {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--spacing-sm) var(--spacing-base);
    background: var(--bg-secondary);
    border: 1px solid var(--border-primary);
    border-radius: var(--border-radius-btn);
    cursor: pointer;
    transition: all 0.2s ease;
    font-size: var(--font-size-body);
    color: var(--text-default);
  }

  .section-header:hover {
    background: var(--bg-hover);
    border-color: var(--primary);
  }

  .section-title {
    font-weight: var(--font-weight-medium);
    color: var(--text-default);
  }

  .expand-icon {
    color: var(--text-muted);
    font-size: 0.9em;
    transition: transform 0.2s ease;
  }

  .section-content {
    padding: var(--spacing-base);
    border: 1px solid var(--border-secondary);
    border-top: none;
    border-radius: 0 0 var(--border-radius-btn) var(--border-radius-btn);
    background: var(--bg-main);
  }

  /* 变量样式 */
  .variables-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: var(--spacing-base);
  }

  .variable-item {
    padding: var(--spacing-base);
    border: 1px solid var(--border-secondary);
    border-radius: var(--border-radius-btn);
    background: var(--bg-secondary);
    transition: all 0.2s ease;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    min-height: 120px;
  }

  .variable-item:hover {
    border-color: var(--primary);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    transform: translateY(-1px);
  }

  .variable-description {
    color: var(--text-default);
    font-weight: var(--font-weight-medium);
    line-height: var(--line-height-normal);
    font-size: clamp(0.85rem, 2vw, 1rem);
    flex: 1;
    word-wrap: break-word;
    overflow-wrap: break-word;
  }

  .variable-details {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    flex-wrap: wrap;
    min-height: 60px;
  }

  .variable-symbol {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    min-width: 60px;
    max-width: 120px;
    padding: 4px 8px;
    background: var(--bg-main);
    border-radius: var(--border-radius-btn);
    border: 1px solid var(--border-light);
    overflow: hidden;
  }

  .variable-symbol :global(.formula-renderer) {
    border: none;
    background: transparent;
    box-shadow: none;
    padding: 2px;
    min-height: 48px;
    max-height: 48px;
  }

  .unit-label {
    color: var(--text-muted);
    font-size: var(--font-size-small);
    font-weight: var(--font-weight-medium);
    flex-shrink: 0;
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
  .drawer-plain-report { white-space: pre-wrap; line-height: 1.6; color: var(--text-default); border: 1px solid var(--border-secondary); background: var(--bg-secondary); padding: var(--spacing-sm); border-radius: var(--border-radius-btn); }
  .drawer-suggestions li.error { color: var(--status-error); }
  .confidence-chip { display:inline-block; margin-top: var(--spacing-sm); padding: 4px 8px; border:1px solid var(--border-secondary); border-radius:999px; font-size:12px; color: var(--text-muted); background: var(--bg-secondary); }
</style>