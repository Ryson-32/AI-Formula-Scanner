import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { tick } from 'svelte';
import FavoritesView from '../../src/components/FavoritesView.svelte';
import { invoke } from '@tauri-apps/api/tauri';
import { goto } from '$app/navigation';
import { historyStore } from '$lib/historyStore';

// Mock tauri and navigation modules
vi.mock('@tauri-apps/api/tauri');
vi.mock('$app/navigation');

describe('FavoritesView.svelte', () => {
  // Get a typed reference to the mocked functions
  const mockedInvoke = vi.mocked(invoke);
  const mockedGoto = vi.mocked(goto);

  beforeEach(() => {
    // Reset mocks before each test
    mockedInvoke.mockClear();
    mockedGoto.mockClear();
    historyStore.replace([]);
  });

  it('renders title at page level; component omits duplicate', async () => {
    mockedInvoke.mockResolvedValue([]);
    const { component } = render(FavoritesView);
    // @ts-ignore
    await component.loadFavorites();
    await tick();
    expect(screen.queryByText('收藏夹')).not.toBeInTheDocument();
  });

  it('renders empty state when there are no favorites', async () => {
    historyStore.replace([
      { id: '1', title: 'Not a favorite', is_favorite: false, latex: '', created_at: '', analysis: { summary: '', suggestions: [] }, confidence_score: 0, original_image: '' } as any
    ]);
    const { component } = render(FavoritesView);
    // @ts-ignore
    await component.loadFavorites();
    await tick();
    expect(screen.getByText('收藏夹为空')).toBeInTheDocument();
  });

  it('shows an error message if fetching fails', async () => {
    const errorMessage = 'Fetch failed';
    const ensureSpy = vi.spyOn(historyStore, 'ensureLoaded').mockRejectedValueOnce(new Error(errorMessage));
    const { component } = render(FavoritesView);
    // @ts-ignore
    await component.loadFavorites();
    await tick();
    ensureSpy.mockRestore();
    const errorElement = await screen.findByText(`加载收藏夹失败: ${errorMessage}`);
    expect(errorElement).toBeInTheDocument();
  });

  it('renders a list of favorite items only', async () => {
    const mockHistory = [
      { id: '1', title: 'Favorite Item 1', is_favorite: true, latex: 'a^2', created_at: new Date().toISOString(), analysis: { summary: '', suggestions: [] }, confidence_score: 0, original_image: '' },
      { id: '2', title: 'Not a Favorite', is_favorite: false, latex: 'b^2', created_at: new Date().toISOString(), analysis: { summary: '', suggestions: [] }, confidence_score: 0, original_image: '' },
      { id: '3', title: 'Favorite Item 2', is_favorite: true, latex: 'c^2', created_at: new Date().toISOString(), analysis: { summary: '', suggestions: [] }, confidence_score: 0, original_image: '' },
    ];
    historyStore.replace(mockHistory as any);
    const { component } = render(FavoritesView);
    // @ts-ignore
    await component.loadFavorites();
    await tick();

    expect(screen.getByText('Favorite Item 1')).toBeInTheDocument();
    expect(screen.queryByText('Not a Favorite')).not.toBeInTheDocument();
    expect(screen.getByText('Favorite Item 2')).toBeInTheDocument();
  });

  it('filters the favorites list based on search query', async () => {
    const mockHistory = [
        { id: '1', title: 'Favorite Apple', is_favorite: true, latex: 'a^2', created_at: new Date().toISOString(), analysis: { summary: '', suggestions: [] }, confidence_score: 0, original_image: '' },
        { id: '2', title: 'Favorite Banana', is_favorite: true, latex: 'b^2', created_at: new Date().toISOString(), analysis: { summary: '', suggestions: [] }, confidence_score: 0, original_image: '' },
    ];
    historyStore.replace(mockHistory as any);
    const { component } = render(FavoritesView);
    // @ts-ignore
    await component.loadFavorites();
    await tick();

    const searchInput = screen.getByPlaceholderText('搜索公式标题或LaTeX内容...');
    await fireEvent.input(searchInput, { target: { value: 'Apple' } });

    expect(screen.getByText('Favorite Apple')).toBeInTheDocument();
    expect(screen.queryByText('Favorite Banana')).not.toBeInTheDocument();
  });
});


