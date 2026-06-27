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
      'local/no-tautological-assertion': 'warn',
      'local/no-swallowed-catch': 'warn',
      'local/no-truthy-on-identifier': 'warn',
      'local/no-prod-base-in-test': 'warn',
      'local/no-hardcoded-string': 'warn',
      'local/no-insecure-random': 'warn',
      'local/no-direct-websocket': 'warn',
      'local/no-arbitrary-tailwind': 'warn',
      // Phase-B type-scale: error-level (zero violations after migration → locks the win).
      'local/no-arbitrary-font-size': 'error',

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
    // The API-served standalone widgets (checkout/active-delivery/etc.) ship their own CDN
    // Tailwind WITHOUT the project's text-step-* scale, so arbitrary text-[Npx] is legitimate there.
    files: ['apps/api/src/client/**', 'apps/api/src/public/**', 'tools/eslint-plugin-local/**'],
    rules: {
      'local/no-arbitrary-font-size': 'off',
    },
  },
  {
    // React app must use the shared <Select>/<Textarea>, not native form controls. Error-level here
    // only — the packages/ui atoms wrap the native elements, and apps/api widgets are out of scope.
    files: ['apps/web/src/**/*.tsx'],
    rules: {
      'local/no-raw-form-control': 'error',
    },
  },
  {
    ignores: ['**/dist/**', '**/node_modules/**', '**/coverage/**', 'src/**', '.opencode/**', '**/build-client.js', '**/churn-report.cjs', '.agents/skills/**', '.claude/skills/**', 'apps/api/public/assets/**', 'apps/api/public/dist/**'],
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
