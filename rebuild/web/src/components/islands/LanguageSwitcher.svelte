<script lang="ts">
  // LanguageSwitcher island — parity: packages/ui LanguageSwitcher (compact/full variants),
  // consumed inside StorefrontShellControls per REBUILD-MAP inventory 11 §7.1 row 4
  // ("client:idle — tiny"). Wired to the Paraglide stand-in (src/lib/paraglide-stub.ts); once
  // the real @inlang/paraglide-js runtime is installed, only the import path changes — the
  // `setLocale`/`getLocale`/`locales` call shape is Paraglide 2's own public API, kept identical
  // here on purpose.
  import { locales, getLocale, setLocale, type Locale } from '../../lib/paraglide-stub';

  let current = $state<Locale>(getLocale());

  function choose(l: Locale) {
    setLocale(l);
    current = l;
    document.documentElement.lang = l;
  }
</script>

<div class="language-switcher" role="group" aria-label="Language">
  {#each locales as l (l)}
    <button
      type="button"
      class:active={current === l}
      onclick={() => choose(l)}
      aria-pressed={current === l}
    >
      {l.toUpperCase()}
    </button>
  {/each}
</div>

<style>
  .language-switcher {
    display: inline-flex;
    gap: 0.25rem;
  }
  .language-switcher button {
    border: 1px solid var(--brand-border);
    background: transparent;
    color: var(--brand-text-muted);
    border-radius: var(--radius-full);
    padding: 0.15rem 0.5rem;
    font-size: 0.75rem;
  }
  .language-switcher button.active {
    background: var(--brand-primary);
    color: var(--brand-bg);
    border-color: var(--brand-primary);
  }
</style>
