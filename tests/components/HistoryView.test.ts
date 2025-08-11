import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { tick } from 'svelte';
import HistoryView from '../../src/components/HistoryView.svelte';
import { invoke } from '@tauri-apps/api/tauri';
import { goto } from '$app/navigation';
import { historyStore } from '$lib/historyStore';

// Mock tauri and navigation modules
vi.mock('@tauri-apps/api/tauri');
vi.mock('$app/navigation');

describe('HistoryView.svelte', () => {
  // Get a typed reference to the mocked functions
  const mockedInvoke = vi.mocked(invoke);
  const mockedGoto = vi.mocked(goto);

  beforeEach(() => {
    // Reset mocks before each test
    mockedInvoke.mockClear();
    mockedGoto.mockClear();
    historyStore.replace([]);
  });

  it('renders the title is handled by page, component omits duplicate', async () => {
    mockedInvoke.mockResolvedValue([]);
    const { component } = render(HistoryView);
    await component.loadHistory();
    await tick();
    // HistoryView 不再渲染标题，标题由路由页面渲染
    expect(screen.queryByText('历史记录')).not.toBeInTheDocument();
  });

  it('renders empty state when there is no history', async () => {
    historyStore.replace([]);
    const { component } = render(HistoryView);
    await component.loadHistory();
    await tick();
    expect(screen.getByText('历史记录为空')).toBeInTheDocument();
  });

  it('shows an error message if fetching history fails', async () => {
    const errorMessage = 'Network Error';
    const ensureSpy = vi.spyOn(historyStore, 'ensureLoaded').mockRejectedValueOnce(new Error(errorMessage));
    const { component } = render(HistoryView);
    await component.loadHistory();
    await tick();
    ensureSpy.mockRestore();
    const errorElement = await screen.findByText(`加载历史记录失败: ${errorMessage}`);
    expect(errorElement).toBeInTheDocument();
  });

  it('renders a list of history items', async () => {
    const mockHistory = [
      { id: '1', title: 'First Item', latex: 'a^2', created_at: new Date().toISOString(), is_favorite: false, analysis: { summary: '', suggestions: [] }, confidence_score: 0, original_image: '' },
      { id: '2', title: 'Second Item', latex: 'b^2', created_at: new Date().toISOString(), is_favorite: true, analysis: { summary: '', suggestions: [] }, confidence_score: 0, original_image: '' },
    ];
    historyStore.replace(mockHistory as any);
    const { component } = render(HistoryView);
    await component.loadHistory();
    await tick();

    expect(screen.getByText('First Item')).toBeInTheDocument();
    expect(screen.getByText('Second Item')).toBeInTheDocument();
  });

  it('filters the history list based on search query', async () => {
    const mockHistory = [
      { id: '1', title: 'Apple Pie', latex: 'a^2', created_at: new Date().toISOString(), is_favorite: false, analysis: { summary: '', suggestions: [] }, confidence_score: 0, original_image: '' },
      { id: '2', title: 'Banana Split', latex: 'b^2', created_at: new Date().toISOString(), is_favorite: true, analysis: { summary: '', suggestions: [] }, confidence_score: 0, original_image: '' },
      { id: '3', title: 'Apple Crumble', latex: 'c^2', created_at: new Date().toISOString(), is_favorite: false, analysis: { summary: '', suggestions: [] }, confidence_score: 0, original_image: '' },
    ];
    historyStore.replace(mockHistory as any);
    const { component } = render(HistoryView);
    await component.loadHistory();
    await tick();

    const searchInput = screen.getByPlaceholderText('搜索公式标题或LaTeX内容...');
    await fireEvent.input(searchInput, { target: { value: 'Apple' } });

    expect(screen.getByText('Apple Pie')).toBeInTheDocument();
    expect(screen.queryByText('Banana Split')).not.toBeInTheDocument();
    expect(screen.getByText('Apple Crumble')).toBeInTheDocument();
  });
});


