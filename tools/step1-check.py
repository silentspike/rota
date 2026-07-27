#!/usr/bin/env python3
"""step1-check.py — Referenz-Rechnung fuer die drei Zauber aus Schritt 1.

Zweck
-----
Belegt das harte Abnahme-Kriterium aus GAME-DESIGN.md 20.3:

    Eine schlechte Rotation muss messbar weniger Schaden machen als eine gute —
    mindestens 10 %.

Das Skript bildet die Regeln aus STEP1-SPEC.md in Miniatur ab und spielt zwei
Rotationen gegen denselben Testboss durch. Es ist damit zweierlei:

  1. der Nachweis, dass die gewaehlten Zahlen ueberhaupt eine Rotationsfrage erzeugen
  2. die **Referenz**, gegen die der E1-Worker seine Umsetzung pruefen kann —
     Spiel und Skript muessen auf dieselben Werte kommen

Bewusste Vereinfachungen (gelten NUR fuer Schritt 1)
---------------------------------------------------
  * kein Krit — auf Stufe 1 ist PURITY = 0, also ist der Erwartungswert der
    tatsaechliche Wert. Damit ist die Rechnung exakt statt gemittelt.
  * keine Ausruestung, keine Talente, keine Widerstaende — alle Faktoren = 1,0
  * kein Boss-Schaden, keine Phasen, keine Mechaniken

Determinismus
-------------
Zeit ist ganzzahlig in Millisekunden, der Takt betraegt 10 ms (GAME-DESIGN 18.2).
Keine Fliesskommazeit, kein Zufall — zwei Laeufe liefern byteweise dasselbe.

Aufruf:  python3 tools/step1-check.py
Rueckgabe: 0 wenn das 10-%-Kriterium erfuellt ist, sonst 1
"""

import sys

# ---------------------------------------------------------------- Konstanten

TICK_MS = 10          # Logik-Takt
GCD_MS = 1500         # globale Wartezeit
FIGHT_MS = 60_000     # Ueberdruck-Timer des Testbosses

HEAT_CAP = 20         # Obergrenze der Hitze
HEAT_DECAY_EVERY_MS = 2000   # ein Punkt Verfall alle 2 s
# Bewusst so gewaehlt, dass die schlechte Rotation KNAPP scheitert (~96 %).
# Das ist die Frust-Vermeidung aus GAME-DESIGN 12.8: Der Versuch bringt etwas
# (die Fortschrittsmarke steht bei 96 %), und die Fehleranalyse sagt, woran es lag.
# Ein Boss, an dem man mit 60 % scheitert, lehrt nichts.
BOSS_HP = 4200

# --- Zauber 1: KINDLE (Fueller) ---
# Abklingzeit = GCD: der Fueller muss den Takt luecken los fuellen koennen.
# Mit einer laengeren Abklingzeit entstuende Zwangs-Leerlauf, den kein Koennen
# vermeiden kann — das waere kein Rotationsspiel, sondern eine Wartezeit.
KINDLE_CD_MS = 1500
KINDLE_DAMAGE = 100
KINDLE_HEAT = 3

# --- Zauber 2: SMOULDER (DoT) ---
# KEINE eigene Abklingzeit (= GCD). Wie in WoW: Das Spiel hindert niemanden
# daran, zu frueh zu erneuern — genau darin liegt die Faehigkeit.
SMOULDER_CD_MS = 1500
SMOULDER_DURATION_MS = 12_000
SMOULDER_TICK_MS = 2000
SMOULDER_TICK_DAMAGE = 40
SMOULDER_TICK_HEAT = 1

# --- Zauber 3: PYRE (Entlade-Zauber) ---
# 12 Schaden je Hitzepunkt — UND ab 15 Hitze das Doppelte.
#
# Warum eine Schwelle und nicht "bei voller Hitze":
#   Rein lineare Skalierung waere mathematisch neutral — jeder Hitzepunkt ist
#   gleich viel wert, egal wann man ihn ausgibt. Es gaebe also gar keine
#   Entscheidung. Eine Bedingung "genau am Maximum" waere das andere Extrem:
#   der Verfall zieht die Hitze staendig unter den Cap, sodass der Spieler den
#   Moment praktisch nie trifft (gemessen: 2 Zuendungen in 60 s statt 8).
#
# Die Schwelle erzeugt zusammen mit dem Cap ein FENSTER von 15 bis 20:
#   zu frueh  -> halber Schaden
#   zu spaet  -> Hitze verpufft am Cap
# Genau das ist die Faehigkeit, die Schritt 1 lehren soll.
PYRE_CD_MS = 5000
PYRE_PER_HEAT = 12
PYRE_BONUS_THRESHOLD = 15
PYRE_BONUS_MULTIPLIER = 2


class Fight:
    """Ein Kampf-Durchlauf. Ganzzahlige Zeit, keine Speicheranforderung im Takt."""

    def __init__(self, name):
        self.name = name
        self.t = 0
        self.damage = 0
        self.gcd_until = 0
        self.cd = {"kindle": 0, "smoulder": 0, "pyre": 0}
        self.heat = 0
        self.next_decay = HEAT_DECAY_EVERY_MS
        self.smoulder_expires = -1        # -1 = nicht aktiv
        self.smoulder_next_tick = -1
        self.log = []                 # (t, was, wert) — spaeter der Ringpuffer
        self.casts = {"kindle": 0, "smoulder": 0, "pyre": 0}
        self.idle_ms = 0
        self.smoulder_ticks_lost = 0      # durch zu fruehes Erneuern verfallene Ticks
        self.heat_overflow = 0        # am Cap verpuffte Hitze

    # -- Hilfen -------------------------------------------------------------

    def ready(self, spell):
        return self.cd[spell] <= self.t and self.gcd_until <= self.t

    def smoulder_active(self):
        return self.smoulder_expires > self.t

    def smoulder_remaining_ms(self):
        return max(0, self.smoulder_expires - self.t)

    def add_heat(self, n):
        before = self.heat
        self.heat = min(HEAT_CAP, self.heat + n)
        self.heat_overflow += n - (self.heat - before)

    # -- Zauber -------------------------------------------------------------

    def cast_kindle(self):
        self.damage += KINDLE_DAMAGE
        self.add_heat(KINDLE_HEAT)
        self.cd["kindle"] = self.t + KINDLE_CD_MS
        self.gcd_until = self.t + GCD_MS
        self.casts["kindle"] += 1
        self.log.append((self.t, "kindle", KINDLE_DAMAGE))

    def cast_smoulder(self):
        # Erneuern setzt zurueck: die verbleibende Restzeit ist verloren.
        if self.smoulder_active():
            self.smoulder_ticks_lost += self.smoulder_remaining_ms() // SMOULDER_TICK_MS
        self.smoulder_expires = self.t + SMOULDER_DURATION_MS
        self.smoulder_next_tick = self.t + SMOULDER_TICK_MS
        self.cd["smoulder"] = self.t + SMOULDER_CD_MS
        self.gcd_until = self.t + GCD_MS
        self.casts["smoulder"] += 1
        self.log.append((self.t, "smoulder", 0))

    def cast_pyre(self):
        dmg = PYRE_PER_HEAT * self.heat
        if self.heat >= PYRE_BONUS_THRESHOLD:
            dmg *= PYRE_BONUS_MULTIPLIER
        self.damage += dmg
        self.log.append((self.t, "pyre", dmg))
        self.heat = 0
        self.cd["pyre"] = self.t + PYRE_CD_MS
        self.gcd_until = self.t + GCD_MS
        self.casts["pyre"] += 1

    # -- Takt ---------------------------------------------------------------

    def step_passive(self):
        """DoT-Ticks und Hitze-Verfall. Laeuft unabhaengig von Tastendruecken."""
        if self.smoulder_active() and self.t >= self.smoulder_next_tick:
            self.damage += SMOULDER_TICK_DAMAGE
            self.add_heat(SMOULDER_TICK_HEAT)
            self.log.append((self.t, "smoulder_tick", SMOULDER_TICK_DAMAGE))
            self.smoulder_next_tick += SMOULDER_TICK_MS
        if self.t >= self.next_decay:
            self.heat = max(0, self.heat - 1)
            self.next_decay += HEAT_DECAY_EVERY_MS

    def run(self, apl):
        """apl(fight) -> Zaubername oder None. Reine Funktion des Zustands."""
        killed_at = None
        while self.t < FIGHT_MS:
            self.step_passive()
            if self.damage >= BOSS_HP and killed_at is None:
                killed_at = self.t
                break
            if self.gcd_until <= self.t:
                choice = apl(self)
                if choice == "kindle":
                    self.cast_kindle()
                elif choice == "smoulder":
                    self.cast_smoulder()
                elif choice == "pyre":
                    self.cast_pyre()
                else:
                    self.idle_ms += TICK_MS
            self.t += TICK_MS
        return killed_at


# ------------------------------------------------------- Die zwei Rotationen

def apl_good(f):
    """Gute Rotation — nutzt beide Entscheidungen, die Schritt 1 anbietet.

    1. SMOULDER erst erneuern, wenn er auslaeuft — keine Sekunde frueher,
       sonst ist die Restlaufzeit verloren UND der GCD verschwendet
    2. PYRE erst bei voller Hitze zuenden, weil dort der Verdopplungs-Bonus liegt
    3. sonst KINDLE als Fueller
    """
    if f.smoulder_remaining_ms() < GCD_MS and f.ready("smoulder"):
        return "smoulder"
    if f.ready("pyre") and f.heat >= PYRE_BONUS_THRESHOLD:
        return "pyre"
    if f.ready("kindle"):
        return "kindle"
    return None


def apl_bad(f):
    """Schlechte Rotation — plausibel falsch, nicht absichtlich dumm.

    So spielt jemand, der die beiden Regeln noch nicht verstanden hat:
    1. erneuert den DoT, sobald er "bald" ablaeuft (halbe Laufzeit) — verschenkt
       damit Restzeit und einen GCD
    2. zuendet PYRE, sobald es bereit ist — nimmt den Verdopplungs-Bonus nie mit
    3. sonst ebenfalls KINDLE
    """
    if f.smoulder_remaining_ms() < SMOULDER_DURATION_MS // 2 and f.ready("smoulder"):
        return "smoulder"
    if f.ready("pyre") and f.heat > 0:
        return "pyre"
    if f.ready("kindle"):
        return "kindle"
    return None


def apl_dot_error_only(f):
    """Nur der DoT-Fehler: erneuert zu frueh, zuendet PYRE aber richtig."""
    if f.smoulder_remaining_ms() < SMOULDER_DURATION_MS // 2 and f.ready("smoulder"):
        return "smoulder"
    if f.ready("pyre") and f.heat >= PYRE_BONUS_THRESHOLD:
        return "pyre"
    if f.ready("kindle"):
        return "kindle"
    return None


def apl_pyre_error_only(f):
    """Nur der PYRE-Fehler: DoT richtig, aber jede Zuendung zu frueh."""
    if f.smoulder_remaining_ms() < GCD_MS and f.ready("smoulder"):
        return "smoulder"
    if f.ready("pyre") and f.heat > 0:
        return "pyre"
    if f.ready("kindle"):
        return "kindle"
    return None


# ------------------------------------------------------------------- Ausgabe

def report(f, killed_at):
    print(f"  {f.name}")
    print(f"    Schaden gesamt      {f.damage:>8,}".replace(",", "."))
    print(f"    Wirkungen           KINDLE {f.casts['kindle']:>2}  "
          f"SMOULDER {f.casts['smoulder']:>2}  PYRE {f.casts['pyre']:>2}")
    print(f"    PYRE-Schaden        {sum(v for _, k, v in f.log if k == 'pyre'):>8,}"
          .replace(",", "."))
    print(f"    SMOULDER-Ticks      "
          f"{sum(v for _, k, v in f.log if k == 'smoulder_tick'):>8,}".replace(",", "."))
    print(f"    verlorene DoT-Ticks {f.smoulder_ticks_lost:>8}")
    print(f"    verpuffte Hitze     {f.heat_overflow:>8}")
    print(f"    Leerlauf            {f.idle_ms / 1000:>8.1f} s")
    if killed_at is None:
        print(f"    Boss                UEBERLEBT ({f.damage}/{BOSS_HP} = "
              f"{100 * f.damage / BOSS_HP:.1f} %)")
    else:
        print(f"    Boss                besiegt bei {killed_at / 1000:.2f} s "
              f"({(FIGHT_MS - killed_at) / 1000:.2f} s Reserve)")
    print()


def main():
    print("=" * 66)
    print("ROTA Schritt 1 — Referenz-Rechnung")
    print(f"Testboss {BOSS_HP} HP, Ueberdruck nach {FIGHT_MS // 1000} s, "
          f"Takt {TICK_MS} ms, GCD {GCD_MS} ms")
    print("=" * 66)
    print()

    good = Fight("GUTE ROTATION   (SMOULDER spaet erneuern, PYRE ab 15 Hitze)")
    good_kill = good.run(apl_good)
    report(good, good_kill)

    bad = Fight("SCHLECHTE ROT.  (SMOULDER zu frueh, PYRE sofort)")
    bad_kill = bad.run(apl_bad)
    report(bad, bad_kill)

    # Fuer den Vergleich zaehlt der Schaden ueber die volle Kampfdauer.
    # Die gute Rotation bricht beim Kill ab, also wird sie ohne Abbruch nachgerechnet.
    good_full = Fight("gut, volle 60 s")
    good_full_hp = BOSS_HP
    globals()["BOSS_HP"] = 10 ** 9          # Kill-Abbruch aushebeln
    good_full.run(apl_good)
    bad_full = Fight("schlecht, volle 60 s")
    bad_full.run(apl_bad)
    globals()["BOSS_HP"] = good_full_hp

    # Was kostet jeder Fehler einzeln?
    globals()["BOSS_HP"] = 10 ** 9
    only_dot = Fight("nur DoT-Fehler")
    only_dot.run(apl_dot_error_only)
    only_pyre = Fight("nur PYRE-Fehler")
    only_pyre.run(apl_pyre_error_only)
    globals()["BOSS_HP"] = good_full_hp

    diff = good_full.damage - bad_full.damage
    pct = 100 * diff / bad_full.damage

    print("-" * 66)
    print("VERGLEICH ueber die vollen 60 s (ohne Kill-Abbruch)")
    print(f"  gut                     {good_full.damage:>8,}".replace(",", "."))
    print(f"  nur DoT zu frueh        {only_dot.damage:>8,}   "
          f"({100 * (only_dot.damage - good_full.damage) / good_full.damage:+.1f} %)"
          .replace(",", "."))
    print(f"  nur PYRE zu frueh       {only_pyre.damage:>8,}   "
          f"({100 * (only_pyre.damage - good_full.damage) / good_full.damage:+.1f} %)"
          .replace(",", "."))
    print(f"  beide Fehler            {bad_full.damage:>8,}   "
          f"({100 * (bad_full.damage - good_full.damage) / good_full.damage:+.1f} %)"
          .replace(",", "."))
    print(f"  Differenz gut/schlecht  {diff:>8,}  = {pct:.1f} % mehr".replace(",", "."))
    print("-" * 66)

    ok = True
    if pct < 10.0:
        print(f"FEHLGESCHLAGEN: {pct:.1f} % — das Kriterium verlangt mindestens 10 %.")
        ok = False
    else:
        print(f"BESTANDEN: {pct:.1f} % >= 10 % (Kriterium GAME-DESIGN 20.3)")

    if good_kill is None:
        print("FEHLGESCHLAGEN: Die gute Rotation legt den Boss nicht im Zeitlimit.")
        ok = False
    if bad_kill is not None:
        print(f"FEHLGESCHLAGEN: Auch die schlechte Rotation legt den Boss "
              f"(bei {bad_kill / 1000:.2f} s) — der Boss trennt nicht.")
        ok = False
    if ok:
        print("BESTANDEN: gute Rotation legt den Boss, schlechte nicht.")

    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
