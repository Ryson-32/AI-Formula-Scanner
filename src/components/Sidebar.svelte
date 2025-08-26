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

    <!-- External Links Section -->
    <div class="external-links">
      <a href="https://github.com/Ryson-32/AI-Formula-Scanner" target="_blank" rel="noopener noreferrer" class="external-link github-link" title="GitHub Repository">
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="white" class="github-icon">
          <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"></path>
        </svg>
      </a>
      <a href="https://linux.do/t/topic/859440?u=ryanvan" target="_blank" rel="noopener noreferrer" class="external-link linuxdo-link" title="LinuxDo Community">
        <img src="/icons/linuxdo.png" alt="LinuxDo" class="linuxdo-icon" width="24" height="24" />
      </a>
      <div class="link-spacer"></div>
    </div>
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

  .external-links {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    margin-top: var(--spacing-md);
    padding-top: var(--spacing-sm);
    width: 100%;
  }

  .external-link {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .github-icon {
    background: #1a1a1a;
    border-radius: 50%;
    padding: 4px;
    transition: all 0.2s ease;
    width: 24px;
    height: 24px;
    flex-shrink: 0;
  }

  .linuxdo-icon {
    border-radius: 50%;
    transition: all 0.2s ease;
    width: 24px;
    height: 24px;
    object-fit: contain;
    flex-shrink: 0;
  }

  .external-link:hover .github-icon {
    background: #333;
    transform: scale(1.1);
  }

  .external-link:hover .linuxdo-icon {
    transform: scale(1.1);
  }

  .link-spacer {
    flex: 1;
  }
</style>