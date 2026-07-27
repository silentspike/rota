import './render/page.css';
import { App } from './core/app.js';
import { installDevTools } from './devtools/index.js';

const root = document.querySelector<HTMLDivElement>('#app');
if (root === null) {
  throw new Error('#app fehlt in index.html');
}

const app = new App(root);
installDevTools(app);
app.start();
