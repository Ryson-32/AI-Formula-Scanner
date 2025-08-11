<script lang="ts">
  import HistoryView from '../../components/HistoryView.svelte';
  import { currentLang, translateNow } from '$lib/i18n';
</script>

<div class="history-page">
  <header class="page-header">
    <h1 class="page-title">{translateNow('history.title', $currentLang)}</h1>
  </header>
  <HistoryView />
  
  </div>

<style>
  .history-page { width: 100%; margin: 0; padding: 0; }
  
  .page-header { margin-bottom: var(--spacing-lg); }
  .page-title { font-size: var(--font-size-h1); font-weight: var(--font-weight-bold); color: var(--text-header); margin: 0; }
</style>