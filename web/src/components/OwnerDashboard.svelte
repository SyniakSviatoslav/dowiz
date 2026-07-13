<script>
  // ── Owner dashboard island ──────────────────────────────────────────
  // Read-only dashboard rendering ChannelLedger output (orders_by_channel +
  // funnel + anomalies). First deterministic reader of order attribution on
  // the frontend (roadmap finding: "attribution captured but ZERO readers on
  // prod; measurement loop open").
  //
  // Compute lives in the kernel WASM (the canonical authority). We call
  // `channelLedger` (web/src/lib/kernel.js) which invokes the kernel's
  // `channel_ledger_js` — one stateless call returns {orders_by_channel,
  // funnel, anomalies}. The legacy plain-JS port (web/src/lib/channel.js) was
  // deleted (RW-02): it was a byte-for-byte mirror of kernel/src/analytics.rs,
  // proven equivalent by web/src/lib/kernel/kernel.test.mjs.
  // Static EXACT numbers — no count-up tween, no animation. No courier scoring.
  //
  // ChannelEvent-like JS object shape (mirrors kernel/src/analytics.rs):
  //   { order_id: string, channel: string, status: string, at_ms: number }
  // `status` is the canonical UPPER_SNAKE string (e.g. "PENDING", "DELIVERED").
  import { onMount } from 'svelte';
  import { channelLedger } from '../lib/kernel.js';

  // ── Hardcoded sample events (oracle-shaped) ─────────────────────────
  // Mix of channels, a duplicate order_id (o1 re-seen, must be ignored for
  // channel attribution but updates the funnel status), and an illegal
  // sequence (o_anom2: PENDING -> DELIVERED) that the anomaly reducer flags.
  const SAMPLE_EVENTS = [
    { order_id: 'o1', channel: 'tiktok', status: 'PENDING', at_ms: 1 },
    { order_id: 'o1', channel: 'instagram', status: 'CONFIRMED', at_ms: 2 }, // duplicate id -> ignored re-attribute
    { order_id: 'o2', channel: 'tiktok', status: 'CONFIRMED', at_ms: 3 },
    { order_id: 'o3', channel: 'tiktok', status: 'DELIVERED', at_ms: 4 },
    { order_id: 'o4', channel: 'instagram', status: 'PENDING', at_ms: 5 },
    { order_id: 'o5', channel: 'instagram', status: 'REJECTED', at_ms: 6 },
    { order_id: 'o6', channel: 'organic', status: 'DELIVERED', at_ms: 7 },
    { order_id: 'o7', channel: 'tiktok', status: 'PREPARING', at_ms: 8 },
    // Anomaly stream — folded by the kernel's decide/fold Law over the same
    // (order_id, status, at_ms) tuples.
    { order_id: 'o_anom1', channel: 'tiktok', status: 'PENDING', at_ms: 9 },
    { order_id: 'o_anom1', channel: 'tiktok', status: 'CONFIRMED', at_ms: 10 },
    { order_id: 'o_anom1', channel: 'tiktok', status: 'PREPARING', at_ms: 11 },
    { order_id: 'o_anom1', channel: 'tiktok', status: 'READY', at_ms: 12 },
    { order_id: 'o_anom1', channel: 'tiktok', status: 'IN_DELIVERY', at_ms: 13 },
    { order_id: 'o_anom1', channel: 'tiktok', status: 'DELIVERED', at_ms: 14 },
    { order_id: 'o_anom2', channel: 'instagram', status: 'PENDING', at_ms: 15 },
    { order_id: 'o_anom2', channel: 'instagram', status: 'DELIVERED', at_ms: 16 }, // illegal jump -> anomaly
  ];

  // Kernel is the authority — one stateless call returns everything.
  let ordersByChannel = $state([]);
  let funnelMap = $state({});
  let anomalyCount = $state(0);

  onMount(async () => {
    const res = await channelLedger(SAMPLE_EVENTS);
    ordersByChannel = res.orders_by_channel;
    funnelMap = res.funnel;
    anomalyCount = res.anomalies;
  });

  // Max channel count for bar scaling.
  const maxChannelCount = $derived(
    ordersByChannel.reduce((m, [, c]) => Math.max(m, c), 0)
  );

  // Selected channel for the funnel view (default to first / highest count once loaded).
  let selectedChannel = $state('');
  $effect(() => {
    if (!selectedChannel && ordersByChannel.length > 0) {
      selectedChannel = ordersByChannel[0][0];
    }
  });

  // Funnel rows for the selected channel (status -> count, fixed stage order).
  const funnelRows = $derived(selectedChannel ? (funnelMap[selectedChannel] ?? []) : []);
  const maxFunnelCount = $derived(
    funnelRows.reduce((m, [, c]) => Math.max(m, c), 0)
  );

  const STATUS_LABEL = Object.freeze({
    PENDING: 'Pending',
    CONFIRMED: 'Confirmed',
    PREPARING: 'Preparing',
    READY: 'Ready',
    IN_DELIVERY: 'In delivery',
    DELIVERED: 'Delivered',
    REJECTED: 'Rejected',
    CANCELLED: 'Cancelled',
    SCHEDULED: 'Scheduled',
    PICKED_UP: 'Picked up',
  });
</script>

<section class="owner-dashboard">
  <h2>Owner dashboard — channel attribution</h2>

  <div class="cards">
    <!-- (a) orders_by_channel bar list -->
    <article class="card">
      <h3>Orders by channel</h3>
      {#if ordersByChannel.length === 0}
        <p>No attributed orders.</p>
      {:else}
        <ul class="barlist">
          {#each ordersByChannel as [channel, count]}
            <li>
              <button
                type="button"
                class="ch-row"
                class:selected={channel === selectedChannel}
                onclick={() => (selectedChannel = channel)}
              >
                <span class="ch-name">{channel}</span>
                <span class="ch-bar" style="width: {maxChannelCount ? (count / maxChannelCount) * 100 : 0}%"></span>
                <span class="ch-count">{count}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </article>

    <!-- (b) funnel for selected channel -->
    <article class="card">
      <h3>
        Funnel{selectedChannel ? ` — ${selectedChannel}` : ''}
      </h3>
      {#if !selectedChannel}
        <p>Select a channel.</p>
      {:else}
        <ul class="funnel">
          {#each funnelRows as [status, count]}
            <li>
              <span class="fn-stage">{STATUS_LABEL[status] ?? status}</span>
              <span class="fn-bar" style="width: {maxFunnelCount ? (count / maxFunnelCount) * 100 : 0}%"></span>
              <span class="fn-count">{count}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </article>

    <!-- (c) anomaly counter -->
    <article class="card anomaly">
      <h3>Anomalies</h3>
      <p class="anomaly-count">{anomalyCount}</p>
      <p class="muted">Illegal status sequences detected (decide/fold Law).</p>
    </article>
  </div>
</section>

<style>
  .owner-dashboard {
    margin-top: 2rem;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
  }
  .card {
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 1rem;
    background: #fafafa;
  }
  .card h3 {
    margin: 0 0 0.75rem;
    font-size: 0.95rem;
  }
  .barlist,
  .funnel {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .ch-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    background: none;
    border: 1px solid transparent;
    border-radius: 6px;
    padding: 0.3rem 0.4rem;
    cursor: pointer;
    font: inherit;
    text-align: left;
  }
  .ch-row:hover {
    background: #f0f0f0;
  }
  .ch-row.selected {
    border-color: #888;
    background: #eceff1;
  }
  .ch-name {
    min-width: 80px;
    font-weight: 600;
  }
  .ch-bar,
  .fn-bar {
    flex: 1;
    height: 10px;
    background: #4f83cc;
    border-radius: 3px;
    min-width: 2px;
  }
  .ch-count,
  .fn-count {
    min-width: 28px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .funnel li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .fn-stage {
    min-width: 80px;
  }
  .anomaly {
    text-align: center;
  }
  .anomaly-count {
    font-size: 2.5rem;
    font-weight: 700;
    margin: 0.25rem 0;
    font-variant-numeric: tabular-nums;
  }
  .muted {
    color: #888;
    font-size: 0.8rem;
    margin: 0;
  }
</style>
