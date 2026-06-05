// Widget Loader
// Intended to be included via <script src=".../widget.js" data-slug="my-loc" integrity="..." crossorigin="anonymous"></script>

(function() {
  const currentScript = document.currentScript;
  if (!currentScript) {
    console.error('Dowiz: Cannot find current script tag');
    return;
  }

  const slug = currentScript.getAttribute('data-slug');
  if (!slug) {
    console.error('Dowiz: Missing data-slug attribute on widget script');
    return;
  }

  const host = new URL(currentScript.getAttribute('src') || 'https://dowiz.org').origin;
  
  // Inject some base styles for the overlay
  const style = document.createElement('style');
  style.textContent = `
    .dowiz-widget-btn {
      display: inline-block;
      padding: 12px 24px;
      background-color: var(--brand-primary, #e63946);
      color: #fff;
      border-radius: 8px;
      font-weight: bold;
      cursor: pointer;
      border: none;
      font-family: system-ui, sans-serif;
      transition: background 0.2s;
    }
    .dowiz-widget-btn:hover {
      opacity: 0.9;
    }
    .dowiz-overlay {
      position: fixed;
      inset: 0;
      z-index: 2147483647;
      background: rgba(0,0,0,0.6);
      display: none;
      align-items: center;
      justify-content: center;
      padding: 20px;
    }
    .dowiz-overlay.open {
      display: flex;
    }
    .dowiz-iframe-container {
      width: 100%;
      max-width: 600px;
      height: 90vh;
      background: #fff;
      border-radius: 12px;
      overflow: hidden;
      position: relative;
      display: flex;
      flex-direction: column;
    }
    .dowiz-iframe-container iframe {
      flex: 1;
      width: 100%;
      border: none;
    }
    .dowiz-close-btn {
      position: absolute;
      top: -40px;
      right: 0;
      background: none;
      border: none;
      color: #fff;
      font-size: 30px;
      cursor: pointer;
    }
  `;
  document.head.appendChild(style);

  // The floating or inline button
  const btn = document.createElement('button');
  btn.className = 'dowiz-widget-btn';
  btn.textContent = 'Order Now';

  // The overlay
  const overlay = document.createElement('div');
  overlay.className = 'dowiz-overlay';
  overlay.innerHTML = `
    <div class="dowiz-iframe-container">
      <button class="dowiz-close-btn">&times;</button>
      <!-- iframe injected on open -->
    </div>
  `;

  document.body.appendChild(overlay);

  // Mount logic
  const mode = currentScript.getAttribute('data-mode') || 'inline';
  if (mode === 'inline') {
    currentScript.parentNode?.insertBefore(btn, currentScript.nextSibling);
  } else {
    // Floating
    btn.style.position = 'fixed';
    btn.style.bottom = '20px';
    btn.style.right = '20px';
    btn.style.zIndex = '2147483646';
    document.body.appendChild(btn);
  }

  // Interaction
  let iframeMounted = false;
  btn.addEventListener('click', () => {
    if (!iframeMounted) {
      const container = overlay.querySelector('.dowiz-iframe-container');
      const iframe = document.createElement('iframe');
      iframe.src = `${host}/s/${slug}?embed=1&widget=1`;
      iframe.allow = "geolocation 'self' https://dowiz.org";
      container?.appendChild(iframe);
      iframeMounted = true;
    }
    overlay.classList.add('open');
    document.body.style.overflow = 'hidden'; // prevent background scroll
  });

  const closeBtn = overlay.querySelector('.dowiz-close-btn');
  closeBtn?.addEventListener('click', () => {
    overlay.classList.remove('open');
    document.body.style.overflow = '';
  });

  // Optional: fetch menu basic info to populate text/color if needed
  // Using credentials: 'omit' for strict CORS adherence
  fetch(`${host}/public/locations/${slug}/menu`, { credentials: 'omit' })
    .then(res => res.json())
    .catch(console.error);

})();
