import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';
import localPlugin from 'eslint-plugin-local';

export default tseslint.config(
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
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
    },
  },
  {
    files: ['packages/ui/**', 'apps/web/**', 'tools/eslint-plugin-local/__fixtures__/**'],
    rules: {
      'local/no-hardcoded-color': 'warn',
      'local/no-hardcoded-tailwind-color': 'warn',
    },
  },
  {
    files: ['apps/api/src/routes/owner/**', 'apps/api/src/routes/courier/**'],
    rules: {
      'local/require-auth-hook': 'warn',
    },
  },
  {
    ignores: ['**/dist/**', '**/node_modules/**', '**/coverage/**', 'src/**', 'tools/eslint-plugin-local/__fixtures__/**'],
  },
  {
    files: ['tools/eslint-plugin-local/__fixtures__/**'],
    rules: {
      '@typescript-eslint/no-unused-vars': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
    },
  }
);
