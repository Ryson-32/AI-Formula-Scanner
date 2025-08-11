<script>
  import RecognitionView from '../components/RecognitionView.svelte';
  import { currentLang, translateNow } from '$lib/i18n';
</script>

<div class="main-page">
  <header class="page-header">
    <h1 class="page-title">{translateNow('app.title', $currentLang)}</h1>
  </header>
  <RecognitionView />
</div>

<style>
  .main-page { width: 100%; padding: 0; }
  .page-header { margin-bottom: var(--spacing-lg); }
  .page-title { font-size: var(--font-size-h1); font-weight: var(--font-weight-bold); line-height: var(--line-height-tight); color: var(--text-header); margin: 0; }
</style>