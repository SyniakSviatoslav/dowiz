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
  },
};
