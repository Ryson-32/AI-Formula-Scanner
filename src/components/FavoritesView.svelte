<script lang="ts">
  import { onMount, tick, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/tauri';
  import FormulaRenderer from './FormulaRenderer.svelte';
  import LatexTextRenderer from './LatexTextRenderer.svelte';
  import { currentLang, translateNow } from '$lib/i18n';
  import { historyStore } from '$lib/historyStore';
  import { Star as StarIcon } from 'lucide-svelte';

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

  async function toImgSrc(path: string) {
    if (!path) return '';
    if (/^(data:|https?:|tauri:)/i.test(path)) return path;
    const fsPath = assetUrlToFsPath(path);
    try {
      return await invoke<string>('read_image_as_data_url', { image_path: fsPath, imagePath: fsPath });
    } catch (e) {
      
      return '';
    }
  }
  
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
    verification_report?: string;
  };
  
  let historyItems: HistoryItem[] = [];
  let favoriteItems: HistoryItem[] = [];
  let filteredItems: HistoryItem[] = [];
  let searchQuery = '';
  let isLoading = true;
  let errorMessage = '';
  let sortBy: 'date_desc' | 'date_asc' | 'title_asc' | 'title_desc' = 'date_desc';

  // 详情抽屉状态（在收藏夹内直接查看，不跳转历史页）
  let isDetailOpen = false;
  let selectedItem: HistoryItem | null = null;
  let isDrawerImageExpanded = true;
  let lastScrollY = 0;
  let drawerWidth = 980;
  let isResizing = false;
  let lastClientX = 0;
  // 详情抽屉：变量和术语折叠状态
  let isVariablesExpanded = true;
  let isTermsExpanded = true;

  // 与后端 camelCase 对齐的兼容映射
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
  
  // 导出的函数，以便在测试中调用
  export async function loadFavorites() {
    try {
      // 确保store已初始化，但不强制刷新（使用缓存数据）
      await historyStore.ensureLoaded();
      const mapped = (historyStore.value as any[]).map(normalizeItem);
      for (const it of mapped) {
        if (it.original_image && !/^data:/i.test(it.original_image)) {
          it.original_image = await toImgSrc(it.original_image);
        }
      }
      historyItems = mapped;
      favoriteItems = historyItems.filter(item => item.is_favorite);
      favoriteItems.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());
      filteredItems = [...favoriteItems];
      applySort();
      isLoading = false;
    } catch (err) {
      const error = err as Error;
      console.error('Failed to load favorites:', error);
      errorMessage = `${translateNow('favorites.load_failed', $currentLang)}: ${error.message}`;
      isLoading = false;
    }
  }

  // 从store数据同步到本地状态
  function syncFromStore() {
    try {
      const mapped = (historyStore.value as any[]).map(normalizeItem);
      historyItems = mapped;
      favoriteItems = historyItems.filter(item => item.is_favorite);
      favoriteItems.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());
      filteredItems = [...favoriteItems];
      applySort();
    } catch (error) {
      console.error('Failed to sync favorites from store:', error);
    }
  }

  // 在组件加载时获取历史记录并筛选收藏项
  onMount(async () => {
    // 先检查store是否已有数据，如果有就立即显示
    if (historyStore.value && historyStore.value.length > 0) {
      syncFromStore();
      isLoading = false;
    }

    // 然后确保数据是最新的
    await loadFavorites();

    // 订阅store变化，自动同步数据
    const unsubscribe = historyStore.subscribe(() => {
      syncFromStore();
    });

    // 组件销毁时取消订阅
    onDestroy(() => {
      unsubscribe();
    });

    try {
      const flag = localStorage.getItem('originalImageExpanded');
      if (flag !== null) isDrawerImageExpanded = flag === 'true';
    } catch {}
  });

  
  
  // 搜索过滤功能
  function handleSearch() {
    if (!searchQuery.trim()) {
      filteredItems = [...favoriteItems];
    } else {
      const query = searchQuery.toLowerCase();
      filteredItems = favoriteItems.filter(item => 
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
    const items = filteredItems.length ? filteredItems : [...favoriteItems];
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
      
      // 更新本地状态
      item.is_favorite = !item.is_favorite;
      historyStore.updateItem(item.id, { is_favorite: item.is_favorite });
      
      // 如果取消收藏，从收藏夹中移除
      if (!item.is_favorite) {
        favoriteItems = favoriteItems.filter(i => i.id !== item.id);
        filteredItems = filteredItems.filter(i => i.id !== item.id);
      }
      applySort();
      
      // 更新原始历史记录中的状态
      const index = historyItems.findIndex(i => i.id === item.id);
      if (index !== -1) {
        historyItems[index].is_favorite = item.is_favorite;
      }
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
      favoriteItems = favoriteItems.filter(i => i.id !== item.id);
      filteredItems = filteredItems.filter(i => i.id !== item.id);
      historyItems = historyItems.filter(i => i.id !== item.id);
      applySort();
    } catch (err) {
      const error = err as Error;
      console.error('Failed to delete history item:', error);
      errorMessage = `${translateNow('favorites.update_failed', $currentLang)}: ${error.message}`;
    }
  }
  
  // 查看详情
  function viewDetails(item: HistoryItem) {
    openDetail(item);
  }

  function openDetail(item: HistoryItem) {
    lastScrollY = window.scrollY;
    selectedItem = item;
    isDetailOpen = true;
    document.body.style.overflow = 'hidden';
  }

  async function closeDetail() {
    isDetailOpen = false;
    selectedItem = null;
    document.body.style.overflow = '';
    await tick();
    window.scrollTo(0, lastScrollY);
  }

  function startResize(e: MouseEvent) {
    isResizing = true;
    lastClientX = e.clientX;
    e.preventDefault();
    window.addEventListener('mousemove', onMouseMovePassive as any, { passive: true } as any);
    window.addEventListener('mouseup', stopResize as any);
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
    window.removeEventListener('mouseup', stopResize as any);
  }

  onDestroy(() => {
    window.removeEventListener('mousemove', onMouseMovePassive as any);
    window.removeEventListener('mouseup', stopResize as any);
  });
</script>

<div class="favorites-view">

  {#if errorMessage}
    <div class="error-message">
      <p>{errorMessage}</p>
      <button on:click={() => errorMessage = ''}>{translateNow('common.close', $currentLang)}</button>
    </div>
  {/if}
  
  <div class="search-container">
    <input 
      type="text" 
      placeholder={translateNow('favorites.search_placeholder', $currentLang)} 
      bind:value={searchQuery} 
      on:input={handleSearch}
      class="search-input"
    />
    <select class="sort-select" bind:value={sortBy} on:change={() => { filteredItems = [...filteredItems]; applySort(); }}>
      <option value="date_desc">按时间（新→旧）</option>
      <option value="date_asc">按时间（旧→新）</option>
      <option value="title_asc">按名称（A→Z）</option>
      <option value="title_desc">按名称（Z→A）</option>
    </select>
  </div>
  
  {#if isLoading}
    <div class="loading-indicator">
      <p>{translateNow('favorites.loading', $currentLang)}</p>
    </div>
  {:else if filteredItems.length === 0}
    <div class="empty-state">
      {#if searchQuery.trim()}
        <p>{translateNow('favorites.no_results', $currentLang)}</p>
      {:else}
        <p>{translateNow('favorites.empty', $currentLang)}</p>
      {/if}
    </div>
  {:else}
    <div class="favorites-list">
      {#each filteredItems as item (item.id)}
        <div class="favorite-item">
          <div class="item-header">
            <button 
              class="favorite-button active" 
              on:click={() => toggleFavorite(item)}
              title={translateNow('common.favorite.remove', $currentLang)}
            >
              <StarIcon size={18} fill="currentColor" />
            </button>
            <h3 class="item-title">{item.title}</h3>
            
          </div>
          
          <button type="button" class="item-preview" on:click={() => viewDetails(item)}>
            <FormulaRenderer latex={item.latex} mode="preview" previewHeight={90} />
          </button>
          
           <div class="item-actions">
             <div class="left-actions">
               <button class="action-button danger" on:click={() => deleteItem(item)}>
                 删除
               </button>
             </div>
              <div class="center-meta">
                {#if item.model_name}
                  <span class="badge model-badge" title="{item.model_name}">{item.model_name}</span>
                {/if}
                <span class="time-text">{formatDate(item.created_at)}</span>
              </div>
              <div class="right-actions">
                <button class="action-button" on:click={() => viewDetails(item)}>
                  {translateNow('favorites.view_details', $currentLang)}
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
      <button class="back-button" on:click={closeDetail}>←</button>
      <h3 class="drawer-title">{selectedItem.title}</h3>
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
      <div class="drawer-section">
        <button class="section-toggle" on:click={() => { isDrawerImageExpanded = !isDrawerImageExpanded; try { localStorage.setItem('originalImageExpanded', String(isDrawerImageExpanded)); } catch {} }}>
          {translateNow('recognition.original_image', $currentLang)}
          <span class="toggle-icon">{isDrawerImageExpanded ? '▲' : '▼'}</span>
        </button>
        {#if isDrawerImageExpanded}
          <div class="drawer-image">
            {#if selectedItem.original_image}
              <img src={selectedItem.original_image} alt="preview" />
            {/if}
          </div>
        {/if}
      </div>
      <div class="drawer-section">
        <h4>LaTeX</h4>
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
          <ul class="drawer-suggestions">
            {#each selectedItem.analysis.suggestions as s}
              <li class={s.type}>{s.message}</li>
            {/each}
          </ul>
        {/if}
      </div>
      {#if selectedItem.confidence_score > 0}
        <div class="drawer-section">
          <h4>{translateNow('recognition.confidence', $currentLang)}</h4>
          <p><strong>{selectedItem.confidence_score}/100</strong></p>
        </div>
      {/if}
      {#if selectedItem.verification_report}
        <div class="drawer-section">
          <h4>核查报告</h4>
          <div class="verification-report">
            <LatexTextRenderer text={selectedItem.verification_report} />
          </div>
        </div>
      {/if}
    </div>
    <button class="drawer-resizer" type="button" aria-label="拖拽调整宽度" on:mousedown={startResize}></button>
  </div>
{/if}

<style>
  .favorites-view { width: 100%; padding: 0; }

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

  .favorites-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: var(--spacing-lg);
  }

  .favorite-item {
    background-color: var(--bg-main);
    border: var(--card-border);
    border-radius: var(--border-radius-card);
    box-shadow: var(--card-shadow);
    transition: box-shadow 0.3s ease, transform 0.3s ease;
    display: flex;
    flex-direction: column;
  }

  .favorite-item:hover {
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

  /* removed .item-date (unused after redesign) */
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
  .left-actions, .right-actions { display: flex; gap: 6px; }
  .center-meta { display:flex; flex-direction:column; align-items:center; justify-content:center; gap:2px; }
  .time-text { color: var(--text-muted); font-size: 11px; }
  /* removed .time-dot (unused) */

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
  .action-button.danger { border-color: #ef4444; color: #ef4444; }
  .action-button.danger:hover { background-color: #ef4444; color: #fff; }

  /* Drawer styles (inline detail view) */
  .detail-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.35); z-index: 999; }
  .detail-drawer { position: fixed; top:0; right:0; bottom:0; background: var(--bg-main); box-shadow: -8px 0 24px rgba(0,0,0,0.18); z-index: 1000; display:flex; flex-direction:column; animation: slideIn .2s ease-out; }
  @keyframes slideIn { from { transform: translateX(16px); opacity:.8 } to { transform: translateX(0); opacity:1 } }
  .drawer-header { display:flex; align-items:center; gap: var(--spacing-sm); padding: var(--spacing-md) var(--spacing-lg); border-bottom: 1px solid var(--border-primary); background: var(--bg-secondary); }
  .back-button { border: 1px solid var(--border-primary); background: var(--bg-main); border-radius: var(--border-radius-btn); padding: 6px 10px; cursor: pointer; }
  .drawer-title { margin: 0; font-size: var(--font-size-h3); }
  .drawer-body { padding: var(--spacing-lg); overflow: auto; display:flex; flex-direction:column; gap: var(--spacing-lg); }
  .drawer-resizer { position: absolute; top:0; left:-6px; bottom:0; width:6px; cursor: ew-resize; background: transparent; border:none; padding:0; }
  .drawer-resizer::before { content:''; position:absolute; top:0; bottom:0; left:2px; width:1px; background: var(--border-primary); }
  .drawer-resizer:hover::before { background: var(--primary); }
  .section-toggle { width: 100%; text-align: left; padding: var(--spacing-md) var(--spacing-base); background-color: var(--bg-secondary); border: 1px solid var(--border-primary); border-radius: var(--border-radius-btn); cursor: pointer; display: flex; justify-content: space-between; align-items: center; font-weight: var(--font-weight-medium); color: var(--text-default); }
  .section-toggle:hover { background-color: var(--bg-hover); }
  .drawer-image { text-align:center; }
  .drawer-image img { max-width: 100%; max-height: 220px; object-fit: contain; }
  .drawer-section h4 { margin: 0 0 var(--spacing-sm); }
  .drawer-latex { padding: var(--spacing-base); border: 1px solid var(--border-primary); border-radius: var(--border-radius-card); }
  .drawer-suggestions { padding-left: 1.2rem; margin: 0; }
  .drawer-suggestions li.warning { color: var(--status-warning); }
  .drawer-suggestions li.info { color: var(--status-info); }
  .drawer-suggestions li.error { color: var(--status-error); }
  .drawer-suggestions li.error { color: var(--status-error); }
  .meta-sep { margin: 0 6px; color: var(--border-secondary, #e5e7eb); }

  .verification-report {
    background-color: var(--bg-hover);
    padding: var(--spacing-sm);
    border-radius: var(--border-radius-btn);
    border-left: 3px solid var(--accent-primary);
    font-size: 0.9rem;
    line-height: 1.4;
    color: var(--text-default);
  }

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
</style>