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
      'local/no-raw-sql': 'warn',
      '@typescript-eslint/no-unused-vars': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
      'no-useless-escape': 'off',
      'no-empty': 'off',
      '@typescript-eslint/ban-ts-comment': 'off',
      'prefer-const': 'off',
      'no-undef': 'off',
      '@typescript-eslint/no-unused-expressions': 'off',
      '@typescript-eslint/no-require-imports': 'off',
      'no-misleading-character-class': 'off'
    },
  },
  {
    files: ['packages/ui/**', 'apps/web/**', 'tools/eslint-plugin-local/__fixtures__/**'],
    rules: {
      'local/no-hardcoded-color': 'warn',
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
