<script>
  export let status = 'pending';

  const labels = {
    pending: 'Pending', confirmed: 'Confirmed', preparing: 'Preparing',
    ready: 'Ready', 'in-delivery': 'In Delivery', delivered: 'Delivered',
    rejected: 'Rejected', cancelled: 'Cancelled', scheduled: 'Scheduled', 'picked-up': 'Picked Up'
  };

  $: cls = `badge badge-${status}`;
  $: hasPulse = status === 'pending' || status === 'preparing';
</script>

<span class={cls}>
  {#if hasPulse}
    <span class="status-dot {status}" />
  {/if}
  {labels[status] || status}
</span>

<style>
  .status-dot {
    display: inline-block; width: 8px; height: 8px;
    border-radius: 50%; margin-right: 4px;
  }
  .status-dot.pending { background: var(--status-pending); animation: pulse 2s infinite; }
  .status-dot.preparing { background: var(--status-preparing); animation: pulse 2s infinite; }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }
</style>
