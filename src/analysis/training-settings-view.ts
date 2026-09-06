/** Training conditions from GAME-DESIGN 15.5, applied between runs. */
import type { DummySettings } from './training.js';

export function buildTrainingSettings(
  initial: DummySettings,
  school: string,
  onStart: (settings: DummySettings) => boolean,
): HTMLFormElement {
  const form = document.createElement('form');
  form.className = 'training-settings';
  // Validate explicitly so the explanation uses the game's language.
  form.noValidate = true;
  // Keep native number editing, checkbox keys and Enter submission local;
  // the window's combat shortcuts must not start a fight behind this form.
  form.addEventListener('keydown', (event) => event.stopPropagation());

  const resistanceLabel = document.createElement('label');
  resistanceLabel.className = 'training-resistance';
  resistanceLabel.append(`${school} resistance (%)`);
  const resistance = document.createElement('input');
  resistance.type = 'number';
  resistance.name = 'resistance';
  resistance.min = '0';
  resistance.max = '100';
  resistance.step = 'any';
  resistance.required = true;
  resistance.value = String((1 - initial.resistance) * 100);
  resistanceLabel.appendChild(resistance);
  form.appendChild(resistanceLabel);

  const hint = document.createElement('p');
  hint.className = 'training-settings-hint';
  hint.textContent = '0% means full damage. Other schools keep their normal damage.';
  form.appendChild(hint);

  const checkbox = (name: string, label: string, checked: boolean): HTMLInputElement => {
    const row = document.createElement('label');
    row.className = 'training-toggle';
    const input = document.createElement('input');
    input.type = 'checkbox';
    input.name = name;
    input.checked = checked;
    row.append(input, document.createTextNode(label));
    form.appendChild(row);
    return input;
  };
  const damage = checkbox('dealsDamage', 'Dummy deals damage', initial.dealsDamage);
  const casts = checkbox('castsSpells', 'Dummy casts interruptible spells', initial.castsSpells);

  const error = document.createElement('p');
  error.className = 'training-settings-error';
  error.setAttribute('role', 'alert');
  error.hidden = true;
  form.appendChild(error);
  const button = document.createElement('button');
  button.type = 'submit';
  button.className = 'training-start';
  button.textContent = 'Enter Training Room';
  form.appendChild(button);

  form.addEventListener('input', () => {
    error.hidden = true;
    resistance.removeAttribute('aria-invalid');
  });
  form.addEventListener('submit', (event) => {
    event.preventDefault();
    error.hidden = true;
    resistance.removeAttribute('aria-invalid');
    const percent = resistance.valueAsNumber;
    if (!Number.isFinite(percent) || percent < 0 || percent > 100) {
      error.textContent = 'Enter resistance from 0 to 100%.';
      error.hidden = false;
      resistance.setAttribute('aria-invalid', 'true');
      resistance.focus();
      return;
    }
    const started = onStart({
      // DummySettings stores the remaining damage factor, not percent blocked.
      resistance: 1 - percent / 100,
      dealsDamage: damage.checked,
      castsSpells: casts.checked,
    });
    if (!started) {
      error.textContent = 'Leave the current run before changing training conditions.';
      error.hidden = false;
    }
  });
  return form;
}
