---
name: screen-builder
description: >
  Use this skill when building full-page screens for DeliveryOS:
  client menu page, admin dashboard, branding settings, courier delivery,
  or admin menu management. Trigger phrases: "screen", "page", "build the",
  "create the dashboard", "menu page", "full layout", "/s/:slug", "/admin/",
  "/courier/delivery".
---

# DeliveryOS Screen Builder

## Goal
Produce complete, self-contained .html files for DeliveryOS screens
that work standalone in a browser without a build step.

## Required CDN links (include in every screen's <head>)
```html
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=DM+Serif+Display:ital@0;1&family=DM+Sans:wght@400;500;600&family=Cormorant+Garamond:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
<link href="https://cdn.jsdelivr.net/npm/@tabler/icons-webfont@latest/tabler-icons.min.css" rel="stylesheet">
<script src="https://cdn.tailwindcss.com"></script>
```

## File structure
Each screen must be a fully formed HTML5 document structured as:
1. `<!DOCTYPE html>` with `<html>`, `<head>`, `<body>`.
2. `<head>` contains CDN links, meta viewports, and `<style>` blocks populated with CSS custom properties (`:root` definitions for `var(--brand-...)`).
3. `<body>` contains the semantic HTML layout (e.g. `<header>`, `<main>`, `<footer>`).
4. Scripts to support interactivity should be placed at the bottom of the `<body>` element.
5. Do not include a build step or external local dependencies outside of the standardized token sets provided by the theme system.
