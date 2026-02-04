import '@testing-library/jest-dom/vitest';

// Mock clipboard API (not available in jsdom)
Object.assign(navigator, {
  clipboard: {
    writeText: () => Promise.resolve(),
    readText: () => Promise.resolve(''),
  },
});
