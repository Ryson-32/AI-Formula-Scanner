import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import LatexTextRenderer from '../../src/components/LatexTextRenderer.svelte';

// Mock FormulaRenderer component
vi.mock('../../src/components/FormulaRenderer.svelte', () => ({
  default: vi.fn(() => ({
    $$: {
      on_mount: [],
      on_destroy: [],
      before_update: [],
      after_update: [],
      context: new Map(),
      callbacks: new Map(),
      dirty: [-1],
      bound: {},
      update: vi.fn(),
      props: {}
    }
  }))
}));

describe('LatexTextRenderer.svelte', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('renders plain text without LaTeX', async () => {
    const text = '这是一段普通文本，没有数学公式。';
    
    render(LatexTextRenderer, {
      props: { text }
    });
    
    await tick();
    
    const container = document.querySelector('.latex-text-renderer');
    expect(container).toBeInTheDocument();
    expect(container?.textContent).toContain('这是一段普通文本');
  });

  it('renders text with LaTeX commands', async () => {
    const text = '在第一个方程的第一个项 \\frac{\\partial \\rho u}{\\partial t} 在图像中为';
    
    render(LatexTextRenderer, {
      props: { text }
    });
    
    await tick();
    
    const container = document.querySelector('.latex-text-renderer');
    expect(container).toBeInTheDocument();
    
    // Should contain both text and LaTeX parts
    const textParts = container?.querySelectorAll('.text-part');
    const latexParts = container?.querySelectorAll('.inline-latex');
    
    expect(textParts?.length).toBeGreaterThan(0);
    expect(latexParts?.length).toBeGreaterThan(0);
  });

  it('handles multiple LaTeX commands in one text', async () => {
    const text = '方程包含 \\partial 和 \\rho 以及 \\frac{a}{b} 等符号';
    
    render(LatexTextRenderer, {
      props: { text }
    });
    
    await tick();
    
    const container = document.querySelector('.latex-text-renderer');
    expect(container).toBeInTheDocument();
    
    const latexParts = container?.querySelectorAll('.inline-latex');
    expect(latexParts?.length).toBeGreaterThan(1);
  });

  it('handles empty text', async () => {
    render(LatexTextRenderer, {
      props: { text: '' }
    });
    
    await tick();
    
    const container = document.querySelector('.latex-text-renderer');
    expect(container).toBeInTheDocument();
    expect(container?.textContent).toBe('');
  });

  it('handles text with only LaTeX', async () => {
    const text = '\\frac{\\partial \\rho u}{\\partial t}';
    
    render(LatexTextRenderer, {
      props: { text }
    });
    
    await tick();
    
    const container = document.querySelector('.latex-text-renderer');
    expect(container).toBeInTheDocument();
    
    const latexParts = container?.querySelectorAll('.inline-latex');
    expect(latexParts?.length).toBeGreaterThan(0);
  });

  it('uses correct render engine', async () => {
    const text = '包含 \\alpha 符号的文本';
    
    render(LatexTextRenderer, {
      props: { 
        text,
        renderEngine: 'KaTeX'
      }
    });
    
    await tick();
    
    const container = document.querySelector('.latex-text-renderer');
    expect(container).toBeInTheDocument();
  });

  it('applies correct CSS classes', async () => {
    const text = '文本 \\beta 更多文本';
    
    render(LatexTextRenderer, {
      props: { text }
    });
    
    await tick();
    
    const container = document.querySelector('.latex-text-renderer');
    const textParts = container?.querySelectorAll('.text-part');
    const latexParts = container?.querySelectorAll('.inline-latex');
    
    expect(container).toHaveClass('latex-text-renderer');
    expect(textParts?.length).toBeGreaterThan(0);
    expect(latexParts?.length).toBeGreaterThan(0);
    
    textParts?.forEach(part => {
      expect(part).toHaveClass('text-part');
    });
    
    latexParts?.forEach(part => {
      expect(part).toHaveClass('inline-latex');
    });
  });
});


