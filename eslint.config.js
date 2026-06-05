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
      'local/no-raw-sql': 'error',
    },
  },
  {
    files: ['packages/ui/**', 'apps/web/**', 'tools/eslint-plugin-local/__fixtures__/**'],
    rules: {
      'local/no-hardcoded-color': 'error',
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
