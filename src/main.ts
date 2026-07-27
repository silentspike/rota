import { installDevTools } from './devtools/index.js';

installDevTools();

const app = document.querySelector<HTMLDivElement>('#app');
if (app) {
  app.textContent = 'ROTA';
}
