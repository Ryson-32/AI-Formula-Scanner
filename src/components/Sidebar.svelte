<script>
  import { page } from '$app/stores';
  import { currentLang, translateNow } from '$lib/i18n';
  import { ScanText, History, Star, Settings } from 'lucide-svelte';
  const appIconUrl = '/icons/app-icon.jpg';

</script>

<nav class="sidebar">
  <div class="sidebar-top">
    <div class="logo">
      <img
        class="app-icon"
        src={appIconUrl}
        srcset="/icons/app-icon-128.png 2x, /icons/app-icon-192.png 3x"
        alt={translateNow('app.title', $currentLang)}
      />
      <span class="logo-text">{translateNow('app.title', $currentLang)}</span>
    </div>
    <ul class="nav-links">
      <li>
        <a href="/" class:active={$page.url.pathname === '/'} title={translateNow('nav.recognition', $currentLang)}>
          <span class="icon"><ScanText size={20} /></span>
          <span class="text">{translateNow('nav.recognition', $currentLang)}</span>
        </a>
      </li>
      <li>
        <a href="/history" class:active={$page.url.pathname === '/history'} title={translateNow('nav.history', $currentLang)}>
          <span class="icon"><History size={20} /></span>
          <span class="text">{translateNow('nav.history', $currentLang)}</span>
        </a>
      </li>
      <li>
        <a href="/favorites" class:active={$page.url.pathname === '/favorites'} title={translateNow('nav.favorites', $currentLang)}>
          <span class="icon"><Star size={20} /></span>
          <span class="text">{translateNow('nav.favorites', $currentLang)}</span>
        </a>
      </li>
    </ul>
  </div>

  <div class="sidebar-bottom">
    <a href="/settings" class="nav-link" title={translateNow('nav.settings', $currentLang)} class:active={$page.url.pathname === '/settings'}>
      <span class="icon"><Settings size={20} /></span>
      <span class="text">{translateNow('nav.settings', $currentLang)}</span>
    </a>
  </div>
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    height: 100vh;
    width: var(--sidebar-width);
    min-width: var(--sidebar-width);
    flex: 0 0 var(--sidebar-width); /* 禁止收缩，固定占位 */
    background: linear-gradient(180deg, var(--bg-secondary) 0%, #f1f5f9 100%);
    padding: var(--spacing-md);
    border-right: 1px solid var(--border-primary);
    box-shadow: 1px 0 3px rgba(0,0,0,0.05);
  }

  .logo {
    margin-bottom: var(--spacing-xl);
    text-align: center;
    padding: var(--spacing-md) 0;
  }

  .app-icon {
    width: 56px;
    height: 56px;
    border-radius: 12px;
    object-fit: cover;
    display: block;
    margin: 0 auto var(--spacing-sm);
    border: 1px solid var(--border-primary);
    box-shadow: 0 2px 6px rgba(0,0,0,0.06);
  }

  .logo-text {
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
    color: var(--text-header);
  }

  .nav-links {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .nav-links li {
    margin: 0 var(--spacing-xs);
  }

  a {
    display: flex;
    align-items: center;
    padding: var(--spacing-md) var(--spacing-base);
    text-decoration: none;
    color: var(--text-default);
    border-radius: var(--border-radius-btn);
    transition: all 0.2s ease;
    font-weight: var(--font-weight-medium);
    font-size: var(--font-size-form-label);
  }

  a:hover:not(.active) {
    background-color: var(--bg-hover);
    color: var(--text-header);
    transform: translateX(2px);
  }

  a.active {
    background-color: var(--primary);
    color: white;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }

  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-right: var(--spacing-md);
  }

  .icon :global(svg) {
    width: 20px;
    height: 20px;
  }

  .text {
    font-size: var(--font-size-form-label);
    font-weight: var(--font-weight-bold);
  }

  .sidebar-bottom {
    padding-top: var(--spacing-md);
    margin-top: var(--spacing-lg);
    border-top: 1px solid var(--border-primary);
  }
</style>