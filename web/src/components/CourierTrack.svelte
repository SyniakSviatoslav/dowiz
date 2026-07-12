<script>
  // CourierTrack — Svelte 5 tracking island.
  //
  // Shows the live delivery stage for a courier-facing order and fires the
  // matching WebGL2 particle-cloud event-visual for the current kernel status.
  //
  // RED LINES (kernel scope, do NOT violate):
  //   * NO money tween / count-up — every number is static, exact text.
  //   * NO courier scoring / rating — no score field, leaderboard, or rating UI.
  //
  // Status prop is the canonical kernel OrderStatus string (UPPER_SNAKE),
  // mirroring kernel/src/order_machine.rs OrderStatus::as_str().

  import { onMount } from 'svelte';
  import { createParticleCloud } from '../../../webgl/particle-cloud/particle-cloud.js';

  // props
  let {
    status = 'PENDING',
    orderId = '',
    pickup = '',
    dropoff = '',
  } = $props();

  // Kernel OrderStatus (UPPER_SNAKE) -> particle-cloud VOCAB key.
  // Mirrors the mapping requested by the courier-tracking task:
  //   InDelivery -> courier_assigned, Delivered -> delivered,
  //   Pending/Preparing -> pending_aging, Rejected -> dispatch_failed.
  // The remaining variants are mapped by the same semantic intent so any
  // kernel status resolves to a valid VOCAB key (never undefined).
  const STATUS_TO_VOCAB = Object.freeze({
    IN_DELIVERY: 'courier_assigned',
    DELIVERED: 'delivered',
    PENDING: 'pending_aging',
    CONFIRMED: 'pending_aging',
    PREPARING: 'pending_aging',
    READY: 'pending_aging',
    SCHEDULED: 'pending_aging',
    PICKED_UP: 'courier_assigned',
    REJECTED: 'dispatch_failed',
    CANCELLED: 'dispatch_failed',
  });

  // Human-readable stage label — static exact text (no animation).
  const STAGE_LABEL = Object.freeze({
    PENDING: 'Order received',
    CONFIRMED: 'Confirmed',
    PREPARING: 'Preparing your order',
    READY: 'Ready for pickup',
    IN_DELIVERY: 'Out for delivery',
    DELIVERED: 'Delivered',
    REJECTED: 'Rejected',
    CANCELLED: 'Cancelled',
    SCHEDULED: 'Scheduled',
    PICKED_UP: 'Picked up',
  });

  let canvas = $state(null);
  let cloud = null;
  let pushState = $state('idle'); // idle | requesting | granted | denied | unsupported | error
  let pushMsg = $state('');

  function vocabFor(s) {
    return STATUS_TO_VOCAB[s] || 'pending_aging';
  }

  onMount(() => {
    // Respect prefers-reduced-motion for the particle burst.
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    try {
      cloud = createParticleCloud();
      cloud.init(canvas);
      cloud.setReducedMotion(mq.matches);
      // Fire the event-visual for the CURRENT status on mount.
      // Hard 24-particle burst (no tween, no count-up).
      cloud.burst(vocabFor(status), 24);
    } catch (e) {
      // WebGL2 may be unavailable (SSR-safe path already guards; this is the
      // client-only failure case). Tracking text still renders without it.
      console.warn('[CourierTrack] particle cloud unavailable:', e);
    }

    const onMotionChange = (e) => {
      if (cloud) cloud.setReducedMotion(e.matches);
    };
    mq.addEventListener?.('change', onMotionChange);

    return () => {
      mq.removeEventListener?.('change', onMotionChange);
      if (cloud) cloud.dispose();
      cloud = null;
    };
  });

  // Opt-in: enable out-of-app dispatch alerts (wires push.js + sw.js).
  // No scoring/rating — purely a notification-subscription action.
  async function enableAlerts() {
    pushState = 'requesting';
    pushMsg = '';
    try {
      const { registerCourierPush } = await import('../lib/push.js');
      await registerCourierPush();
      pushState = 'granted';
      pushMsg = 'Dispatch alerts enabled.';
    } catch (e) {
      const denied = e && /denied/i.test(String(e.message));
      pushState = denied ? 'denied' : 'error';
      pushMsg = e && e.message ? e.message : 'Could not enable alerts.';
    }
  }
</script>

<section class="courier-track">
  <header>
    <h2>Live status</h2>
    {#if orderId}
      <span class="order-id">#{String(orderId).slice(0, 8).toUpperCase()}</span>
    {/if}
  </header>

  <div class="stage">
    <!-- Static exact stage text. NO count-up / money tween. -->
    <strong>{STAGE_LABEL[status] || status}</strong>
  </div>

  <canvas bind:this={canvas} class="cloud" aria-hidden="true"></canvas>

  <dl class="route">
    <dt>Pickup</dt>
    <dd>{pickup || '—'}</dd>
    <dt>Dropoff</dt>
    <dd>{dropoff || '—'}</dd>
  </dl>

  <button
    type="button"
    onclick={enableAlerts}
    disabled={pushState === 'requesting' || pushState === 'granted'}
  >
    {pushState === 'granted'
      ? 'Dispatch alerts on'
      : pushState === 'requesting'
        ? 'Enabling…'
        : 'Enable dispatch alerts'}
  </button>

  {#if pushMsg}
    <p class="push-note" role="status">{pushMsg}</p>
  {/if}
</section>

<style>
  .courier-track {
    border: 1px solid #ddd;
    border-radius: 10px;
    padding: 1rem;
    margin: 1rem 0;
    position: relative;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
  }
  header h2 {
    margin: 0;
    font-size: 1rem;
  }
  .order-id {
    font-family: ui-monospace, monospace;
    font-size: 0.8rem;
    color: #666;
  }
  .stage {
    margin: 0.75rem 0;
    font-size: 1.25rem;
  }
  .cloud {
    display: block;
    width: 100%;
    height: 120px;
    background: transparent;
  }
  .route {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.2rem 0.75rem;
    margin: 0.5rem 0;
  }
  .route dt {
    font-weight: 600;
    color: #555;
  }
  .route dd {
    margin: 0;
  }
  button {
    cursor: pointer;
    padding: 0.4rem 0.8rem;
    border-radius: 8px;
    border: 1px solid #999;
    background: #f4f4f4;
  }
  button:disabled {
    cursor: default;
    opacity: 0.7;
  }
  .push-note {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
    color: #444;
  }
</style>
