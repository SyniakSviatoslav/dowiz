# Widget Loader

To embed the menu as a floating button or inline overlay:

```html
<script src="https://cdn.dowiz.org/widget.js" 
        data-slug=":slug" 
        data-mode="overlay" 
        integrity="sha384-...hash..." 
        crossorigin="anonymous"></script>
```

## Modes
- `data-mode="overlay"`: Spawns a floating button in the bottom right corner.
- `data-mode="inline"`: Renders a button precisely where the script tag is located.

## Security
- `integrity` is required! This enforces strict Subresource Integrity (SRI).
- The widget operates using a strict allowlisted CORS policy, omitting all cookies (`credentials: 'omit'`).
