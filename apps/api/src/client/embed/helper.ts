// @ts-nocheck
// Helper for iframe embed mode to sync height with parent

function postHeight() {
  const h = document.documentElement.scrollHeight;
  const slugMeta = document.querySelector('meta[name="dowiz:slug"]');
  const slug = slugMeta ? slugMeta.getAttribute('content') : '';

  window.parent.postMessage({ 
    type: 'dowiz:resize', 
    height: h, 
    slug 
  }, '*'); // Parent origin could be anything if they didn't specify, but ideally should be restricted by frame-ancestors
}

if (typeof window !== 'undefined') {
  const ro = new ResizeObserver(postHeight);
  ro.observe(document.body);
  window.addEventListener('load', postHeight);
  // Safety net
  setInterval(postHeight, 5000);
}
