const sqlKeywords = /\b(select|insert|update|delete|where|from|join|into|values)\b/i;

export default {
  rules: {
    'no-raw-sql': {
      meta: {
        type: 'problem',
        docs: {
          description: 'disallow raw SQL with interpolation',
        },
      },
      create(context) {
        return {
          TemplateLiteral(node) {
            if (node.expressions.length > 0) {
              const hasSqlKeyword = node.quasis.some(q => sqlKeywords.test(q.value.raw));
              if (hasSqlKeyword) {
                context.report({
                  node,
                  message: 'raw SQL with interpolation — use parameterized queries ($1, $2)',
                });
              }
            }
          },
          BinaryExpression(node) {
            if (node.operator === '+') {
              // Basic check for string concatenation containing SQL keywords
              let hasSql = false;
              if (node.left.type === 'Literal' && typeof node.left.value === 'string') {
                if (sqlKeywords.test(node.left.value)) hasSql = true;
              }
              if (node.right.type === 'Literal' && typeof node.right.value === 'string') {
                if (sqlKeywords.test(node.right.value)) hasSql = true;
              }
              
              if (hasSql) {
                 context.report({
                  node,
                  message: 'raw SQL with interpolation — use parameterized queries ($1, $2)',
                });
              }
            }
          }
        };
      },
    },
    'no-hardcoded-color': {
      meta: {
        type: 'problem',
        docs: {
          description: 'disallow hardcoded hex colors, require CSS variables',
        },
      },
      create(context) {
        const hexColorPattern = /#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})\b/;
        return {
          Literal(node) {
            if (typeof node.value === 'string' && hexColorPattern.test(node.value)) {
              context.report({
                node,
                message: 'hardcoded hex color — use var(--brand-*)',
              });
            }
          },
        };
      },
    },
  },
};
