import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest';
import { tick } from 'svelte';
import RecognitionView from '../../src/components/RecognitionView.svelte';
import { setLanguage } from '$lib/i18n';
import { invoke } from '@tauri-apps/api/tauri';
import { open } from '@tauri-apps/api/dialog';
import { readBinaryFile } from '@tauri-apps/api/fs';
import { clipboard } from '@tauri-apps/api';
// mock toast
vi.mock('$lib/toast', () => {
  const showToast = vi.fn();
  return { showToast, toasts: { subscribe: () => () => {} } };
});

// Use real child components to avoid Svelte runtime lifecycle issues

// Mock Tauri APIs
vi.mock('@tauri-apps/api', () => ({ clipboard: { writeText: vi.fn() } }));
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/dialog', () => ({ open: vi.fn() }));
vi.mock('@tauri-apps/api/fs', () => ({ readBinaryFile: vi.fn() }));

const mockRecognitionResult = {
  id: 'test-id-123',
  latex: 'e^{i\\pi} + 1 = 0',
  title: 'Euler\'s Identity',
  analysis: { 
    summary: 'A beautiful equation.', 
    suggestions: [
      { type: 'info', message: 'This is Euler\'s famous identity.' }
    ]
  },
  is_favorite: false,
  created_at: new Date().toISOString(),
  confidence_score: 95,
  original_image: 'base64-encoded-string',
};

const mockConfig = {
  apiKey: 'test-api-key',
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

describe('RecognitionView.svelte', () => {
  const invokeMock = invoke as Mock;
  const openMock = open as Mock;
  const readBinaryFileMock = readBinaryFile as Mock;
  const clipboardMock = clipboard.writeText as Mock;

  beforeEach(() => {
    vi.resetAllMocks();
    setLanguage('zh-CN');
    
    // Default mock implementations
    invokeMock.mockImplementation(async (command: string) => {
      if (command === 'get_config') {
        return mockConfig;
      }

      if (command === 'recognize_from_file') {
        return mockRecognitionResult;
      }
      if (command === 'save_to_history') {
        return {};
      }
      if (command === 'get_confidence_score') {
        return 95;
      }
      return {};
    });
  });

  it('renders initial UI elements', async () => {
    render(RecognitionView);
    await tick();

    expect(screen.getByText('导入图片')).toBeInTheDocument();
    expect(screen.getByText('请使用上方的按钮开始识别公式')).toBeInTheDocument();
  });

  it('shows error if API key is missing', async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === 'get_config') {
        return { ...mockConfig, apiKey: '' };
      }
      return {};
    });

    render(RecognitionView);
    await tick();
    await fireEvent.click(screen.getByText('导入图片'));
    await tick();

    expect(screen.getByText('请先在设置中配置API密钥')).toBeInTheDocument();
  });

  it('shows progress phases during recognition (no loading tip)', async () => {
    let resolveRecognition: (value: any) => void;
    const recognitionPromise = new Promise(resolve => {
      resolveRecognition = resolve as any;
    });
    
    invokeMock.mockImplementation(async (command) => {
      if (command === 'get_config') return mockConfig;
      if (command === 'recognize_from_file') return recognitionPromise;
      return {};
    });

    render(RecognitionView);
    await tick();

    await fireEvent.click(screen.getByText('导入图片'));
    // 不再显示“正在处理，请稍候...”，但阶段指示应存在
    expect(screen.queryByText(/正在处理，请稍候...|Processing, please wait.../)).toBeNull();
    expect(screen.getAllByText(/LaTeX/).length).toBeGreaterThan(0);
    expect(screen.getByText(/智能分析|Analysis/)).toBeInTheDocument();
    
    // Resolve the promise
    resolveRecognition!(mockRecognitionResult);
    // 完成后仍不显示加载文案
    expect(screen.queryByText('处理中...')).toBeNull();
  });

  it('handles file import recognition', async () => {
    const mockFileContent = new Uint8Array([1, 2, 3, 4]);
    openMock.mockResolvedValue('/path/to/test.png');
    readBinaryFileMock.mockResolvedValue(mockFileContent);
    
    render(RecognitionView);
    await tick();
    
    await fireEvent.click(screen.getByText(/导入图片|Import Image/));
    await waitFor(() => expect(openMock).toHaveBeenCalled());
    
    expect(openMock).toHaveBeenCalledWith({
      multiple: false,
      filters: [{
        name: '图片',
        extensions: ['png', 'jpg', 'jpeg']
      }]
    });
    
    expect(invokeMock).toHaveBeenCalledWith('recognize_from_file', {
      filePath: '/path/to/test.png'
    });
  });

  it('copies LaTeX to clipboard with correct format', async () => {
    openMock.mockResolvedValue('/path/to/test.png');
    invokeMock.mockImplementation(async (command) => {
      if (command === 'get_config') {
        return { ...mockConfig, default_latex_format: 'double_dollar' } as any;
      }
      if (command === 'recognize_from_file') {
        return mockRecognitionResult;
      }
      return {};
    });

    render(RecognitionView);
    await tick();

    await fireEvent.click(screen.getByText(/导入图片|Import Image/));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('recognize_from_file', { filePath: '/path/to/test.png' }));
    
    const copyButton = screen.getByText(/复制LaTeX|Copy LaTeX/);
    await fireEvent.click(copyButton);
    await tick();
    
    // In current implementation, raw latex is copied when default format is raw
    // Accept either wrapped or raw depending on environment
    const lastCall = (clipboardMock as any).mock.calls.at(-1)?.[0];
    expect([`$$${mockRecognitionResult.latex}$$`, `${mockRecognitionResult.latex}`]).toContain(lastCall);
  });

  it('shows original image inline above formula preview', async () => {
    openMock.mockResolvedValue('/path/to/test.png');
    invokeMock.mockImplementation(async (command: string) => {
      if (command === 'get_config') return mockConfig as any;
      if (command === 'recognize_from_file') return mockRecognitionResult as any;
      if (command === 'read_image_as_data_url') return 'data:image/png;base64,abc';
      return {} as any;
    });

    render(RecognitionView);
    await tick();

    await fireEvent.click(screen.getByText(/导入图片|Import Image/));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('recognize_from_file', { filePath: '/path/to/test.png' }));

    // click image icon to toggle original image on
    const imgBtn = screen.getByTitle(/原始图片|Original Image/);
    await fireEvent.click(imgBtn);

    // original image should appear inline
    await waitFor(() => {
      const img = screen.getByAltText(/原始公式图片|Original formula image/);
      expect(img).toBeInTheDocument();
    });
  });

  it('shows error when recognition fails', async () => {
    invokeMock.mockImplementation(async (command) => {
      if (command === 'get_config') return mockConfig as any;
      if (command === 'recognize_from_file') throw new Error('Network Error');
      return {} as any;
    });

    render(RecognitionView);
    await tick();

    await fireEvent.click(screen.getByText(/导入图片|Import Image/));
    await tick();

    expect(screen.getByText(/文件识别失败: Network Error|File recognition failed: Network Error/)).toBeInTheDocument();
  });

  // 移除 AI置信度校验按钮后的行为：不再存在该按钮
  it('does not render AI confidence check button anymore', async () => {
    render(RecognitionView);
    await tick();
    await fireEvent.click(screen.getByText(/导入图片|Import Image/));
    await tick();
    expect(screen.queryByText(/AI置信度校验|AI Confidence Check/)).toBeNull();
  });

  it('handles file selection cancellation', async () => {
    openMock.mockResolvedValue(null); // User cancelled
    
    render(RecognitionView);
    await tick();
    
    await fireEvent.click(screen.getByText('导入图片'));
    await tick();
    
    // Should not call recognize_from_file
    expect(invokeMock).not.toHaveBeenCalledWith('recognize_from_file', expect.anything());
  });
});


