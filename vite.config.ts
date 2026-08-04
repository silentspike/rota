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
              // Prüfblatt zu Issue #85. Es liegt bewusst hier und nicht hinter
              // einer nginx-Location: Eine Location in /etc/nginx/ zeigt auf
              // einen Worktree im gerade ausgecheckten Branch und ist damit
              // Evidence, die nur auf einem Rechner funktioniert.
              archiveCheck: 'docs/archive-check/index.html',
              trainingCheck: 'docs/training-check/index.html',
              // Die Prüfblätter aus Epic E7. Sie liegen bewusst hier und nicht
              // hinter einer nginx-Location: Eine Location in /etc/nginx/ zeigt
              // auf einen Worktree im gerade ausgecheckten Branch und ist damit
              // Evidence, die nur auf einem Rechner funktioniert. Als
              // Eingangsseite sieht jeder dasselbe, der den Entwicklungs-Bau
              // ausführt.
              saveCheck: 'docs/save-check/index.html',
              analysisCheck: 'docs/analysis-check/index.html',
            },
          },
        }),
  },
  test: {
    include: ['tests/**/*.test.ts'],
    environment: 'node',
  },
}));
