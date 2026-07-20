/** Tailwind build for the server-rendered client shells (cart/checkout/status)
 *  and the static admin HTML pages. Output: public/dist/tailwind.css, served
 *  at /dist/tailwind.css — replaces the runtime cdn.tailwindcss.com script. */
module.exports = {
  content: [
    './src/client/**/*.{ts,js}',
    './src/public/admin/**/*.html',
    './src/lib/ssr-client-renderer.ts',
  ],
  theme: { extend: {} },
};
