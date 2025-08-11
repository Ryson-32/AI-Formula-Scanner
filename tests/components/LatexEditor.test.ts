import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { tick } from 'svelte';
import LatexEditor from '../../src/components/LatexEditor.svelte';

describe('LatexEditor.svelte', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('renders textarea with placeholder', async () => {
    render(LatexEditor);
    await tick();
    
    const textarea = screen.getByPlaceholderText('在此输入或编辑LaTeX代码...');
    expect(textarea).toBeInTheDocument();
    expect((textarea as HTMLTextAreaElement).tagName).toBe('TEXTAREA');
  });

  it('displays initial latex value', async () => {
    const initialLatex = 'x^2 + y^2 = z^2';
    render(LatexEditor, { props: { latex: initialLatex } });
    await tick();
    
    const textarea = screen.getByDisplayValue(initialLatex);
    expect(textarea).toBeInTheDocument();
  });

  // 已移除控制按钮，相关测试删除或跳过
  
  it('dispatches update event on textarea input', async () => {
    const { component } = render(LatexEditor);
    const updateHandler = vi.fn();
    component.$on('update', updateHandler);
    
    const textarea = screen.getByPlaceholderText('在此输入或编辑LaTeX代码...');
    await fireEvent.input(textarea, { target: { value: 'a + b = c' } });
    
    expect(updateHandler).toHaveBeenCalledWith(
      expect.objectContaining({
        detail: { latex: 'a + b = c' }
      })
    );
  });

  it('disables textarea when disabled prop is true', async () => {
    render(LatexEditor, { props: { disabled: true } });
    await tick();
    
    const textarea = screen.getByPlaceholderText('在此输入或编辑LaTeX代码...');
    expect(textarea).toBeDisabled();
  });

  it('enables textarea when disabled prop is false', async () => {
    render(LatexEditor, { props: { disabled: false } });
    await tick();
    
    const textarea = screen.getByPlaceholderText('在此输入或编辑LaTeX代码...');
    expect(textarea).not.toBeDisabled();
  });

  it('updates latex prop when component receives new value', async () => {
    const { component } = render(LatexEditor, { props: { latex: 'initial' } });
    await tick();
    
    let textarea = screen.getByDisplayValue('initial');
    expect(textarea).toBeInTheDocument();
    
    // Update the prop
    component.$set({ latex: 'updated' });
    await tick();
    
    textarea = screen.getByDisplayValue('updated');
    expect(textarea).toBeInTheDocument();
  });

  it('applies correct CSS classes', async () => {
    render(LatexEditor);
    await tick();
    
    const editor = document.querySelector('.latex-editor');
    const textarea = document.querySelector('.editor-textarea');
    
    expect(editor).toBeInTheDocument();
    expect(textarea).toBeInTheDocument();
  });

  it('maintains textarea attributes', async () => {
    render(LatexEditor);
    await tick();
    
    const textarea = screen.getByPlaceholderText('在此输入或编辑LaTeX代码...');
    expect(textarea).toHaveAttribute('rows', '4');
    expect(textarea).toHaveClass('editor-textarea');
  });

  it('handles empty latex value', async () => {
    render(LatexEditor, { props: { latex: '' } });
    await tick();
    
    const textarea = screen.getByPlaceholderText('在此输入或编辑LaTeX代码...');
    expect(textarea).toHaveValue('');
  });

  it('handles special characters in latex input', async () => {
    const { component } = render(LatexEditor);
    const updateHandler = vi.fn();
    component.$on('update', updateHandler);
    
    const specialLatex = '\\alpha + \\beta = \\gamma';
    const textarea = screen.getByPlaceholderText('在此输入或编辑LaTeX代码...');
    
    await fireEvent.input(textarea, { target: { value: specialLatex } });
    
    expect(updateHandler).toHaveBeenCalledWith(
      expect.objectContaining({
        detail: { latex: specialLatex }
      })
    );
  });
});


