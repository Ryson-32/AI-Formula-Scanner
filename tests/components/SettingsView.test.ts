import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { tick } from 'svelte';
import SettingsView from '../../src/components/SettingsView.svelte';
import { setLanguage } from '$lib/i18n';
import { invoke } from '@tauri-apps/api/tauri';
import { showToast } from '$lib/toast';

// Mock Tauri API
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn() }));

// Mock toast store: intercept showToast imports
vi.mock('$lib/toast', () => {
  const showToast = vi.fn();
  return { showToast, toasts: { subscribe: () => () => {} } };
});

const mockConfig = {
  apiKey: 'test-api-key',
  apiBaseUrl: 'https://generativelanguage.googleapis.com/v1beta/models',
  provider: 'gemini',
  defaultEngine: 'gemini-2.0-flash-exp',
  customPrompt: 'Test prompt for LaTeX recognition',
  verificationPrompt: 'Test verification prompt',
  renderEngine: 'MathJax',
  autoCalculateConfidence: true,
  enableClipboardWatcher: false,
  defaultLatexFormat: 'double_dollar',
  requestTimeoutSeconds: 30,
  maxRetries: 3
};

describe('SettingsView.svelte', () => {
  const invokeMock = vi.mocked(invoke);

  beforeEach(() => {
    vi.resetAllMocks();
    vi.clearAllTimers();
    vi.useFakeTimers();
    // Force English to match test assertions
    setLanguage('en');
    
    invokeMock.mockImplementation(async (command: string) => {
      if (command === 'get_config') {
        return mockConfig;
      }
      if (command === 'save_config') {
        return {};
      }
      return {};
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders the settings page title', async () => {
    render(SettingsView);
    await tick();
    
    expect(screen.getByText('Settings')).toBeInTheDocument();
  });

  it('loads and displays configuration on mount', async () => {
    render(SettingsView);
    await tick();
    
    expect(invokeMock).toHaveBeenCalledWith('get_config');
    
    // Wait for config to be loaded and UI to update
    await waitFor(() => {
      const apiKeyInput = screen.getByPlaceholderText('Enter your API key') as HTMLInputElement;
      expect(apiKeyInput.value).toBe(mockConfig.apiKey);
    });
  });

  it('renders all form sections and inputs', async () => {
    render(SettingsView);
    await tick();
    
    // Check main sections
    expect(screen.getByText('AI Configuration')).toBeInTheDocument();
    
    // Check form inputs
    expect(screen.getByLabelText(/Provider|服务提供商/)).toBeInTheDocument();
    expect(screen.getByLabelText('API Key')).toBeInTheDocument();
    expect(screen.getByLabelText('API Base URL')).toBeInTheDocument();
    expect(screen.getByLabelText('Model')).toBeInTheDocument();
    
    // Check test button
    expect(screen.getByText('Test')).toBeInTheDocument();
  });

  it('handles test connection button click', async () => {
    render(SettingsView);
    await tick();
    
    const testButton = screen.getByText(/Test|测试/);
    await fireEvent.click(testButton);
    
    // Now shows toast "coming soon" message
    await waitFor(() => expect(showToast).toHaveBeenCalled());
  });

  it('saves configuration with debouncing', async () => {
    render(SettingsView);
    await tick();
    
    // Wait for initial config load
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('get_config');
    });
    
    // Clear the initial call
    invokeMock.mockClear();
    
    // Simulate user input
    const apiKeyInput = screen.getByPlaceholderText(/Enter your API key|请输入你的 API 密钥/);
    await fireEvent.input(apiKeyInput, { target: { value: 'new-api-key' } });
    
    // Should not save immediately (debounced)
    expect(invokeMock).not.toHaveBeenCalledWith('save_config', expect.anything());
    
    // Fast-forward timers to trigger debounced save
    vi.advanceTimersByTime(500);
    await tick();
    
    expect(invokeMock).toHaveBeenCalledWith('save_config', expect.objectContaining({
      config: expect.objectContaining({
        apiKey: 'new-api-key'
      })
    }));
  });

  it('handles config loading failure gracefully', async () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    invokeMock.mockImplementation(async (command) => {
      if (command === 'get_config') {
        throw new Error('Config load failed');
      }
      return {};
    });
    
    render(SettingsView);
    await tick();
    
    expect(consoleErrorSpy).toHaveBeenCalledWith('Failed to load config:', expect.any(Error));
    
    consoleErrorSpy.mockRestore();
  });

  it('handles config saving failure gracefully', async () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    
    invokeMock.mockImplementation(async (command) => {
      if (command === 'get_config') {
        return mockConfig;
      }
      if (command === 'save_config') {
        throw new Error('Save failed');
      }
      return {};
    });
    
    render(SettingsView);
    await tick();
    
    // Wait for config to load
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('get_config');
    });
    
    invokeMock.mockClear();
    
    // Trigger a save
    const apiKeyInput = screen.getByPlaceholderText(/Enter your API key|请输入你的 API 密钥/);
    await fireEvent.input(apiKeyInput, { target: { value: 'new-key' } });
    
    vi.advanceTimersByTime(500);
    await tick();
    
    expect(consoleErrorSpy).toHaveBeenCalledWith('Failed to save config:', expect.any(Error));
    
    consoleErrorSpy.mockRestore();
  });

  it('prevents saving empty config on initial load', async () => {
    render(SettingsView);
    await tick();
    
    // Should only call get_config, not save_config for empty initial state
    expect(invokeMock).toHaveBeenCalledWith('get_config');
    expect(invokeMock).not.toHaveBeenCalledWith('save_config', expect.anything());
  });

  it('debounces multiple rapid changes', async () => {
    render(SettingsView);
    await tick();
    
    // Wait for initial load
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('get_config');
    });
    
    invokeMock.mockClear();
    
    const apiKeyInput = screen.getByPlaceholderText(/Enter your API key|请输入你的 API 密钥/);
    
    // Make multiple rapid changes
    await fireEvent.input(apiKeyInput, { target: { value: 'key1' } });
    await fireEvent.input(apiKeyInput, { target: { value: 'key2' } });
    await fireEvent.input(apiKeyInput, { target: { value: 'key3' } });
    
    // Should not save yet
    expect(invokeMock).not.toHaveBeenCalledWith('save_config', expect.anything());
    
    // Fast-forward past debounce time
    vi.advanceTimersByTime(500);
    await tick();
    
    // Should only save once with the final value
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('save_config', expect.objectContaining({
      config: expect.objectContaining({
        apiKey: 'key3'
      })
    }));
  });

  it('renders provider dropdown with options', async () => {
    render(SettingsView);
    await tick();
    
    const providerSelect = screen.getByLabelText(/Provider|服务提供商/);
    expect(providerSelect).toBeInTheDocument();
    
    // Check dropdown options
    expect(screen.getByText(/OpenAI/)).toBeInTheDocument();
    expect(screen.getAllByText(/Google/).length).toBeGreaterThan(0);
    expect(screen.getByText(/Anthropic/)).toBeInTheDocument();
  });

  it('applies correct CSS classes and styling', async () => {
    render(SettingsView);
    await tick();
    
    const settingsView = document.querySelector('.settings-view');
    expect(settingsView).toBeInTheDocument();
    
    const settingsGroup = document.querySelector('.settings-group');
    expect(settingsGroup).toBeInTheDocument();
    
    const formGrid = document.querySelector('.form-grid');
    expect(formGrid).toBeInTheDocument();
  });
});


