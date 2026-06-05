import { h } from 'preact';
import render from 'preact-render-to-string';
import htm from 'htm';

const html = htm.bind(h);

export interface ShellProps {
  title: string;
  slug: string;
  scriptUrl: string;
  nonce: string;
  cssHash?: string;
  themeVersion?: number;
  locationId?: string;
}

export function renderClientShell({ title, slug, scriptUrl, nonce, cssHash, themeVersion, locationId }: ShellProps): string {
  const vdom = html`
    <html lang="en">
      <head>
        <meta charset="utf-8" />
        <title>${title}</title>
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <meta name="robots" content="noindex, nofollow" />
        <meta name="dos-slug" content="${slug}" />
        ${locationId ? html`<meta name="dos-location-id" content="${locationId}" />` : null}
        <link rel="manifest" href="/s/${slug}/manifest.webmanifest" />
        <link rel="apple-touch-icon" href="https://cdn.dowiz.org/locations/default/logo-192.png" />
        <link href="https://cdn.jsdelivr.net/npm/@tabler/icons-webfont@latest/tabler-icons.min.css" rel="stylesheet" />
        <script src="https://cdn.tailwindcss.com"></script>
        ${cssHash ? html`<link rel="stylesheet" href="/public/locations/${locationId}/theme.css?hash=${cssHash}&v=${themeVersion}" />` : null}
        <style nonce="${nonce}">
          :root {
            --brand-primary: #e63946;
            --brand-bg: #f8f9fa;
            --brand-text: #212529;
          }
          body {
            font-family: system-ui, sans-serif;
            margin: 0;
            padding: 0;
            background: var(--brand-bg);
            color: var(--brand-text);
          }
          .container {
            max-width: 800px;
            margin: 0 auto;
            padding: 1rem;
          }
        </style>
      </head>
      <body>
        <div id="app" class="container">
          <p>Loading...</p>
        </div>
        
        <script>
          if ('serviceWorker' in navigator) {
            window.addEventListener('load', () => {
              navigator.serviceWorker.register('/sw.js').catch(err => {
                console.error('SW registration failed', err);
              });
            });
          }
        </script>
        <script type="module" src="${scriptUrl}"></script>
      </body>
    </html>
  `;

  return '<!DOCTYPE html>\n' + render(vdom);
}
