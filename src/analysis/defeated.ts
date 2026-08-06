/**
 * Welche Bosse besiegt wurden (Issue #434, GAME-DESIGN 12.7).
 *
 * WARUM DAS FELD NICHT DURCH `bestRuns` ERSETZT WIRD
 * --------------------------------------------------
 * Ein Sieg landet seit #377 in `bestRuns` — aber das ist eine **andere
 * Aussage**: „beste Zeit" ist nicht „besiegt". Wer einen Boss besiegt und
 * danach schlechter läuft, steht weiter mit seiner alten Bestzeit dort; und
 * ein Eintrag entsteht dort ausschliesslich bei einem Sieg, was die beiden
 * heute zufällig gleich aussehen lässt.
 *
 * „Zufällig gleich" ist der gefährliche Teil. `bestRuns` hält die Güte eines
 * Siegs, `defeatedBosses` seine Tatsache. Sobald eine der beiden Listen ihre
 * Bedingung ändert, laufen sie auseinander — und wer die eine für die andere
 * gelesen hat, merkt es nicht.
 *
 * ES IST DER HAUPTPFAD
 * --------------------
 * GAME-DESIGN 12.7 nennt vier Freischaltungs-Kriterien nebeneinander, und
 * „Instanz abgeschlossen" ist das erste. Ohne dieses Feld gibt es Fortschritt
 * nur über die Punkte-Schwelle — die ausdrücklich als „Ventil gegen
 * Feststecken" gedacht ist und nicht als Hauptweg.
 *
 * JE BOSS **UND** STUFE
 * ---------------------
 * Derselbe Schlüssel wie bei der Fortschrittsmarke, und aus demselben Grund:
 * Ein Boss auf mythisch ist nicht derselbe Kampf wie auf normal. Der Schlüssel
 * entsteht deshalb über `markKey` und nicht ein zweites Mal daneben — „wer ihn
 * an zwei Stellen bildet, bildet ihn irgendwann verschieden".
 */
import { markKey } from './progress-mark.js';

/**
 * Den Sieg über einen Boss vermerken.
 *
 * Gibt eine **neue** Liste zurück und ändert die übergebene nicht — dieselbe
 * Vorsorge wie bei `recordAttempt` und `recordBest`: Der Aufrufer entscheidet,
 * wann er ablegt.
 *
 * **Ohne Sieg passiert nichts.** Die Entscheidung fällt hier und nicht an der
 * Aufrufstelle: Ob ein Lauf zählt, ist eine Frage an den Lauf, nicht an den,
 * der ihn aufzeichnet.
 */
export function recordDefeat(
  defeated: readonly string[],
  bossId: string,
  difficulty: number,
  victory: boolean,
): readonly string[] {
  if (!victory) return defeated;
  const schluessel = markKey(bossId, difficulty);
  // **Kein doppelter Eintrag.** Einmal besiegt ist besiegt; ein zweiter Sieg
  // sagt nichts Neues, und eine Liste, die mit jedem Lauf wächst, wäre ein
  // Zähler mit falschem Namen.
  if (defeated.includes(schluessel)) return defeated;
  return [...defeated, schluessel];
}

/** Ob dieser Boss auf dieser Stufe schon besiegt wurde. */
export function isDefeated(
  defeated: readonly string[],
  bossId: string,
  difficulty: number,
): boolean {
  return defeated.includes(markKey(bossId, difficulty));
}
