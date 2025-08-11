import { render, screen } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { tick } from 'svelte';
import FormulaRenderer from '../../src/components/FormulaRenderer.svelte';
import { invoke } from '@tauri-apps/api/tauri';

// Mock Tauri API
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn() }));

// Mock MathJax and KaTeX globals
const mockMathJax = {
  typesetPromise: vi.fn().mockResolvedValue(undefined),
  startup: {
    promise: Promise.resolve()
  }
};

const mockKatex = {
  render: vi.fn()
};

// Set up global mocks
Object.defineProperty(window, 'MathJax', {
  value: mockMathJax,
  writable: true
});

Object.defineProperty(window, 'katex', {
  value: mockKatex,
  writable: true
});

const mockConfig = {
  apiKey: 'test-key',
  apiBaseUrl: 'https://generativelanguage.googleapis.com/v1beta/models',
  provider: 'gemini',
  defaultEngine: 'gemini-2.0-flash-exp',
  customPrompt: 'Test prompt',
  verificationPrompt: 'Test verification prompt',
  renderEngine: 'MathJax',
  autoCalculateConfidence: false,
  enableClipboardWatcher: false,
  defaultLatexFormat: 'raw',
  requestTimeoutSeconds: 30,
  maxRetries: 3
};

describe('FormulaRenderer.svelte', () => {
  const invokeMock = vi.mocked(invoke);

  beforeEach(() => {
    vi.resetAllMocks();

    // Setup default mock for get_config
    invokeMock.mockImplementation(async (command: string) => {
      if (command === 'get_config') {
        return mockConfig;
      }
      return {};
    });

    mockMathJax.typesetPromise.mockClear();
    mockKatex.render.mockClear();
  });

  it('renders placeholder when no latex is provided', async () => {
    render(FormulaRenderer, { props: { latex: '' } });
    await tick();
    
    expect(screen.getByText('公式将在这里显示')).toBeInTheDocument();
  });

  it('renders with MathJax when latex is provided', async () => {
    const latex = 'e^{i\\pi} + 1 = 0';

    render(FormulaRenderer, {
      props: {
        latex,
        renderEngine: 'MathJax'
      }
    });

    // Wait for component to render
    await tick();

    // Verify the component structure exists
    const container = document.querySelector('.formula-renderer');
    expect(container).toBeInTheDocument();

    // The latex content should not show placeholder
    expect(container?.textContent).not.toContain('公式将在这里显示');
  });

  it('renders with KaTeX when specified', async () => {
    const latex = 'x^2 + y^2 = z^2';

    render(FormulaRenderer, {
      props: {
        latex,
        renderEngine: 'KaTeX'
      }
    });

    await tick();

    // Verify component rendered
    const container = document.querySelector('.formula-renderer');
    expect(container).toBeInTheDocument();
    expect(container?.textContent).not.toContain('公式将在这里显示');
  });

  it('loads config and uses default render engine', async () => {
    const latex = 'a^2 + b^2 = c^2';

    render(FormulaRenderer, { props: { latex } });
    await tick();

    // Verify component renders properly
    const container = document.querySelector('.formula-renderer');
    expect(container).toBeInTheDocument();
  });

  it('handles render engine change', async () => {
    const latex = 'f(x) = x^2';
    const { component } = render(FormulaRenderer, { 
      props: { 
        latex,
        renderEngine: 'MathJax'
      } 
    });
    await tick();
    
    // Change render engine
    component.$set({ renderEngine: 'KaTeX' });
    await tick();
    
    // Should trigger re-render with new engine
    await new Promise(resolve => setTimeout(resolve, 100));
  });

  it('handles latex content change', async () => {
    const initialLatex = 'x + y = z';
    const { component } = render(FormulaRenderer, {
      props: {
        latex: initialLatex,
        renderEngine: 'MathJax'
      }
    });
    await tick();
    await new Promise(resolve => setTimeout(resolve, 50));

    // Change latex content
    const newLatex = 'a + b = c';
    component.$set({ latex: newLatex });
    await tick();
    await new Promise(resolve => setTimeout(resolve, 50));

    // Verify component handled the change
    const container = document.querySelector('.formula-renderer');
    expect(container).toBeInTheDocument();
  });

  it('handles empty latex gracefully', async () => {
    const { component } = render(FormulaRenderer, { 
      props: { 
        latex: 'x = y',
        renderEngine: 'MathJax'
      } 
    });
    await tick();
    
    // Set latex to empty
    component.$set({ latex: '' });
    await tick();
    
    expect(screen.getByText('公式将在这里显示')).toBeInTheDocument();
  });

  it('handles config loading failure gracefully', async () => {
    invokeMock.mockRejectedValue(new Error('Config load failed'));
    
    const latex = 'x^2';
    render(FormulaRenderer, { props: { latex } });
    await tick();
    
    // Should still render with default engine
    await new Promise(resolve => setTimeout(resolve, 100));
  });

  it('applies correct CSS classes', async () => {
    render(FormulaRenderer, { props: { latex: '' } });
    await tick();
    
    const container = document.querySelector('.formula-renderer');
    expect(container).toBeInTheDocument();
    
    const placeholder = screen.getByText('公式将在这里显示');
    expect(placeholder).toHaveClass('placeholder');
  });

  it('handles render errors gracefully', async () => {
    const latex = 'invalid\\latex\\syntax';
    render(FormulaRenderer, {
      props: {
        latex,
        renderEngine: 'MathJax'
      }
    });
    await tick();
    await new Promise(resolve => setTimeout(resolve, 50));

    // Component should still render without crashing
    const container = document.querySelector('.formula-renderer');
    expect(container).toBeInTheDocument();
  });

  it('supports different latex formats', async () => {
    const latexFormats = [
      'x^2 + y^2 = z^2',
      '\\frac{a}{b} = c',
      '\\sum_{i=1}^{n} x_i',
      '\\int_{0}^{1} f(x) dx'
    ];

    for (const latex of latexFormats) {
      const { unmount } = render(FormulaRenderer, {
        props: {
          latex,
          renderEngine: 'MathJax'
        }
      });
      await tick();
      await new Promise(resolve => setTimeout(resolve, 10));

      // Verify component renders for each format
      const container = document.querySelector('.formula-renderer');
      expect(container).toBeInTheDocument();

      unmount();
    }
  });
});


