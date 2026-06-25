const sqlKeywords = /\b(select|insert|update|delete|where|from|join|into|values)\b/i;
const hexColorPattern = /#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})\b/;
const twColorPattern = /\b(bg|text|border|ring|shadow|outline|accent|fill|stroke)-(red|blue|green|yellow|orange|purple|pink|gray|grey|white|black|slate|zinc|neutral|stone|amber|lime|emerald|teal|cyan|sky|indigo|violet|fuchsia|rose)-\d+\b/;

export default {
  rules: {
    'no-hardcoded-string': {
      meta: {
        type: 'problem',
        docs: { description: 'disallow hardcoded strings — use t(\'key\', \'fallback\')' },
      },
      create(context) {
        const filename = context.getFilename();
        const isTestFile = /\.(spec|test)\.(ts|js|tsx|jsx)$/.test(filename);
        if (isTestFile || filename.includes('node_modules') || filename.includes('.agents/')) return {};
        const allowedPatterns = [
          /^(data:|blob:)/, /^[<>{}[\]]$/, /^[\d%+\-/·\.]$/, /^[A-Z]{2,4}$/,
          /\{\{/, /^(var|calc|env)\(/, /^[a-z-]+$/, /^ti ti-/,
          /^(http|https|ftp):\/\//, /^(#|\/self|\*|none)/, /^(min|max)-/,
        ];
        return {
          Literal(node) {
            if (typeof node.value !== 'string') return;
            if (!node.value || node.value.length < 3) return;
            if (node.parent && node.parent.type === 'JSXText') return;
            if (node.parent && node.parent.type === 'JSXAttribute' && node.parent.name.name === 'className') return;
            if (node.parent && node.parent.type === 'CallExpression' && node.parent.callee.name === 't') return;
            if (node.parent && node.parent.type === 'ImportDeclaration') return;
            if (node.parent && node.parent.type === 'ExportNamedDeclaration') return;
            if (node.parent && node.parent.type === 'ExportDefaultDeclaration') return;
            if (allowedPatterns.some(p => p.test(node.value))) return;
            const isPrintable = /^[\x20-\x7E\s]+$/.test(node.value);
            if (!isPrintable) return;
            if (node.parent && node.parent.type === 'TemplateLiteral') return;
            if (/^(px|py|mx|my|gap|w|h|text|leading|tracking|rounded|border|shadow|z|top|right|bottom|left|opacity|scale|rotate|translate|space|inset|flex|grid|col|row|order)-/.test(node.value)) return;
            if (node.parent && node.parent.type === 'Property' && node.parent.key === node) return;
            if (node.parent && node.parent.type === 'JSXExpressionContainer' &&
                node.parent.parent.type === 'JSXAttribute') return;
            context.report({ node, message: `hardcoded string "${node.value.substring(0, 40)}" — use t('key', 'fallback')` });
          },
        };
      },
    },
    'no-raw-sql': {
      meta: {
        type: 'problem',
        docs: { description: 'disallow raw SQL with interpolation' },
      },
      create(context) {
        return {
          TemplateLiteral(node) {
            if (node.expressions.length > 0) {
              const hasSqlKeyword = node.quasis.some(q => sqlKeywords.test(q.value.raw));
              if (hasSqlKeyword) {
                context.report({ node, message: 'raw SQL with interpolation — use parameterized queries ($1, $2)' });
              }
            }
          },
          BinaryExpression(node) {
            if (node.operator === '+') {
              let hasSql = false;
              if (node.left.type === 'Literal' && typeof node.left.value === 'string' && sqlKeywords.test(node.left.value)) hasSql = true;
              if (node.right.type === 'Literal' && typeof node.right.value === 'string' && sqlKeywords.test(node.right.value)) hasSql = true;
              if (hasSql) context.report({ node, message: 'raw SQL with interpolation — use parameterized queries ($1, $2)' });
            }
          }
        };
      },
    },
    'no-hardcoded-color': {
      meta: {
        type: 'problem',
        docs: { description: 'disallow hardcoded hex colors in CSS/JS, require CSS variables' },
      },
      create(context) {
        return {
          Literal(node) {
            if (typeof node.value === 'string' && hexColorPattern.test(node.value)) {
              context.report({ node, message: 'hardcoded hex color — use var(--brand-*) or var(--color-*)' });
            }
          },
        };
      },
    },
    'no-hardcoded-tailwind-color': {
      meta: {
        type: 'problem',
        docs: { description: 'disallow hardcoded Tailwind color classes, require CSS variable equivalents' },
      },
      create(context) {
        return {
          JSXAttribute(node) {
            if (node.value && node.value.type === 'Literal' && typeof node.value.value === 'string') {
              const val = node.value.value;
              if (twColorPattern.test(val)) {
                context.report({ node, message: 'hardcoded Tailwind color — use CSS variable classes or inline var()' });
              }
            }
          },
          Literal(node) {
            if (typeof node.value === 'string' && twColorPattern.test(node.value)) {
              context.report({ node, message: 'hardcoded Tailwind color — use CSS variable classes or inline var()' });
            }
          },
        };
      },
    },
    // UI Build-Verification Loop · Layer 1 (author-time rail): arbitrary Tailwind values
    // (p-[13px], w-[100px], text-[#fff], gap-[7px]) bypass the design scale/tokens → off-rhythm
    // spacing + off-token colour, the visually-wrong-but-valid class this loop exists to catch.
    // Use scale classes (p-3, w-24) or var(--brand-*) instead. Warn-level design-drift signal.
    'no-arbitrary-tailwind': {
      meta: {
        type: 'problem',
        docs: { description: 'disallow arbitrary Tailwind bracket values; use the design scale / tokens' },
      },
      create(context) {
        const arb = /\b(p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|ml|mr|w|h|min-w|max-w|min-h|max-h|gap|space-x|space-y|text|leading|tracking|rounded|top|bottom|left|right|inset|z|translate-x|translate-y)-\[[^\]]+\]/;
        const check = (node, val) => {
          if (typeof val === 'string' && arb.test(val)) {
            context.report({ node, message: 'arbitrary Tailwind value — use a design-scale class (p-3, w-24) or var(--brand-*), not p-[13px]/text-[#fff]' });
          }
        };
        return {
          JSXAttribute(node) {
            if (node.value && node.value.type === 'Literal') check(node, node.value.value);
          },
          Literal(node) { check(node, node.value); },
        };
      },
    },
    // Focused ERROR rail (the broad no-arbitrary-tailwind above is warn-level and also fires on
    // intentional token brackets like text-[var(--brand-text)]). This one targets ONLY arbitrary
    // FONT SIZES — text-[10px] / text-[1.5rem] — which must use the type scale (text-step-2xs..3xl
    // → --text-* tokens). Zero violations after the Phase-B migration; error-level locks the win.
    'no-arbitrary-font-size': {
      meta: {
        type: 'problem',
        docs: { description: 'disallow arbitrary font sizes; use text-step-* / --text-* scale' },
      },
      create(context) {
        const sizeArb = /\btext-\[\d+(?:\.\d+)?(?:px|rem|em)\]/;
        const check = (node, val) => {
          if (typeof val === 'string' && sizeArb.test(val)) {
            context.report({ node, message: 'arbitrary font size — use the type scale (text-step-2xs / text-step-xs … text-step-3xl), not bracketed pixels' });
          }
        };
        return {
          JSXAttribute(node) { if (node.value && node.value.type === 'Literal') check(node, node.value.value); },
          Literal(node) { check(node, node.value); },
          TemplateElement(node) { check(node, node.value && node.value.raw); },
        };
      },
    },
    'no-ts-nocheck': {
      meta: {
        type: 'problem',
        docs: { description: 'disallow @ts-nocheck — fix types properly' },
      },
      create(context) {
        return {
          Program(node) {
            const sourceCode = context.getSourceCode();
            const comments = sourceCode.getAllComments();
            for (const comment of comments) {
              if (comment.type === 'Block' && comment.value.trim() === '@ts-nocheck') {
                context.report({ node: comment, message: '@ts-nocheck disables type checking — fix the types instead' });
              }
            }
          },
        };
      },
    },
    'no-raw-any': {
      meta: {
        type: 'problem',
        docs: { description: 'warn on `as any` type assertions' },
      },
      create(context) {
        return {
          TSAsExpression(node) {
            if (node.typeAnnotation && node.typeAnnotation.type === 'TSAnyKeyword') {
              context.report({ node, message: '`as any` disables type safety — use a proper type' });
            }
          },
        };
      },
    },
    'no-duplicate-import': {
      meta: {
        type: 'problem',
        docs: { description: 'disallow importing the same module multiple times' },
      },
      create(context) {
        const imports = new Map();
        return {
          ImportDeclaration(node) {
            const source = node.source.value;
            if (imports.has(source)) {
              context.report({ node, message: `duplicate import from '${source}' — merge with existing import` });
            }
            imports.set(source, node);
          },
        };
      },
    },
    'require-auth-hook': {
      meta: {
        type: 'problem',
        docs: { description: 'warn on owner/courier route files that lack auth hooks' },
      },
      create(context) {
        const filename = context.getFilename();
        const isOwnerRoute = /\/routes\/owner\//.test(filename);
        const isCourierRoute = /\/routes\/courier\//.test(filename);
        if (!isOwnerRoute && !isCourierRoute) return {};

        let hasAuthHook = false;
        return {
          CallExpression(node) {
            if (node.callee.type === 'MemberExpression' &&
                node.callee.property.name === 'addHook') {
              const args = node.arguments;
              if (args.length >= 2 && args[1].type === 'Identifier' &&
                  (args[1].name === 'verifyAuth' || args[1].name === 'requireRole')) {
                hasAuthHook = true;
              }
            }
            // Check for preHandler/preValidation with auth
            if (node.callee.type === 'MemberExpression' &&
                node.callee.property.name === 'register' &&
                node.arguments.length >= 1) {
              // Inline route definitions with preHandler
              const opts = node.arguments[1];
              if (opts && opts.type === 'ObjectExpression') {
                for (const prop of opts.properties) {
                  if (prop.key && (prop.key.name === 'preHandler' || prop.key.name === 'preValidation')) {
                    hasAuthHook = true;
                  }
                }
              }
            }
          },
          'Program:exit'(node) {
            if (!hasAuthHook) {
              context.report({ node, message: `${isOwnerRoute ? 'owner' : 'courier'} route file lacks auth hook — add verifyAuth + requireRole` });
            }
          },
        };
      },
    },
    'no-empty-catch': {
      meta: {
        type: "problem",
        docs: { description: 'disallow empty catch blocks' },
      },
      create(context) {
        return {
          CatchClause(node) {
            if (node.body.body.length === 0) {
              context.report({ node, message: 'empty catch block — log the error or add a comment explaining why it is ignored' });
            }
          },
        };
      },
    },
    'no-process-exit': {
      meta: {
        type: "problem",
        docs: { description: 'disallow process.exit() in library code — throw instead' },
      },
      create(context) {
        return {
          CallExpression(node) {
            if (node.callee.type === 'MemberExpression' &&
                node.callee.object.name === 'process' &&
                node.callee.property.name === 'exit') {
              const filename = context.getFilename();
              if (!filename.includes('scripts/') && !filename.includes('server.ts')) {
                context.report({ node, message: 'process.exit() in non-entry code — throw an error instead' });
              }
            }
          },
        };
      },
    },
    'no-permissive-status-assertion': {
      meta: {
        type: "problem",
        docs: { description: 'disallow expect([200,400,...]).toContain(x) — use expect(x).toBe(N) for exact status' },
        fixable: "code",
        schema: [],
      },
      create(context) {
        const filename = context.getFilename();
        const isTestFile = /\.(spec|test)\.(ts|js|tsx|jsx)$/.test(filename);
        if (!isTestFile) return {};

        function isNumericArray(node) {
          return node.type === 'ArrayExpression'
            && node.elements.length > 0
            && node.elements.every(e => e && e.type === 'Literal' && typeof e.value === 'number');
        }

        return {
          CallExpression(node) {
            if (node.callee.type !== 'MemberExpression') return;
            const method = node.callee.property;
            if (method.type !== 'Identifier' || method.name !== 'toContain') return;
            if (node.callee.object.type !== 'CallExpression') return;
            const expectCall = node.callee.object;
            if (expectCall.callee.type !== 'Identifier' || expectCall.callee.name !== 'expect') return;
            const arg = expectCall.arguments[0];
            if (!arg || !isNumericArray(arg)) return;
            const statuses = arg.elements.map((e) => e.value).join(', ');
            context.report({
              node,
              message: `Permissive status assertion expect([${statuses}]).toContain(x) — use expect(x).toBe(N) for exact expected status. Each status is a separate test case.`,
            });
          },
        };
      },
    },
    'no-mock-in-prod': {
      meta: {
        type: "problem",
        docs: { description: 'warn on mock/test data in production code paths' },
      },
      create(context) {
        return {
          VariableDeclarator(node) {
            if (node.id.type === 'Identifier' && /mock|fake|stub|dummy/i.test(node.id.name)) {
              const filename = context.getFilename();
              if (!filename.includes('test') && !filename.includes('spec') && !filename.includes('__fixtures__') && !filename.includes('dev/')) {
                context.report({ node, message: 'mock/test variable in production code — move to test fixtures' });
              }
            }
          },
        };
      },
    },
    // Ratchet (Regression Ledger): Math.random() for a security-sensitive value
    // (token / otp / secret / nonce / session id / password / salt / csrf / api key /
    // verification / recovery code) is predictable. Recurrent: dev-login backdoor +
    // auth-token class (ADR-0003). Require crypto.randomUUID() / crypto.randomBytes() /
    // crypto.randomInt() / crypto.getRandomValues(). Narrowed to security identifiers so
    // legitimate Math.random() jitter / animation / toast-id stays green.
    'no-insecure-random': {
      meta: {
        type: 'problem',
        docs: { description: 'disallow Math.random() for security-sensitive values — use crypto.*' },
      },
      create(context) {
        const securityName = /(token|otp|secret|nonce|session|passw(or)?d|salt|csrf|verif|api[_-]?key|apikey|recovery|reset[_-]?code|magic)/i;

        function isMathRandom(node) {
          return node
            && node.type === 'CallExpression'
            && node.callee.type === 'MemberExpression'
            && node.callee.object.type === 'Identifier'
            && node.callee.object.name === 'Math'
            && node.callee.property.type === 'Identifier'
            && node.callee.property.name === 'random';
        }

        // True if a Math.random() call appears anywhere within an expression subtree
        // (covers `Math.random().toString(36)`, `prefix + Math.random()*1e9`, etc.).
        function containsMathRandom(node) {
          if (!node || typeof node !== 'object') return false;
          if (isMathRandom(node)) return true;
          if (node.type === 'CallExpression') {
            if (containsMathRandom(node.callee)) return true;
            for (const a of node.arguments) if (containsMathRandom(a)) return true;
            return false;
          }
          if (node.type === 'MemberExpression') {
            return containsMathRandom(node.object) || containsMathRandom(node.property);
          }
          if (node.type === 'BinaryExpression' || node.type === 'LogicalExpression') {
            return containsMathRandom(node.left) || containsMathRandom(node.right);
          }
          if (node.type === 'TemplateLiteral') {
            return node.expressions.some(containsMathRandom);
          }
          if (node.type === 'UnaryExpression') return containsMathRandom(node.argument);
          if (node.type === 'ConditionalExpression') {
            return containsMathRandom(node.consequent) || containsMathRandom(node.alternate);
          }
          return false;
        }

        return {
          VariableDeclarator(node) {
            if (node.id.type !== 'Identifier' || !securityName.test(node.id.name)) return;
            if (containsMathRandom(node.init)) {
              context.report({
                node,
                message: `Math.random() used for security-sensitive value "${node.id.name}" — use crypto.randomUUID() / crypto.randomBytes() / crypto.randomInt() / crypto.getRandomValues()`,
              });
            }
          },
          AssignmentExpression(node) {
            const left = node.left;
            const name = left.type === 'Identifier'
              ? left.name
              : (left.type === 'MemberExpression' && left.property.type === 'Identifier' ? left.property.name : null);
            if (!name || !securityName.test(name)) return;
            if (containsMathRandom(node.right)) {
              context.report({
                node,
                message: `Math.random() used for security-sensitive value "${name}" — use crypto.randomUUID() / crypto.randomBytes() / crypto.randomInt() / crypto.getRandomValues()`,
              });
            }
          },
        };
      },
    },
    // Ratchet (Regression Ledger): in the frontend, every WebSocket must go through the
    // single shared client (apps/web useWebSocket.ts) which owns reconnect-jitter +
    // ordered-frame handling. Recurrent: out-of-order WS frames + reconnect bugs. A second
    // `new WebSocket(...)` in a component re-introduces them. (F14: the old capped
    // packages/ui/src/lib/websocket.ts client was deleted — there is now exactly one.)
    // Scoped to apps/web + packages/ui/src and excludes the one shared-client file.
    'no-direct-websocket': {
      meta: {
        type: 'problem',
        docs: { description: 'disallow new WebSocket() in frontend components — use the shared WS client' },
      },
      create(context) {
        const filename = context.getFilename().replace(/\\/g, '/');
        const inFrontend = /\/apps\/web\//.test(filename)
          || /\/packages\/ui\/src\//.test(filename)
          || /\/__fixtures__\//.test(filename); // fixtures exercise the rule for the red→green proof
        if (!inFrontend) return {};
        // The shared client is the one allowed place to construct a raw WebSocket.
        const isSharedClient = /\/apps\/web\/src\/lib\/useWebSocket\.tsx?$/.test(filename);
        if (isSharedClient) return {};
        const isTestFile = /\.(spec|test)\.(ts|js|tsx|jsx)$/.test(filename);
        if (isTestFile) return {};

        return {
          NewExpression(node) {
            if (node.callee.type === 'Identifier' && node.callee.name === 'WebSocket') {
              context.report({
                node,
                message: 'direct `new WebSocket()` in a frontend component — use the shared WS client (useWebSocket) so reconnect + frame-ordering stay centralized',
              });
            }
          },
        };
      },
    },
  },
};
