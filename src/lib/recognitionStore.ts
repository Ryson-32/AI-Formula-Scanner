import { writable } from 'svelte/store';
import type { RecognitionResult } from '$lib/types';

export type RecognitionState = {
  result: RecognitionResult | null;
  isLoading: boolean;
  errorMessage: string;
  currentPromptVersion?: string;
  currentImageBase64?: string;
};

const initialState: RecognitionState = {
  result: null,
  isLoading: false,
  errorMessage: '',
  currentPromptVersion: undefined
};

const { subscribe, set, update } = writable<RecognitionState>(initialState);

export const recognitionStore = {
  subscribe,
  reset(): void {
    set(initialState);
  },
  setLoading(value: boolean): void {
    update((state) => ({ ...state, isLoading: value }));
  },
  start(): void {
    update((state) => ({ ...state, isLoading: true, errorMessage: '' }));
  },
  finish(result: RecognitionResult): void {
    update((state) => ({ ...state, isLoading: false, result }));
  },
  setError(message: string): void {
    update((state) => ({ ...state, isLoading: false, errorMessage: message }));
  },
  clearError(): void {
    update((state) => ({ ...state, errorMessage: '' }));
  },
  setResult(result: RecognitionResult | null): void {
    update((state) => ({ ...state, result }));
  },
  updateLatex(latex: string): void {
    update((state) => {
      if (!state.result) return state;
      return { ...state, result: { ...state.result, latex } };
    });
  },
  patch(partial: Partial<RecognitionResult>): void {
    update((state) => {
      if (!state.result) return state;
      return { ...state, result: { ...state.result, ...partial } };
    });
  },
  setPromptVersion(version: string): void {
    update((state) => ({ ...state, currentPromptVersion: version }));
  },
  setCurrentImageBase64(value: string | null): void {
    update((state) => ({ ...state, currentImageBase64: value ?? undefined }));
  }
};


