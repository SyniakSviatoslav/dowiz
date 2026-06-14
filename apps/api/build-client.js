import * as esbuild from 'esbuild';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import crypto from 'node:crypto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const outDir = path.join(__dirname, 'public', 'dist');

if (!fs.existsSync(outDir)) {
  fs.mkdirSync(outDir, { recursive: true });
}

// Ensure the pwa SW goes to public/sw.js instead of dist/
const swOutDir = path.join(__dirname, 'public');

const isDev = process.argv.includes('--watch');

async function build() {
  const options = {
    entryPoints: [
      path.join(__dirname, 'src', 'client', 'cart', 'app.ts'),
      path.join(__dirname, 'src', 'client', 'checkout', 'app.ts'),
      path.join(__dirname, 'src', 'client', 'status', 'app.ts'),
      path.join(__dirname, 'src', 'client', 'menu', 'app.ts')
    ],
    bundle: true,
    minify: !isDev,
    sourcemap: isDev,
    outdir: outDir,
    target: ['es2020'],
    format: 'esm'
  };

  const swOptions = {
    entryPoints: [
      path.join(__dirname, 'src', 'client', 'pwa', 'sw.ts')
    ],
    bundle: true,
    minify: !isDev,
    outfile: path.join(swOutDir, 'sw.js'),
    target: ['es2020'],
    format: 'iife'
  };

  const embedOptions = {
    entryPoints: [
      path.join(__dirname, 'src', 'client', 'embed', 'helper.ts')
    ],
    bundle: true,
    minify: !isDev,
    outfile: path.join(outDir, 'embed-helper.js'),
    target: ['es2020'],
    format: 'iife'
  };

  const widgetOptions = {
    entryPoints: [
      path.join(__dirname, 'src', 'client', 'widget', 'loader.ts')
    ],
    bundle: true,
    minify: !isDev,
    outfile: path.join(outDir, 'widget.js'),
    target: ['es2020'],
    format: 'iife'
  };

  try {
    if (isDev) {
      const ctx = await esbuild.context(options);
      await ctx.watch();
      const swCtx = await esbuild.context(swOptions);
      await swCtx.watch();
      const embedCtx = await esbuild.context(embedOptions);
      await embedCtx.watch();
      const widgetCtx = await esbuild.context(widgetOptions);
      await widgetCtx.watch();
      console.log('Watching client scripts for changes...');
    } else {
      await esbuild.build(options);
      await esbuild.build(swOptions);
      await esbuild.build(embedOptions);
      await esbuild.build(widgetOptions);

      // Compute SRI Hash for Widget
      const widgetContent = fs.readFileSync(path.join(outDir, 'widget.js'));
      const hash = crypto.createHash('sha384').update(widgetContent).digest('base64');
      fs.writeFileSync(path.join(outDir, 'widget.integrity.txt'), `sha384-${hash}`);
      console.log(`Widget compiled. SRI: sha384-${hash}`);
      
      console.log('Client scripts built successfully.');
    }
  } catch (err) {
    console.error('Build failed', err);
    process.exit(1);
  }
}

build();
