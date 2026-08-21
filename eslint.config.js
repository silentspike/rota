import js from '@eslint/js';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  {
    ignores: [
      'dist/**',
      'target/**', // Rust-Bauartefakte; `cargo doc` legt dort JS ab
      'node_modules/**',
      'sim/**', // Rust — hat eigene Prüfung (clippy, rustfmt)
      'docs/**',
      // tools/ ist kein Spiel-Code und wird nie gebündelt: Python, Shell und
      // ein Snippet, das als String in eine fremde Seite injiziert wird
      // (tools/suno-hook.js ist deshalb absichtlich ein blosser Ausdruck).
      'tools/**',
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    languageOptions: {
      globals: {
        // Von Vite zur Bauzeit ersetzt (siehe vite.config.ts)
        __ROTA_DEVTOOLS__: 'readonly',
        window: 'readonly',
        document: 'readonly',
        performance: 'readonly',
        console: 'readonly',
      },
    },
    rules: {
      // Im heißen Pfad wird bewusst kein Objekt erzeugt (GAME-DESIGN 18.2).
      // Diese Regel bleibt an, damit versehentliche Allokationen auffallen.
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
    },
  },
);
