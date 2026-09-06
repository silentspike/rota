# Numerischer Vertrag der Krit-Verteilung (#965)

## Geltungsbereich und Zahlenformat

Die vorhandene Verteilung führt weiterhin `f64`-Massen in der unveränderten
Reihenfolge einer `BTreeMap`. Es werden weder Zustände beschnitten noch Massen
renormiert. Die bisherigen Methoden geben dieselben Punktrechnungen aus;
`*_grenze` und `massen_nachweis` liefern zusätzlich nach außen gerundete
Fehlerzertifikate. Die neue `*_geprueft`-API meldet verletzte Voraussetzungen
als `NumerikFehler`; die bisherigen Konstruktoren brechen dabei ausdrücklich ab.

**Entscheidung AC1:** binary64 beibehalten. Das Format entspricht dem
vorhandenen skalaren Rechenpfad und erfordert weder eine neue Abhängigkeit
noch eine Änderung von Kampfwerten, Zufall oder Paritätsaufzeichnungen.
Festkomma würde einen neuen Skalierungs-/Überlaufvertrag benötigen;
rationale Produktionsmassen würden mit der Trefferzahl wachsende ganze Zahlen
benötigen. Eine Leistungsüberlegenheit wird für keines dieser Formate behauptet.
Rationale Arithmetik dient als unabhängige Testreferenz.

Der Bezugspunkt des Zertifikats ist **genau das vorhandene Bernoulli-Modell**:

- Die Trefferfolge ist fest; ein Krit verändert keine folgenden Treffer.
- `Treffer` enthält bereits gerundete ganzzahlige Schäden in derselben Einheit.
- `p` bezeichnet die exakte reelle Zahl des übergebenen binary64-Wertes.
- Für jeden Treffer sind die beiden Modellwahrscheinlichkeiten exakt `p` und
  `1-p`; die Bernoulli-Entscheidungen der festen Trefferstufen werden als
  unabhängig modelliert. Der DP selbst berechnet deren Produkte und Summen mit
  binary64.
- Phasen sind die vorhandene Beschriftung durch kumulierte Schadensschwellen;
  eine Siegchance bedeutet hier `P(Schaden >= Lebenspunkte)` am Ende dieser
  festen Folge.

Dies ist **keine vollständige Kampf-DP**. Die Trefferfolge wird weder aus
kritabhängigem Kontrollfluss bewiesen noch bei einem Kill abgeschnitten.
Ebenso ist ein Modellfehler kein Rundungsfehler: **Unter dem Modell eines
gleichverteilten `u32`-Wurfs** beträgt die Trefferwahrscheinlichkeit der
ganzzahligen Schwellenabbildung `ceil(p*2^32)/2^32`. Bei `p=0.1` liegt dieser Wert ungefähr `9.31e-11`
über dem vorhandenen DP-Modell. Ein eigener Kontrolltest zeigt getrennte
Intervalle. Wer genau diese diskrete Modellwahrscheinlichkeit untersuchen
will, übergibt sie ausdrücklich; die Voreinstellung wird nicht still verändert.
Ein fester `Wurfschluessel` liefert dagegen einen deterministischen FNV-Wert.
Der DP und dieses Fehlerzertifikat beweisen weder Gleichverteilung noch
Unabhängigkeit der tatsächlichen PRNG-Ausgaben.

`analytisch::erwarteter_schaden` kombiniert dagegen zwei vollständige
Balance-Läufe. Es verwendet die DP-Massen nicht und erhält durch diesen
Vertrag kein Zertifikat seiner Eingangs-/Kampfstatistiken. Sein Kommentar
benennt diese Grenze jetzt ausdrücklich. Ebenso gehören Stichprobenfehler und
Konfidenzfolgen anderer Module nicht zu den DP-Rundungsgrenzen.

## Im Code geprüfte Voraussetzungen

`p` und angefragte Quantilwahrscheinlichkeiten müssen endlich und in `[0,1]`
liegen. Alle geführten Massen und Fehlergrenzen müssen endlich und nichtnegativ
sein. Schadensaddition und Übergangs-/Schrittzähler verwenden `checked_add`;
die Zahl der Phasenschwellen wird vor der Umwandlung in `u32` geprüft. Eine
leere `Default`-Verteilung besitzt keinen Nachweis. Ein Zertifikat entsteht nur
aus der exakten Startmasse 1 durch die geprüften Übergänge.

Die elementaren Operationen folgen den
[Rust-binary64-Semantiken](https://doc.rust-lang.org/std/primitive.f64.html).
Es werden keine algebraischen/fast-math-Operationen, approximativen Intrinsics
oder FMA-Umschreibungen verwendet. Laufzeitproben mit `black_box` prüfen im
ausführenden Thread Subnormal-Eingaben, Subnormal-Ergebnisse und die beiden
Richtungen des ties-to-even-Falls. FTZ/DAZ oder ein geänderter Rundungsmodus
werden damit für diese Trägerfälle abgelehnt. Compileroptionen, welche Rusts
elementare Rechensemantik verletzen, sind außerhalb des Vertrags; endliche
Laufzeitproben sind kein universeller Compilerbeweis.

`u64`-Schäden oberhalb `2^53` werden nicht als exakt in binary64 darstellbar
angenommen. Ihre Momentrechnung benutzt ein Intervall zwischen benachbarten
binary64-Zahlen. Der Vertrag benötigt deshalb keinen neuen globalen
Werte-Deckel oder eine vorweggenommene Entscheidung aus E14.1.

## Beweis der Massenfehlergrenze (AC2, AC4, AC5)

Sei `q` der Vektor der idealen Modellmassen und `m` der gespeicherte
binary64-Vektor. Der private Nachweis führt

```text
||m - q||₁ <= E.
```

Am Anfang ist `m=q=(1)` und `E=0`. Für einen Treffer ist die ideale
Übergangsabbildung `T` stochastisch: Jede Ausgangsmasse wird mit den
nichtnegativen Faktoren `p` und `1-p` aufgeteilt, deren Summe 1 ist.
Zusammenlegen gleicher Zielzustände vergrößert die L1-Norm nicht. Daher gilt

```text
||T(m) - T(q)||₁ <= ||m - q||₁ <= E.
```

Die Implementation berechnet `c = fl(1-p)`. Sei `d` eine konservative
absolute Grenze für `|c-(1-p)|`. Vor der lokalen Produktrundung ist der
zusätzliche Gesamtfehler höchstens `d * sum(m)`.

Für jedes tatsächlich berechnete Produkt und jede Addition beim Zusammenlegen
wird eine absolute Rundungsgrenze `r` addiert. Für ein endliches,
nichtnegatives elementares Ergebnis `y` genügt

```text
r(y) = next_up(y) - y.
```

Diese obere Nachbarlücke ist mindestens die untere Nachbarlücke. Sie enthält
damit den Rundungsfehler auf beiden Seiten, auch an einer Zweierpotenz.
Bei einem Ergebnis 0 und im Subnormalbereich ist sie `2^-1074`; ein unterlaufener
positiver Beitrag erhält also weiterhin eine absolute Fehlergrenze. Der
Beweis verwendet **keine** relative `gamma_n`-Formel ohne Unterlaufterm.
Exakte Operationen mit 0 oder dem Faktor 1 benötigen keinen zusätzlichen Term.

Induktiv genügt

```text
E_neu >= E_alt
         + sum_z m_z * d
         + sum_ausgeführte_Produkte r(produkt)
         + sum_ausgeführte_Zusammenlegungen r(summe).
```

Die rechte Seite selbst wird ausschließlich nach oben gerundet. Damit ist
auch die Fehlerrechnung in der Schranke enthalten. Dieser Ansatz benutzt
die klassischen Regeln für absolute lokale Rundungsfehler, wie in
[Highams Summationsanalyse](https://nhigham.com/wp-content/uploads/2023/10/high93s.pdf),
und führt sie für die konkrete positive DP-Rechnung induktiv fort.

Der Nachweis nennt finale Zustandszahl, Trefferstufen und **alle tatsächlich
ausgeführten Zustandsübergänge über sämtliche Stufen**. Die Fehlerterme
entstehen an diesen Operationen. Eine feste Toleranz für beispielsweise zehn
Zustände wird nicht auf größere Rechnungen übertragen. Auch tausend Treffer
mit nur einem finalen Zustand erzeugen tausend Aufteilungen und Merges und
eine entsprechend fortgeschriebene Grenze.

Nach jedem Treffer wird die beobachtete Gesamtmasse zusätzlich durch eine
nach außen gerundete Summation eingeschlossen und um `E` erweitert. Dieses
Intervall muss 1 enthalten. Andernfalls endet der Übergang mit
`Massenerhalt`. Diese Prüfung ersetzt den L1-Beweis nicht: Zwei verkehrte
Massen könnten sich in der Gesamtsumme aufheben.

## Eigener Nachweis je Statistik (AC3)

`StatistikGrenze` enthält den bisherigen Punktwert, ein Intervall für den
idealen Modellwert und eine nach außen gerundete absolute Fehlergrenze vom
Punktwert zum entferntesten Intervallende. Ein Punktwert muss selbst nicht
innerhalb des Modellintervalls liegen.

| Statistik                                    | Beweis und berechnete Grenze                                                                                                                                                                                                                                                      |
| -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Gesamtmasse                                  | Nach außen gerundete Summe der gespeicherten Massen, erweitert um `E`; enthält die ideale 1.                                                                                                                                                                                      |
| Beliebiges Ereignis, Phasen- oder Siegchance | Für jede Zustandsmenge `A` gilt `abs(sum_A(m-q)) <= E`. Die eingeschlossene Teilsumme wird um `E` erweitert und mit `[0,1]` geschnitten. Das Ereignis wird je Zustand genau einmal ausgewertet.                                                                                   |
| Schadensmittel                               | Mit nichtnegativen Schäden `x <= M` gilt `abs(sum x*(m-q)) <= M*E`. Integerkonversion, Produkte und Summation erhalten eigene Intervalle; die resultierende Momentgrenze wird um `M*E` erweitert.                                                                                 |
| Zweites Schadensmoment                       | Dieselbe Aussage mit `x²` und `M²`; sowohl Integerkonversion als auch Quadrierung, Gewichtung und Summation werden nach außen eingeschlossen.                                                                                                                                     |
| Schadensvarianz                              | Für `mu` und zweites Moment `m2` gelten eigene Intervalle. `Var = m2 - mu²` liegt in `[m2_lo-mu_hi², m2_hi-mu_lo²]`, nach außen gerundet und mit den nichtnegativen Zahlen geschnitten. Bei fast null Varianz kann das Intervall breit sein; es gibt keine relative Fehlerzusage. |
| Standardabweichung                           | Die Quadratwurzel ist auf nichtnegativen Zahlen monoton. Die nach außen gerundeten Wurzeln beider Varianzgrenzen schließen die Standardabweichung ein, auch bei null; keine Ableitungsabschätzung durch Division durch die Standardabweichung.                                    |
| Schadensquantil                              | Eigene CDF-Inversion mit den folgenden Schadensgrenzen. Keine Übertragung einer Massentoleranz als Schadenstoleranz.                                                                                                                                                              |

Für `0<alpha<1` sei `Q(alpha)=min{x:F(x)>=alpha}`. Jede CDF-Teilsumme erhält
ein Intervall `[L(x),U(x)]`. Dann gilt:

- Untere Schadensgrenze: erster Schaden mit `U(x)>=alpha`. Alle kleineren
  Schäden haben sicher noch nicht die erforderliche kumulierte Masse.
- Obere Schadensgrenze: erster Schaden mit `L(x)>=alpha`. Spätestens dort
  erreicht die wahre CDF `alpha`. Falls keine untere CDF-Grenze dies belegt,
  bleibt der maximal erreichbare Schaden die sichere obere Grenze.

Für `alpha=0` wird der minimale erreichbare Schaden definiert, für `alpha=1`
der maximale. Erreichbare Zustände mit numerisch unterlaufener Masse bleiben
erhalten; ihr wahrer positiver Beitrag darf nicht aus der Unterstützung
verschwinden. Bei einer halben Masse auf 0 und einer halben Masse auf `10^9`
liefert der unsichere Median deshalb gegebenenfalls das Intervall `[0,10^9]`.

## Ausführbare Kontrollproben

`sim/tests/verteilung_numerik.rs` enthält folgende unabhängige Kontrollen:

- **16.384 Zustände mit `p=1/16`:** Alle Referenzmassen sind rationale Zahlen
  `15^(14-popcount(x))/2^56`. `15^14>2^53` erzwingt reale Rundung. Die
  IEEE-Bitmuster der gespeicherten Massen und der Fehlergrenze werden als
  ganzzahlige Vielfache von `2^-108` gelesen. Damit wird die tatsächliche
  L1-Abweichung exakt mit `u128` gegen die Grenze verglichen, ohne zweite DP
  und ohne Gleitkommasumme im Referenzvergleich. Die CDF-Quantile entstehen
  ebenfalls aus diesen exakten ganzzahligen Zählern.
- **16.384 Zustände mit dem tatsächlichen binary64-Wert `p=0.1`:** Die
  unabhängige Python-`Fraction`-Referenz in
  `sim/tests/fixtures/verteilung-965-reference.json` enthält exakte Zähler und
  Nenner sowie einschließende binary64-Endpunkte. Massen, Mittel, Varianz,
  Standardabweichung, Phasen-/Siegchance und mehrere Quantile werden geprüft.
- **Ein Zustand, tausend Schritte:** Die gleiche finale Zustandszahl darf die
  wachsende Anzahl von Übergängen und Zusammenlegungen nicht verbergen.
- **Median am großen Sprung:** Zwei benachbarte Wahrscheinlichkeiten um 0.5
  können Mediane mit einer Milliarde Abstand haben; die Schadensintervalle
  müssen beide Möglichkeiten korrekt einordnen.
- **Unterlauf:** Der seltenste positive Modellpfad rundet auf Masse 0, bleibt
  aber erreichbar, mit positiver Wahrscheinlichkeitsobergrenze und richtigem
  maximalem Quantil. Der kleinste Subnormalwert wird auch als Eingabe geprüft.
- **Fehlende Voraussetzungen:** NaN, Unendlich, Wahrscheinlichkeiten außerhalb
  `[0,1]`, fehlende Provenienz, manipulierte Gesamtmasse, Zählerüberlauf,
  Schadensüberlauf und Integerkonversion oberhalb `2^53`.
- **Modellgrenze:** Der Unterschied zwischen `p` und der ganzzahligen
  RNG-Abbildung wird ausdrücklich sichtbar und nicht als Rundung zertifiziert.

Die vorhandenen DP-Prüfungen gegen vollständige Aufzählung und Binomialformeln
bleiben erhalten. Ihre Punktwerte werden nicht auf eine neue Summationsfolge
umgestellt.

## Ausführungsstand am 6. September 2026

Nach Freigabe des gemeinsam genutzten Buildservers wurden die Rust-Schritte
seriell ausgeführt. Der Formatierungswrapper verwendete Rustfmt 1.97.1;
Cargo-Tests und Clippy verwendeten Rust 1.98.1. Alle Cargo-Aufrufe liefen mit
`--no-copy-lock`, ohne Artefakt-Copyback. Lokal lief kein Rust-Test.

| Ausgeführter Befehl                                                                        | Ergebnis                                                                                        |
| ------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| `cargo-remote-fmt`                                                                         | PASS, Exit 0; nur die vorgesehenen Rust-Dateien verändert.                                      |
| `cargo remote --no-copy-lock -- test -p rota-sim --test verteilung_numerik -- --nocapture` | PASS, Exit 0; 8 Tests, 0 fehlgeschlagen, 0 ignoriert.                                           |
| `cargo remote --no-copy-lock -- test -p rota-sim --lib verteilung:: -- --nocapture`        | PASS, Exit 0; 15 Tests: 12 vorhandene Verteilungsprüfungen und 3 neue interne Numerikprüfungen. |
| `cargo remote --no-copy-lock -- clippy --workspace --all-targets -- -D warnings`           | PASS, Exit 0, keine Warnungen.                                                                  |

Die echte 16.384-Zustandskontrolle mit `p=0.1` meldete 32.766 ausgeführte
Übergänge und 12.052 Zustände mit beobachteter Rundungsabweichung. Ihre
unabhängig nach oben eingeschlossene L1-Abweichung betrug
`5.696985822832958e-16`, die zertifizierte Grenze
`3.764855861227265e-15`. Die rationale `p=1/16`-Kontrolle fand einen tatsächlich
gerundeten Zustand und verglich die exakte L1-Abweichung
`4503599627370496 / 2^108` erfolgreich gegen die Grenze.

Auch die vollständigen Rust-Läufe sind abgeschlossen:

| Ausgeführter Befehl                                                                                   | Ergebnis                                                                                                                    |
| ----------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `cargo remote --no-copy-lock -- test --workspace --no-fail-fast`                                      | PASS, Exit 0; 1261 bestanden, 0 fehlgeschlagen, 7 ignoriert; 248,67 s einschließlich Übertragung und Bau.                   |
| `cargo remote --no-copy-lock -- test --workspace --all-targets --features diagnostics --no-fail-fast` | PASS, Exit 0; 1296 bestanden, 0 fehlgeschlagen, 7 ignoriert; 336,28 s einschließlich Wartezeit auf den globalen Serverlock. |

Die Zahlen stammen aus den tatsächlichen Testzusammenfassungen. Die
Diagnostics-Konfiguration schließt mit `--all-targets` auch die Beispieltargets
ein; der Standardlauf enthält zusätzlich die Doctests. Die sieben bereits
ignorierten Tests wurden nicht als bestanden gezählt. Diese Prüfungen gelten
für den isolierten Patch auf `e2d100a4`; die spätere Integration und ihre CI
werden separat abgenommen.

Die vollständigen lokalen Ausführungsbelege liegen unter
`/work/tmp/rota-965-evidence-20260906/`. Vor und nach den Läufen wurden
`Cargo.lock`, Paket-Lockfile, Kampfdaten und Paritätsaufzeichnungen per SHA-256
abgeglichen; keine dieser Eingaben wurde verändert. Die Python-Referenzprüfungen sind separat dokumentierte
mathematische Kontrollen und werden nicht als zusätzliche Rust-Tests gezählt.
