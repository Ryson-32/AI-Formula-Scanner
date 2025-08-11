<script lang="ts">
  import FavoritesView from '../../components/FavoritesView.svelte';
  import { currentLang, translateNow } from '$lib/i18n';
</script>

<div class="favorites-page">
  <header class="page-header">
    <h1 class="page-title">{translateNow('favorites.title', $currentLang)}</h1>
  </header>
  <FavoritesView />
</div>

<style>
  .favorites-page { width: 100%; margin: 0; padding: 0; }
  .page-header { margin-bottom: var(--spacing-lg); }
  .page-title { font-size: var(--font-size-h1); font-weight: var(--font-weight-bold); color: var(--text-header); margin: 0; }
</style>