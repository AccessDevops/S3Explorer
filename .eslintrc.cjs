/* eslint-env node */
require('@rushstack/eslint-patch/modern-module-resolution')

module.exports = {
  root: true,
  extends: [
    'plugin:vue/vue3-essential',
    'eslint:recommended',
    '@vue/eslint-config-typescript',
    '@vue/eslint-config-prettier/skip-formatting'
  ],
  parserOptions: {
    ecmaVersion: 'latest'
  },
  rules: {
    // Allow single-word component names for UI library components
    'vue/multi-word-component-names': ['error', {
      ignores: ['Badge', 'Button', 'Card', 'Dialog', 'Input', 'Separator', 'Table']
    }],
    // Disable unused vars warning for props destructuring (Vue 3 Composition API)
    '@typescript-eslint/no-unused-vars': ['error', {
      argsIgnorePattern: '^_',
      varsIgnorePattern: '^_',
      destructuredArrayIgnorePattern: '^_'
    }]
  },
  overrides: [
    {
      files: ['tailwind.config.js', '*.config.js'],
      env: {
        node: true
      }
    }
  ]
}
