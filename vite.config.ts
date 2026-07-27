import { defineConfig } from 'vite';

// GAME-DESIGN 19.3: Die Entwicklungs-Werkzeuge stehen hinter __ROTA_DEVTOOLS__.
// In der Auslieferungs-Fassung ersetzt Vite die Konstante durch `false`; der
// Code dahinter fällt beim Tree-Shaking vollständig weg. Kein Byte landet beim
// Spieler — geprüft im CI-Job "build".
export default defineConfig(({ mode }) => ({
  base: '/rota/',
  define: {
    __ROTA_DEVTOOLS__: JSON.stringify(mode !== 'production'),
  },
  build: {
    target: 'es2022',
    outDir: 'dist',
    sourcemap: mode !== 'production',
    // Das Abhoer-Soundboard (Epic E8) ist ein Entwicklungs-Werkzeug und steht
    // deshalb nur in der Entwicklungs-Fassung im Bau — in der Auslieferung
    // existiert die Seite nicht (GAME-DESIGN 19.3).
    ...(mode === 'production'
      ? {}
      : {
          rollupOptions: {
            input: {
              main: 'index.html',
              soundboard: 'soundboard.html',
            },
          },
        }),
  },
  test: {
    include: ['tests/**/*.test.ts'],
    environment: 'node',
  },
}));
