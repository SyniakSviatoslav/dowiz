/**
 * ccc indexer (ADR-0012 C1) — ignore-aware AST symbol extraction.
 *
 * The walker calls `ignore.isIgnored(relPath)` BEFORE `readFileSync` (B10), so a secret is never
 * opened. `readPaths` records every file whose bytes were read — the secret-scan merge gate
 * asserts no secret/ignored path appears there, proving the ordering, not just the output.
 *
 * Lightweight by design (ADR-0012): the TypeScript compiler API parses each source file into an
 * AST and we extract top-level + exported symbols (functions, classes, methods, interfaces, types,
 * arrow/const exports) with a one-line signature and location. No pgvector, no daemon, no embeddings.
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import ts from 'typescript';
import { loadIgnore, type IgnoreRules } from './ignore.js';

export interface Symbol {
  file: string; // POSIX-relative path
  line: number; // 1-based
  kind: string; // function | class | method | interface | type | const | enum
  name: string;
  signature: string; // trimmed one-line declaration
  exported: boolean;
}

export interface Index {
  root: string;
  generatedFrom: string; // commit/sha or label, supplied by caller (Date.now is unavailable here)
  symbols: Symbol[];
}

const SOURCE_EXT = /\.(ts|tsx|js|jsx|mjs|cjs)$/;
const DECL = /\.d\.ts$/;

function toPosix(p: string): string {
  return sep === '/' ? p : p.split(sep).join('/');
}

/** Recursively collect indexable source files, consulting ignore rules BEFORE reading. */
export function collectFiles(
  root: string,
  ignore: IgnoreRules,
  readPaths: string[],
): string[] {
  const out: string[] = [];
  const walk = (dir: string) => {
    let entries: string[];
    try {
      entries = readdirSync(dir);
    } catch {
      return;
    }
    for (const name of entries) {
      const abs = join(dir, name);
      const rel = toPosix(relative(root, abs));
      // ── ignore check happens BEFORE any stat/read of file content ──
      if (ignore.isIgnored(rel)) continue;
      let st;
      try {
        st = statSync(abs); // metadata only — not the file's bytes
      } catch {
        continue;
      }
      if (st.isDirectory()) {
        walk(abs);
      } else if (SOURCE_EXT.test(name) && !DECL.test(name)) {
        out.push(abs);
      }
    }
  };
  walk(root);
  return out;
}

function lineOf(sf: ts.SourceFile, pos: number): number {
  return sf.getLineAndCharacterOfPosition(pos).line + 1;
}

function firstLine(text: string): string {
  const line = text.split('\n')[0].trim();
  return line.length > 200 ? line.slice(0, 197) + '…' : line;
}

function isExported(node: ts.Node): boolean {
  const mods = (ts.canHaveModifiers(node) ? ts.getModifiers(node) : undefined) ?? [];
  return mods.some((m) => m.kind === ts.SyntaxKind.ExportKeyword);
}

/** Extract symbols from one already-read source file. */
export function extractSymbols(relFile: string, content: string): Symbol[] {
  const sf = ts.createSourceFile(relFile, content, ts.ScriptTarget.Latest, true);
  const symbols: Symbol[] = [];
  const push = (name: string | undefined, kind: string, node: ts.Node, exported: boolean) => {
    if (!name) return;
    symbols.push({
      file: relFile,
      line: lineOf(sf, node.getStart(sf)),
      kind,
      name,
      signature: firstLine(node.getText(sf)),
      exported,
    });
  };
  const visit = (node: ts.Node) => {
    if (ts.isFunctionDeclaration(node)) push(node.name?.text, 'function', node, isExported(node));
    else if (ts.isClassDeclaration(node)) {
      push(node.name?.text, 'class', node, isExported(node));
      for (const m of node.members) {
        if (ts.isMethodDeclaration(m) && m.name && ts.isIdentifier(m.name)) push(m.name.text, 'method', m, false);
      }
    } else if (ts.isInterfaceDeclaration(node)) push(node.name.text, 'interface', node, isExported(node));
    else if (ts.isTypeAliasDeclaration(node)) push(node.name.text, 'type', node, isExported(node));
    else if (ts.isEnumDeclaration(node)) push(node.name.text, 'enum', node, isExported(node));
    else if (ts.isVariableStatement(node)) {
      const exported = isExported(node);
      for (const d of node.declarationList.declarations) {
        if (ts.isIdentifier(d.name) && d.initializer &&
          (ts.isArrowFunction(d.initializer) || ts.isFunctionExpression(d.initializer))) {
          push(d.name.text, 'const', d, exported);
        }
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sf);
  return symbols;
}

/** Build the full index for `root`. `generatedFrom` is a caller-supplied label (no clock here). */
export function buildIndex(root: string, generatedFrom: string): { index: Index; readPaths: string[] } {
  const ignore = loadIgnore(root);
  const readPaths: string[] = [];
  const files = collectFiles(root, ignore, readPaths);
  const symbols: Symbol[] = [];
  for (const abs of files) {
    const rel = toPosix(relative(root, abs));
    // Final guard: re-check ignore immediately before the ONLY readFileSync of content (B10).
    if (ignore.isIgnored(rel)) continue;
    let content: string;
    try {
      content = readFileSync(abs, 'utf8');
    } catch {
      continue;
    }
    readPaths.push(rel); // recorded only AFTER a real content read
    symbols.push(...extractSymbols(rel, content));
  }
  return { index: { root, generatedFrom, symbols }, readPaths };
}
