import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/tauri';

export type HistoryItem = {
  id: string;
  latex: string;
  title: string;
  analysis: {
    summary: string;
    variables?: Array<{ symbol: string; description: string; unit?: string | null }>;
    terms?: Array<{ name: string; description: string }>;
    suggestions: Array<{ type: string; message: string }>;
  };
  is_favorite: boolean;
  created_at: string;
  confidence_score: number;
  original_image: string;
  model_name?: string;
  verification_report?: string;
};

let loaded = false;
let isLoading = false;
let autoRefreshInterval: number | null = null;

const { subscribe, set, update } = writable<HistoryItem[]>([]);

async function fetchHistory(): Promise<HistoryItem[]> {
  const list = await invoke<any[]>('get_history');
  // 保持原始文件路径，展示阶段再转换，以避免 asset:// 加载限制
  return (list as any[]) as unknown as HistoryItem[];
}

// 自动检查并刷新数据
async function autoRefresh(): Promise<void> {
  if (isLoading) return;
  try {
    isLoading = true;
    const list = await fetchHistory();
    set(list);
    loaded = true;
  } catch (error) {
    console.error('Auto refresh history failed:', error);
  } finally {
    isLoading = false;
  }
}

export const historyStore = {
  subscribe,
  // 初始化：启动自动刷新机制
  async initialize(): Promise<void> {
    if (loaded) return;

    // 立即加载一次数据
    await autoRefresh();

    // 启动定期检查机制（每3秒检查一次文件变化）
    if (autoRefreshInterval === null) {
      autoRefreshInterval = window.setInterval(autoRefresh, 3000);
    }
  },
  // 确保只加载一次（保持向后兼容）
  async ensureLoaded(): Promise<void> {
    if (loaded) return;
    await autoRefresh();
  },
  // 强制刷新（例如外部修改或调试用）
  async refresh(): Promise<void> {
    if (isLoading) return;
    isLoading = true;
    try {
      const list = await fetchHistory();
      set(list);
      loaded = true;
    } finally {
      isLoading = false;
    }
  },
  // 本地更新：切换收藏状态
  updateItem(id: string, partial: Partial<HistoryItem>) {
    update(items => Array.isArray(items) ? items.map(it => (it.id === id ? { ...it, ...partial } : it)) : items);
  },
  // 本地删除
  remove(id: string) {
    update(items => Array.isArray(items) ? items.filter(it => it.id !== id) : []);
  },
  // 本地新增
  add(item: HistoryItem) {
    update(items => (Array.isArray(items) ? [item, ...items] : [item]));
  },
  // 直接替换（如批量编辑后同步）
  replace(items: HistoryItem[]) {
    set(items);
    loaded = true;
  },
  // 读取当前值
  get value() {
    return get({ subscribe });
  },
  // 停止自动刷新
  destroy() {
    if (autoRefreshInterval !== null) {
      clearInterval(autoRefreshInterval);
      autoRefreshInterval = null;
    }
  }
};



