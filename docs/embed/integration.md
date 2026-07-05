# Iframe Integration

To embed the menu on your website using a standard iframe:

```html
<iframe src="https://dowiz.org/s/:slug?embed=1" 
        style="width: 100%; border: 0;" 
        allow="geolocation 'self' https://dowiz.org" 
        title="Menu">
</iframe>
<script>
window.addEventListener('message', (e) => {
  if (e.data?.type === 'dowiz:resize' && e.data.slug === ':slug') {
    document.querySelector('iframe').style.height = e.data.height + 'px';
  }
});
</script>
```

## Security
- Make sure to update your `location_themes` to include your domain in `frame_ancestors` (e.g. `https://my-restaurant.com`), otherwise the browser will block the iframe.
- Embed mode automatically demotes `position: fixed` elements to prevent UX issues and disables PWA prompts.
