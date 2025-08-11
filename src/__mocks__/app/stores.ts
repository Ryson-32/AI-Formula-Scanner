import { writable, type Writable } from 'svelte/store';

// Create a writable store for page
const pageStore = writable({
  url: { pathname: '/' },
  params: {},
  route: { id: null },
  data: {}
});

export const page = pageStore;
export const navigating = writable(null);
export const updated = writable(false);
