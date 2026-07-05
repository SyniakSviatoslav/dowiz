<script lang="ts">
  // LanguageSwitcher island — parity: packages/ui LanguageSwitcher (compact/full variants),
  // consumed inside StorefrontShellControls per REBUILD-MAP inventory 11 §7.1 row 4
  // ("client:idle — tiny"). Wired to the real compiled `src/paraglide/runtime.js` (recompiled
  // with `--strategy globalVariable baseLocale` — see gen:messages in package.json — so locale
  // resolution stays an in-memory module variable defaulting to baseLocale, same behavior as the
  // stub this replaced; no cookie/URL/localStorage strategies are included, so none of that code
  // ships). `reload: false` matches the stub's non-reloading behavior — Paraglide's default is to
  // full-page-reload on setLocale so SSR-rendered strings re-render in the new locale too; that's
  // a deliberate follow-up for whoever wires real locale-aware routing, not in scope here.
  import { locales, getLocale, setLocale } from '../../paraglide/runtime.js';

  type Locale = (typeof locales)[number];

  let current = $state<Locale>(getLocale());

  function choose(l: Locale) {
    setLocale(l, { reload: false });
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
