import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';
import localPlugin from 'eslint-plugin-local';
import jsxA11y from 'eslint-plugin-jsx-a11y';

export default tseslint.config(
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  jsxA11y.flatConfigs.recommended,
  {
    plugins: {
      local: localPlugin,
    },
    rules: {
      // --- Existing rules (kept) ---
      'local/no-raw-sql': 'warn',
      'no-useless-escape': 'off',
      'no-empty': 'off',
      '@typescript-eslint/ban-ts-comment': 'off',
      'no-undef': 'off',
      '@typescript-eslint/no-require-imports': 'off',

      // --- Tightened rules ---
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
      '@typescript-eslint/no-explicit-any': 'warn',
      'prefer-const': 'warn',
      '@typescript-eslint/no-unused-expressions': 'warn',
      'no-misleading-character-class': 'warn',

      // --- New programmatic rules ---
      'local/no-ts-nocheck': 'warn',
      'local/no-raw-any': 'warn',
      'local/no-duplicate-import': 'warn',
      'local/no-empty-catch': 'warn',
      'local/no-mock-in-prod': 'warn',
      'local/no-permissive-status-assertion': 'warn',
      'local/no-hardcoded-string': 'warn',

      // --- Karpathy P2: Simplicity rules (warn-only, no CI block) ---
      'max-depth': ['warn', 4],
      'max-nested-callbacks': ['warn', 3],
      'max-params': ['warn', 4],
      'no-lonely-if': 'warn',
      'no-useless-rename': 'warn',
      'object-shorthand': 'warn',
      '@typescript-eslint/prefer-as-const': 'warn',
      '@typescript-eslint/prefer-optional-chain': 'off',
      '@typescript-eslint/prefer-nullish-coalescing': 'off',
    },
  },
  {
    files: ['packages/ui/**', 'apps/web/**', 'tools/eslint-plugin-local/__fixtures__/**'],
    rules: {
      'local/no-hardcoded-color': 'warn',
      'local/no-hardcoded-tailwind-color': 'warn',
      'local/no-hardcoded-string': 'warn',
    },
  },
  {
    files: ['apps/api/src/routes/owner/**', 'apps/api/src/routes/courier/**'],
    rules: {
      'local/require-auth-hook': 'warn',
    },
  },
  {
    ignores: ['**/dist/**', '**/node_modules/**', '**/coverage/**', 'src/**', '.opencode/**', '**/build-client.js', '**/churn-report.cjs', '.agents/skills/**', '.claude/skills/**'],
  },
  {
    files: ['tools/eslint-plugin-local/__fixtures__/**'],
    rules: {
      '@typescript-eslint/no-unused-vars': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/prefer-optional-chain': 'off',
      '@typescript-eslint/prefer-nullish-coalescing': 'off',
      'local/no-permissive-status-assertion': 'warn',
    },
  }
);
