import * as esbuild from 'esbuild';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
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

// Compile Tailwind to a static, purged stylesheet served at /dist/tailwind.css,
// replacing the runtime cdn.tailwindcss.com script on the client shells + admin
// pages. Content is scanned from src/client, src/public/admin, and the shell.
function buildTailwind() {
  const bin = path.join(__dirname, 'node_modules', '.bin', 'tailwindcss');
  const args = [
    '-c', path.join(__dirname, 'tailwind.config.cjs'),
    '-i', path.join(__dirname, 'src', 'styles', 'tailwind.css'),
    '-o', path.join(outDir, 'tailwind.css'),
  ];
  if (!isDev) args.push('--minify');
  execFileSync(bin, args, { stdio: 'inherit', cwd: __dirname });
  console.log('Tailwind CSS built -> public/dist/tailwind.css');
}

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
      buildTailwind();
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

      buildTailwind();
      console.log('Client scripts built successfully.');
    }
  } catch (err) {
    console.error('Build failed', err);
    process.exit(1);
  }
}

build();
