import '@testing-library/jest-dom';
// Ensure Settings-related tests run in English UI
// Other tests that rely on Chinese copy render their own components and are unaffected
(globalThis as any).__TEST_LANG__ = 'en';

// Polyfill IntersectionObserver for JSDOM test environment
if (typeof (globalThis as any).IntersectionObserver === 'undefined') {
  (globalThis as any).IntersectionObserver = class {
    constructor(_callback: any, _options?: any) {}
    observe() {}
    unobserve() {}
    disconnect() {}
  } as any;
}