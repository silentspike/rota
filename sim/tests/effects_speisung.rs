//! Die Wirkungsbeschreibung stammt aus den Daten — nicht aus einer Liste.
//!
//! # Warum dieser Prueflauf noetig ist
//!
//! Ein Modul, das nie mit echten Daten gefuellt wird, ist eine gebaute Leitung
//! ohne Anschluss: Jede Zusicherung darin gilt, und keine davon betrifft das
//! Spiel. Diese Datei prueft die Speisung selbst — dass [`EFFECT_NODES`] aus
//! `data/*.json` entsteht und mit der Zauber-Tabelle daneben zusammenpasst.
//!
//! Die Kreuzprobe ist der Kern: [`SPELLS`] und `EFFECT_NODES` werden im selben
//! Bauschritt, aber aus getrennten Funktionen erzeugt. Laufen sie auseinander,
//! faellt es hier auf und nirgends sonst.

use rota_sim::data::{EFFECT_NODES, SPELLS};
use rota_sim::effects::registry::{alle_analysen, Verdict};
use rota_sim::effects::EffectOp;

/// Die Bossregeln, die `sim/build.rs` mit Semantik deklariert — als Signatur.
///
/// **Eine Liste, keine Zahl** — bei der Planung stand hier „sieben", und
/// nachgezaehlt waren es elf. Eine Zahl haette den Irrtum nicht gemeldet,
/// diese Liste nennt beim Namen, was fehlt.
///
/// # Seit #892 in beide Richtungen
///
/// Bis dahin prüfte der Fall nur, dass jeder Eintrag einen Knoten hat. Eine neu
/// deklarierte Regel fehlte der Liste, ohne dass es auffiel. Jetzt muss sie
/// genau die deklarierten Signaturen der Daten nennen — eine Abschrift, die
/// bemerkt, dass sie veraltet.
const BOSS_SYSTEME: &[&str] = &[
    "energy",
    "doom",
    "affliction",
    "race",
    "runaway",
    "stacks",
    "escalation",
    "bossHeal",
    "timelineLoopMs",
    "resistances",
    // Kein Zusatz-System, sondern die harte Zeitgrenze aus dem Kopffeld
    // `overpressureMs` — build.rs deklariert sie trotzdem als Signatur mit
    // Semantik. Fehlte hier, bis der Fall in beide Richtungen prüfte (#892).
    "overpressure",
];

#[test]
fn die_beschreibung_ist_nicht_leer() {
    assert!(
        !EFFECT_NODES.is_empty(),
        "EFFECT_NODES ist leer — dann prueft jede Zusicherung darunter nichts."
    );
    // Die Zahl steht in der Meldung, nicht in der Bedingung: Sie darf wachsen.
    let signaturen = EFFECT_NODES
        .iter()
        .filter(|n| matches!(n.op, EffectOp::CustomSignature(_)))
        .count();
    assert!(
        signaturen < EFFECT_NODES.len(),
        "Alle {} Knoten sind Signaturen. Dann traegt kein einziger Zauber eine beschriebene \
         Wirkung, und die Beschreibung beschreibt nur Bosse.",
        EFFECT_NODES.len()
    );
}

#[test]
fn jeder_zauber_mit_grundschaden_hat_einen_schadensknoten() {
    // Die Kreuzprobe: SPELLS und EFFECT_NODES entstehen aus derselben Quelle,
    // aber in getrennten Funktionen. Ein Leser, der ein Feld mit dem falschen
    // JSON-Typ liest, laesst den Zauber hier fehlen — genau so fehlten beim
    // Bau von #911 vier Zauber mit Ressourcen-Wirkung, weil `resourceGain` ein
    // Objekt ist und keine Zahl.
    let mut fehlend: Vec<&str> = Vec::new();
    let mut erwartet = 0usize;
    for spell in SPELLS.iter() {
        if spell.base_damage == 0 {
            continue;
        }
        erwartet += 1;
        let hat_knoten = EFFECT_NODES
            .iter()
            .any(|n| n.origin == spell.id && matches!(n.op, EffectOp::DealDamage));
        if !hat_knoten {
            fehlend.push(spell.id);
        }
    }
    assert!(
        erwartet > 0,
        "Kein einziger Zauber traegt Grundschaden — dann prueft dieser Fall nichts."
    );
    assert!(
        fehlend.is_empty(),
        "{} von {} Zaubern mit Grundschaden fehlen in der Wirkungsbeschreibung: {fehlend:?}",
        fehlend.len(),
        erwartet
    );
}

#[test]
fn jeder_zauber_mit_brand_hat_einen_brandknoten() {
    let mut fehlend: Vec<&str> = Vec::new();
    let mut erwartet = 0usize;
    for spell in SPELLS.iter() {
        if spell.dot.is_none() {
            continue;
        }
        erwartet += 1;
        if !EFFECT_NODES
            .iter()
            .any(|n| n.origin == spell.id && matches!(n.op, EffectOp::ApplyDot))
        {
            fehlend.push(spell.id);
        }
    }
    assert!(erwartet > 0, "Kein Zauber traegt einen Brand-Effekt.");
    assert!(
        fehlend.is_empty(),
        "{} von {erwartet} Zaubern mit Brand fehlen: {fehlend:?}",
        fehlend.len()
    );
}

#[test]
fn jeder_zauber_mit_abklingzeit_verkuerzung_hat_seinen_knoten() {
    // Der Fall, an dem sich die Planung geirrt hat: „der Abklingzeit-Ruecksetzer
    // kommt erst noch" — tatsaechlich tragen mehrere Zauber solche Effekte
    // bereits, und der Simulator kennt die Felder.
    let mut fehlend: Vec<&str> = Vec::new();
    let mut erwartet = 0usize;
    for spell in SPELLS.iter() {
        if spell.cooldown_reduction_ms == 0 && spell.cooldown_reset_charges == 0 {
            continue;
        }
        erwartet += 1;
        if !EFFECT_NODES.iter().any(|n| {
            n.origin == spell.id
                && matches!(n.op, EffectOp::ReduceCooldown | EffectOp::ResetCooldown)
        }) {
            fehlend.push(spell.id);
        }
    }
    assert!(
        erwartet > 0,
        "Kein Zauber verkuerzt oder setzt Abklingzeiten zurueck — dann prueft dieser Fall \
         nichts, und die Planungs-Aussage waere doch richtig gewesen."
    );
    assert!(
        fehlend.is_empty(),
        "{} von {erwartet} Zaubern mit Abklingzeit-Wirkung fehlen: {fehlend:?}",
        fehlend.len()
    );
}

#[test]
fn die_boss_systeme_sind_genau_die_deklarierten() {
    // Deklariert heißt: mit Semantik, also ohne Lücke. Ein unbekannter
    // Schlüssel der Bossdaten bekommt eine Signatur OHNE Semantik und fällt in
    // jeder Analyse zurück (build.rs, `knoten_fuer_boss`) — er ist keine
    // deklarierte Regel und gehört nicht in diese Liste.
    let mut deklariert: Vec<&str> = EFFECT_NODES
        .iter()
        .filter(|n| !n.hat_luecke())
        .filter_map(|n| match n.op {
            EffectOp::CustomSignature(id) => Some(id.0),
            _ => None,
        })
        .collect();
    deklariert.sort_unstable();
    deklariert.dedup();
    assert!(
        !deklariert.is_empty(),
        "Keine einzige deklarierte Signatur in den Daten — dann prüft dieser Fall nichts."
    );
    let mut liste = BOSS_SYSTEME.to_vec();
    liste.sort_unstable();
    let doppelt = liste.windows(2).filter(|p| p[0] == p[1]).count();
    let fehlend: Vec<&str> = liste
        .iter()
        .copied()
        .filter(|s| !deklariert.contains(s))
        .collect();
    let neu: Vec<&str> = deklariert
        .iter()
        .copied()
        .filter(|s| !liste.contains(s))
        .collect();
    assert!(
        fehlend.is_empty() && neu.is_empty() && doppelt == 0,
        "Ohne deklarierten Knoten: {fehlend:?} — sie stehen in der Liste, werden aber von \
         keiner Analyse gesehen. Deklariert, aber nicht in der Liste: {neu:?} — eine neue Regel \
         gehört hierher. Doppelt: {doppelt}. Deklariert sind {} Signaturarten.",
        deklariert.len()
    );
    // Die Phasen tragen einen eigenen Baustein und stehen deshalb nicht in der
    // Signatur-Liste.
    assert!(
        EFFECT_NODES
            .iter()
            .any(|n| matches!(n.op, EffectOp::ChangePhase)),
        "Kein einziger Phasen-Knoten — dabei fuehren 33 von 35 Bossdateien Phasen."
    );
}

#[test]
fn jede_baustein_art_bekommt_wenigstens_eine_erlaubnis() {
    // # Auch hier: je ART, nicht ueber die Gesamtheit
    //
    // Die erste Fassung fragte „bekommt IRGENDEIN Knoten eine Erlaubnis?".
    // Das ist grün, sobald **ein einziger** von 218 durchkommt — und waere es
    // auch dann, wenn eine ganze Baustein-Art durchgehend blockiert waere.
    // Wieder eine Zaehlung ueber die Gesamtheit statt je Element; in Issue
    // #911 war genau diese Verwechslung fuenfmal der Fehler.
    //
    // Waere eine Art durchgehend blockiert, waere ihre Beschreibung fuer jede
    // Optimierung wertlos, und niemand merkte den Unterschied zu „gar keine
    // Beschreibung".
    let mut arten: Vec<(&str, usize, usize)> = Vec::new();
    for knoten in EFFECT_NODES.iter() {
        let art = knoten.op.name();
        let erlaubt = alle_analysen(knoten)
            .iter()
            .any(|r| matches!(r.verdict, Verdict::Permits));
        match arten.iter_mut().find(|(a, _, _)| *a == art) {
            Some(eintrag) => {
                eintrag.1 += 1;
                eintrag.2 += usize::from(erlaubt);
            }
            None => arten.push((art, 1, usize::from(erlaubt))),
        }
    }
    assert!(
        !arten.is_empty(),
        "Keine Bausteine in den Daten — dann prueft dieser Fall nichts."
    );
    let blockiert: Vec<&str> = arten
        .iter()
        .filter(|(_, _, mit)| *mit == 0)
        .map(|(a, _, _)| *a)
        .collect();
    assert!(
        blockiert.is_empty(),
        "Diese Baustein-Arten bekommen von KEINER Analyse je eine Erlaubnis: {blockiert:?}. \
         Ihre Beschreibung ist damit fuer jede Optimierung wertlos. Verteilung: {arten:?}"
    );
}

#[test]
fn jeder_knoten_nennt_seine_herkunft() {
    for knoten in EFFECT_NODES.iter() {
        assert!(
            !knoten.origin.is_empty(),
            "Ein Knoten ({:?}) nennt keine Herkunft. Ohne sie ist ein Befund nicht auffindbar.",
            knoten.op
        );
    }
}

#[test]
fn ein_knoten_ohne_semantik_in_den_echten_daten_faellt_ueberall_zurueck() {
    // # Der Waechter fuer AC5 an echten Daten
    //
    // Im Bestand findet er nichts: Alle elf Boss-Systeme sind eingeordnet.
    // Kommt ein zwoelftes dazu, ohne dass jemand seine Semantik deklariert,
    // erzeugt `build.rs` einen Knoten ohne jede Angabe — und dieser Fall
    // haelt fest, dass der dann WIRKLICH ueberall zurueckfaellt statt
    // guenstig durchzulaufen.
    //
    // **Gemessen am 2026-08-18 als Differenz zweier Laeufe:** mit einem
    // testweise eingefuegten Feld `erfundenesZusatzSystem` in
    // `data/bosses/first-crucible.json` sah dieser Fall 1 Knoten und alle
    // Analysen fielen zurueck; ohne das Feld sieht er 0. Das Feld wurde
    // danach wieder entfernt.
    let ohne_semantik: Vec<&rota_sim::effects::EffectNode> = EFFECT_NODES
        .iter()
        .filter(|n| n.signature_ohne_semantik())
        .collect();

    eprintln!(
        "Knoten ohne Semantik in den echten Daten: {} von {}",
        ohne_semantik.len(),
        EFFECT_NODES.len()
    );

    for knoten in &ohne_semantik {
        let ergebnisse = alle_analysen(knoten);
        let zurueckgefallen = ergebnisse
            .iter()
            .filter(|r| matches!(r.verdict, Verdict::Conservative))
            .count();
        assert_eq!(
            zurueckgefallen,
            ergebnisse.len(),
            "Der Knoten \"{}\" hat keine Semantik, aber nur {zurueckgefallen} von {} Analysen \
             sind zurueckgefallen. Ein unbekanntes Zusatz-System darf nicht guenstig \
             durchlaufen — unbekannt muss langsam sein, nie falsch.",
            knoten.origin,
            ergebnisse.len()
        );
    }
}

#[test]
fn jeder_zauber_hat_mindestens_einen_knoten() {
    // # Die Pruefung, die von Anfang an gefehlt hat
    //
    // Beim ersten Bau von #911 bekamen **13 von 70** Zaubern gar keinen
    // Knoten: die sieben Kontroll-Zauber, `purge`, `quench`, `harden`,
    // `sepsis`, `decay` und `putrid-bloom`. Der Bau war gruen, alle
    // Zusicherungen hielten, und die Beschreibung war fuer ein Fuenftel der
    // Zauber blind.
    //
    // Die Einzelpruefungen darueber konnten das nicht finden: Sie fragen je
    // Wirkungsart, und wer gar keine der geprueften Arten traegt, faellt durch
    // jedes Raster. **Dieser Fall fragt von der anderen Seite** — nicht „hat
    // jede Art ihre Zauber?", sondern „hat jeder Zauber seine Beschreibung?".
    //
    // Ein Zauber ohne jede Wirkung ist selbst ein Befund und kein Grund, diese
    // Zusicherung aufzuweichen.
    let mut ohne: Vec<&str> = Vec::new();
    for spell in SPELLS.iter() {
        if !EFFECT_NODES.iter().any(|n| n.origin == spell.id) {
            ohne.push(spell.id);
        }
    }
    assert!(
        !SPELLS.is_empty(),
        "Es gibt keine Zauber — dann prueft dieser Fall nichts."
    );
    assert!(
        ohne.is_empty(),
        "{} von {} Zaubern haben KEINEN einzigen Knoten in der Wirkungsbeschreibung: \
         {ohne:?}. Entweder liest `knoten_fuer_zauber` ihre Felder nicht, oder ein neues \
         Wirkungsfeld ist noch nirgends eingeordnet.",
        ohne.len(),
        SPELLS.len()
    );
}

#[test]
fn jede_boss_zeitleiste_ist_beschrieben() {
    // # Die groesste Luecke der ersten Fassung
    //
    // Alle 35 Bosse fuehren eine `timeline`, und sie erzeugte **keinen
    // einzigen Knoten** — die Schlaege des Bosses, seine Kernmechanik, standen
    // in keiner Beschreibung.
    //
    // Auffallen konnte das nicht: Jeder Boss hatte trotzdem Knoten, allein
    // wegen `resistances`. Eine Zaehlung „35 von 35 Bossen beschrieben" war
    // richtig und wertlos, weil sie **Bosse** zaehlte statt **Wirkungen**.
    // Dieser Fall zaehlt Wirkungen.
    let mut fehlend: Vec<&str> = Vec::new();
    let mut erwartet = 0usize;
    for boss in rota_sim::data::BOSSES.iter() {
        if boss.timeline.is_empty() {
            continue;
        }
        erwartet += 1;
        let praefix = format!("{}/timeline:", boss.id);
        if !EFFECT_NODES.iter().any(|n| n.origin.starts_with(&praefix)) {
            fehlend.push(boss.id);
        }
    }
    assert!(
        erwartet > 0,
        "Kein Boss fuehrt eine Zeitleiste — dann prueft dieser Fall nichts."
    );
    assert!(
        fehlend.is_empty(),
        "{} von {erwartet} Bossen mit Zeitleiste haben keinen Knoten dafuer: {fehlend:?}",
        fehlend.len()
    );
}

#[test]
fn kein_knoten_beschreibt_eine_wirkung_die_es_nicht_gibt() {
    // # Der Fehler in die andere Richtung
    //
    // Die erste Fassung fragte `contains_key` statt nach dem Inhalt. Ergebnis:
    // Knoten fuer `energy: null` (29 von 33 Bossen), fuer `resistances: {}`
    // (31 von 35) und fuer `phases: []` (26 von 33) — lauter beschriebene
    // Wirkungen, die es nicht gibt.
    //
    // Das ist harmloser als eine Luecke, weil der konservative Weg nur langsam
    // ist und nicht falsch. Aber es macht die Beschreibung fuer ihren Zweck
    // unbrauchbar: „welche Ereigniskanaele braucht dieser Kampf" haette bei
    // fast jedem Boss alle genannt.
    let mut falsch: Vec<String> = Vec::new();
    for boss in rota_sim::data::BOSSES.iter() {
        let hat_knoten = |feld: &str| {
            EFFECT_NODES
                .iter()
                .any(|n| n.origin == format!("{}/{feld}", boss.id))
        };
        if boss.energy.is_none() && hat_knoten("energy") {
            falsch.push(format!("{}/energy", boss.id));
        }
        if boss.doom.is_none() && hat_knoten("doom") {
            falsch.push(format!("{}/doom", boss.id));
        }
        if boss.phases.is_empty() && hat_knoten("phases") {
            falsch.push(format!("{}/phases", boss.id));
        }
        if boss.resistances.is_empty() && hat_knoten("resistances") {
            falsch.push(format!("{}/resistances", boss.id));
        }
    }
    assert!(
        falsch.is_empty(),
        "{} Knoten beschreiben eine Wirkung, die der Boss gar nicht hat: {falsch:?}",
        falsch.len()
    );
}
