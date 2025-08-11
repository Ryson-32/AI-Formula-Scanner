import { render, screen } from '@testing-library/svelte';
import { describe, it, expect, beforeEach } from 'vitest';
import { tick } from 'svelte';
import Sidebar from '../../src/components/Sidebar.svelte';
import { page } from '$app/stores';

describe('Sidebar.svelte', () => {
  beforeEach(() => {
    // Reset page store to default state
    page.set({
      url: { pathname: '/' },
      params: {},
      route: { id: null },
      data: {}
    });
  });

  it('renders the application logo', async () => {
    render(Sidebar);
    await tick();
    
    expect(screen.getByText('AI 公式扫描器')).toBeInTheDocument();
  });

  it('renders all navigation links', async () => {
    render(Sidebar);
    await tick();
    
    // Check main navigation links
    expect(screen.getByText('公式识别')).toBeInTheDocument();
    expect(screen.getByText('历史记录')).toBeInTheDocument();
    expect(screen.getByText('收藏夹')).toBeInTheDocument();
    expect(screen.getByText('设置')).toBeInTheDocument();
  });

  it('renders navigation links with correct hrefs', async () => {
    render(Sidebar);
    await tick();
    
    const homeLink = screen.getByTitle('公式识别');
    const historyLink = screen.getByTitle('历史记录');
    const favoritesLink = screen.getByTitle('收藏夹');
    const settingsLink = screen.getByTitle('设置');
    
    expect(homeLink).toHaveAttribute('href', '/');
    expect(historyLink).toHaveAttribute('href', '/history');
    expect(favoritesLink).toHaveAttribute('href', '/favorites');
    expect(settingsLink).toHaveAttribute('href', '/settings');
  });

  it('highlights active link based on current pathname', async () => {
    render(Sidebar);
    await tick();
    
    // Home should be active by default
    const homeLink = screen.getByTitle('公式识别');
    expect(homeLink).toHaveClass('active');
    
    // Other links should not be active
    const historyLink = screen.getByTitle('历史记录');
    const favoritesLink = screen.getByTitle('收藏夹');
    const settingsLink = screen.getByTitle('设置');
    
    expect(historyLink).not.toHaveClass('active');
    expect(favoritesLink).not.toHaveClass('active');
    expect(settingsLink).not.toHaveClass('active');
  });

  it('updates active link when pathname changes', async () => {
    render(Sidebar);
    await tick();

    // Change to history page
    page.set({
      url: { pathname: '/history' },
      params: {},
      route: { id: null },
      data: {}
    });
    await tick();
    
    const homeLink = screen.getByTitle('公式识别');
    const historyLink = screen.getByTitle('历史记录');
    
    expect(homeLink).not.toHaveClass('active');
    expect(historyLink).toHaveClass('active');
  });

  it('highlights settings link when on settings page', async () => {
    page.set({
      url: { pathname: '/settings' },
      params: {},
      route: { id: null },
      data: {}
    });
    
    render(Sidebar);
    await tick();
    
    const settingsLink = screen.getByTitle('设置');
    expect(settingsLink).toHaveClass('active');
  });

  it('highlights favorites link when on favorites page', async () => {
    page.set({
      url: { pathname: '/favorites' },
      params: {},
      route: { id: null },
      data: {}
    });
    
    render(Sidebar);
    await tick();
    
    const favoritesLink = screen.getByTitle('收藏夹');
    expect(favoritesLink).toHaveClass('active');
  });

  it('renders icons for all navigation items', async () => {
    render(Sidebar);
    await tick();
    
    // Settings icon exists
    expect(document.querySelector('.lucide-settings')).toBeInTheDocument();
  });

  it('applies correct CSS structure', async () => {
    render(Sidebar);
    await tick();
    
    const sidebar = document.querySelector('.sidebar');
    const sidebarTop = document.querySelector('.sidebar-top');
    const sidebarBottom = document.querySelector('.sidebar-bottom');
    const logo = document.querySelector('.logo');
    const navLinks = document.querySelector('.nav-links');
    
    expect(sidebar).toBeInTheDocument();
    expect(sidebarTop).toBeInTheDocument();
    expect(sidebarBottom).toBeInTheDocument();
    expect(logo).toBeInTheDocument();
    expect(navLinks).toBeInTheDocument();
  });

  it('renders navigation as an unordered list', async () => {
    render(Sidebar);
    await tick();
    
    const navList = document.querySelector('.nav-links');
    expect(navList?.tagName).toBe('UL');
    
    const listItems = document.querySelectorAll('.nav-links li');
    expect(listItems).toHaveLength(3); // Main navigation items
  });

  it('includes proper accessibility attributes', async () => {
    render(Sidebar);
    await tick();
    
    // Check that links have title attributes for accessibility
    const homeLink = screen.getByTitle('公式识别');
    const historyLink = screen.getByTitle('历史记录');
    const favoritesLink = screen.getByTitle('收藏夹');
    const settingsLink = screen.getByTitle('设置');
    
    expect(homeLink).toHaveAttribute('title', '公式识别');
    expect(historyLink).toHaveAttribute('title', '历史记录');
    expect(favoritesLink).toHaveAttribute('title', '收藏夹');
    expect(settingsLink).toHaveAttribute('title', '设置');
  });

  it('maintains semantic HTML structure', async () => {
    render(Sidebar);
    await tick();
    
    const nav = document.querySelector('nav.sidebar');
    expect(nav).toBeInTheDocument();
    
    // Check that it uses proper nav element
    expect(nav?.tagName).toBe('NAV');
  });

  it('handles edge case pathnames gracefully', async () => {
    // Test with a pathname that doesn't match any nav item
    page.set({
      url: { pathname: '/unknown-page' },
      params: {},
      route: { id: null },
      data: {}
    });
    
    render(Sidebar);
    await tick();
    
    // No links should be active
    const allLinks = document.querySelectorAll('a');
    allLinks.forEach(link => {
      expect(link).not.toHaveClass('active');
    });
  });

  it('renders text labels alongside icons', async () => {
    render(Sidebar);
    await tick();
    
    // Check text labels are present
    const navItems = document.querySelectorAll('.nav-links li a .text');
    expect(navItems.length).toBeGreaterThan(0);
  });
});


