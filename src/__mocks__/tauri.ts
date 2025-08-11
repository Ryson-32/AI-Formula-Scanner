import { vi } from 'vitest';

export const invoke = vi.fn();
export const open = vi.fn();
export const readBinaryFile = vi.fn();

export const clipboard = {
  writeText: vi.fn(),
};

// Mock a file selection for the 'open' dialog
export function mockFileSelect(filePath: string | null) {
  (open as any).mockResolvedValue(filePath);
}

// Mock the binary content of a file
export function mockFileContent(content: Uint8Array) {
  (readBinaryFile as any).mockResolvedValue(content);
}