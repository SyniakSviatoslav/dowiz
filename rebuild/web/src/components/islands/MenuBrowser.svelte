<script lang="ts">
  // MenuBrowser island — the largest storefront island (REBUILD-MAP inventory 11 §7.1 row 1,
  // client:load). Hydrates OVER the Astro-SSR category-chip nav + product grid: smooth-scroll,
  // IntersectionObserver scroll-spy (parity: MenuPage.tsx:516-533), a client-side text filter, and
  // add-to-cart event delegation (one listener on the grid root, not one per ProductCard).
  //
  // Deliberately thin for the Phase-A spike: no product-detail sheet, no modifiers UI yet (those
  // are Phase-B scope per REBUILD-MAP §7.1 rationale — "largest storefront island" grows from here).
  import { onMount } from 'svelte';
  import { cart } from '../../lib/cart-store.svelte';

  let searchQuery = $state('');
  let activeCategory = $state<string | null>(null);

  function pickCategory(id: string | null) {
    activeCategory = id;
    const target = id ? document.getElementById(id) : document.querySelector('.venue-header');
    target?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  function handleGridClick(e: MouseEvent) {
    const btn = (e.target as HTMLElement).closest<HTMLElement>('[data-add-to-cart]');
    if (!btn) return;
    const card = btn.closest<HTMLElement>('[data-product-id]');
    if (!card) return;
    cart.add({
      id: card.dataset.productId!,
      name: card.dataset.productName!,
      price: Number(card.dataset.productPrice ?? 0),
    });
    window.dispatchEvent(new CustomEvent('dos:bounceCart'));
  }

  function applyFilter() {
    const q = searchQuery.trim().toLowerCase();
    document.querySelectorAll<HTMLElement>('[data-product-id]').forEach((card) => {
      const name = card.dataset.productName?.toLowerCase() ?? '';
      card.style.display = !q || name.includes(q) ? '' : 'none';
    });
  }

  $effect(() => {
    applyFilter();
  });

  onMount(() => {
    const root = document.querySelector('main') as HTMLElement | null;
    const sections = Array.from(document.querySelectorAll<HTMLElement>('section[id]'));
    const io = new IntersectionObserver(
      (entries) => {
        const hit = entries
          .filter((e) => e.isIntersecting)
          .sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)[0];
        if (hit) activeCategory = hit.target.id;
      },
      { root, rootMargin: '-64px 0px -62% 0px', threshold: 0 },
    );
    sections.forEach((s) => io.observe(s));

    document.querySelectorAll<HTMLAnchorElement>('[data-category-chip]').forEach((chip) => {
      chip.addEventListener('click', (e) => {
        e.preventDefault();
        const id = chip.dataset.categoryChip === '__all' ? null : chip.dataset.categoryChip ?? null;
        pickCategory(id);
      });
    });

    const grid = document.querySelector('[data-product-grid]');
    grid?.addEventListener('click', handleGridClick);

    return () => {
      io.disconnect();
      grid?.removeEventListener('click', handleGridClick);
    };
  });

  $effect(() => {
    document.querySelectorAll<HTMLElement>('[data-category-chip]').forEach((chip) => {
      const isActive =
        (chip.dataset.categoryChip === '__all' && activeCategory === null) ||
        chip.dataset.categoryChip === activeCategory;
      chip.setAttribute('aria-pressed', String(isActive));
    });
  });
</script>

<div class="menu-browser-search">
  <input
    type="search"
    placeholder="Search the menu…"
    bind:value={searchQuery}
    aria-label="Search menu"
  />
</div>

<style>
  .menu-browser-search {
    padding: 0 1rem 0.5rem;
  }
  .menu-browser-search input {
    width: 100%;
    padding: 0.5rem 0.75rem;
    border-radius: 0.5rem;
    border: 1px solid var(--brand-border);
    background: var(--brand-surface-raised);
    color: var(--brand-text);
  }
</style>
