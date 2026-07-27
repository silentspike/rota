import './render/page.css';
import './credits/credits.css';
import { App } from './core/app.js';
import { installDevTools } from './devtools/index.js';
import { CreditsView } from './credits/credits.js';
import { CREDITS } from './credits/credits-data.js';

const root = document.querySelector<HTMLDivElement>('#app');
if (root === null) {
  throw new Error('#app fehlt in index.html');
}

const app = new App(root);
installDevTools(app);
app.start();

// Namensnennung ist Lizenzauflage der Zauber-Symbole (CC BY 3.0, GAME-DESIGN
// 17.2). Der Bereich hängt außerhalb der Zeichenfläche und außerhalb des
// Bild-Pfads — er kostet im Kampf nichts.
new CreditsView(document.body, CREDITS).mount();
