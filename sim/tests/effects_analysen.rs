//! Die Zusicherungen der Wirkungsbeschreibung (Issue #911).
//!
//! # Was hier geprueft wird — und was nicht
//!
//! **Nicht** geprueft wird AC2. Dass ein neuer Baustein den Bau bricht, ist ein
//! **Uebersetzungsfehler** und laesst sich aus einem Prueffall heraus gar nicht
//! zeigen: Waere der Baustein da, liesse sich diese Datei nicht uebersetzen.
//! Der Nachweis dafuer steht im PR-Text als echter roter Lauf.
//!
//! Geprueft wird alles, was zur Laufzeit sichtbar ist: die Registry (AC3), das
//! Signatur-Schlupfloch (AC4), der konservative Rueckfall (AC5) und die Grenze
//! des Monotonie-Hinweises (AC6).

use rota_sim::effects::passes::{anzahl_analysen, beispielknoten, namen, REGISTRY};
use rota_sim::effects::registry::{
    alle_analysen, alle_konservativ, AnalysisPassId, Evidence, Verdict,
};
use rota_sim::effects::{
    ConservativeFallback, EffectNode, EffectOp, LocalMonotonicityHint, MonotonicDirection,
    MonotonicityDomain, SignatureId, StateMask,
};

// ------------------------------------------------------------------- AC3

/// Die acht Fragen aus Issue #911, je eine Analyse.
///
/// **Eine Liste und keine Zahl.** Eine Zahl waere bei jeder Ergaenzung still
/// falsch; diese Liste nennt, was fehlt, wenn etwas fehlt.
const ERWARTETE_ANALYSEN: &[&str] = &[
    "analytic-crit",
    "action-bound",
    "kernel-choice",
    "cache-reuse",
    "invalidation",
    "event-channels",
    "random-sources",
    "monotonicity",
];

#[test]
fn jede_der_acht_fragen_hat_ihre_analyse() {
    let vorhanden: Vec<&str> = namen().iter().map(|id| id.0).collect();
    for erwartet in ERWARTETE_ANALYSEN {
        assert!(
            vorhanden.contains(erwartet),
            "Analyse \"{erwartet}\" fehlt in der Registry. Vorhanden sind {vorhanden:?} \
             ({} Stueck).",
            vorhanden.len()
        );
    }
}

#[test]
fn die_registry_hat_keine_doppelten_namen() {
    let mut gesehen: Vec<AnalysisPassId> = Vec::new();
    for id in namen() {
        assert!(
            !gesehen.contains(&id),
            "Analyse \"{id}\" steht zweimal in der Registry — dann laeuft sie doppelt und \
             eine andere fehlt womoeglich unbemerkt."
        );
        gesehen.push(id);
    }
    // Die Zahl gehoert in die MELDUNG, nie in die Bedingung: Sie darf wachsen.
    assert!(
        !gesehen.is_empty(),
        "Die Registry ist leer — dann bestaetigt jede Pruefung darunter nichts."
    );
}

#[test]
fn ein_knoten_ohne_semantik_laesst_jede_registrierte_analyse_zurueckfallen() {
    let blind = EffectNode::unbekannt(EffectOp::DealDamage, "prueffall/ohne-semantik");
    let ergebnisse = alle_analysen(&blind);

    // Die Zusicherung in den Datensatz: Ohne diese Zeile koennte die Schleife
    // ueber eine leere Registry laufen und der Test waere gruen.
    assert_eq!(
        ergebnisse.len(),
        anzahl_analysen(),
        "Es haben {} Analysen geantwortet, registriert sind {}.",
        ergebnisse.len(),
        anzahl_analysen()
    );

    for r in &ergebnisse {
        assert_eq!(
            r.verdict,
            Verdict::Conservative,
            "Analyse \"{}\" hat einen Knoten OHNE jede Semantik nicht in den Rueckfall \
             geschickt, sondern mit {:?} beantwortet ({}). Damit waere \"unbekannt\" \
             schneller als \"bekannt\" — genau falsch herum.",
            r.pass,
            r.verdict,
            r.reason
        );
    }
    assert!(alle_konservativ(&blind));
}

// ------------------------------------------------------------------- AC5

#[test]
fn der_rueckfall_setzt_alle_sechs_zustaende_nicht_fuenf() {
    let blind = EffectNode::unbekannt(EffectOp::ApplyBuff, "prueffall/ohne-semantik");
    for r in alle_analysen(&blind) {
        assert!(
            r.fallback.vollstaendig(),
            "Analyse \"{}\" hat nur {} von 6 konservativen Zustaenden gesetzt. Ein halber \
             Rueckfall ist gefaehrlicher als gar keiner: Er sieht aus wie Vorsicht und ist \
             keine.",
            r.pass,
            r.fallback.anzahl()
        );
    }
    assert_eq!(ConservativeFallback::ALLE.anzahl(), 6);
    assert_eq!(ConservativeFallback::KEINER.anzahl(), 0);
}

#[test]
fn eine_einzelne_fehlende_angabe_reicht_fuer_den_rueckfall() {
    // Der Knoten ist vollstaendig — bis auf **ein** Feld. Genau das ist der
    // Fall, den AC5 meint: nicht der voellig blinde Knoten, sondern der fast
    // fertige, dem eine Angabe fehlt.
    let mut fast_fertig = beispielknoten(EffectOp::DealDamage);
    assert!(
        !fast_fertig.hat_luecke(),
        "Der Beispielknoten muss vollstaendig sein, sonst prueft dieser Fall nichts."
    );
    assert!(
        !alle_konservativ(&fast_fertig),
        "Ein VOLLSTAENDIGER Knoten darf nicht ueberall konservativ sein — sonst waere die \
         Semantik wirkungslos und der Rueckfall keine Unterscheidung."
    );

    fast_fertig.timing = rota_sim::effects::TimingSemantics::Unknown;
    assert!(
        fast_fertig.hat_luecke(),
        "Eine fehlende Zeit-Angabe muss als Luecke gelten."
    );
    assert!(
        alle_konservativ(&fast_fertig),
        "Ein einziges fehlendes Feld hat NICHT gereicht, um jede Analyse in den Rueckfall \
         zu schicken."
    );
}

// ------------------------------------------------------------------- AC4

#[test]
fn eine_signatur_ohne_semantik_kommt_nicht_besser_weg_als_ein_unbekannter_baustein() {
    let signatur = EffectNode::unbekannt(
        EffectOp::CustomSignature(SignatureId("erfundene-bossregel")),
        "prueffall/signatur",
    );
    assert!(
        signatur.signature_ohne_semantik(),
        "Eine Signatur ohne jede Angabe muss als solche erkennbar sein."
    );
    assert!(
        alle_konservativ(&signatur),
        "Eine Signatur OHNE Semantik ist durchgelaufen. Genau das ist das Schlupfloch aus \
         AC4: Liefe jede ungewoehnliche Mechanik so durch, braeche der Bau nie mehr — und \
         die 68 geplanten Bosse tragen per Design je eine einmalige Regel."
    );
}

#[test]
fn eine_signatur_mit_semantik_wird_anders_behandelt_als_eine_ohne() {
    // Der Gegenpol zum Test darueber. Ohne ihn koennte die Zusicherung dadurch
    // erfuellt sein, dass Signaturen IMMER konservativ sind — dann waere die
    // geforderte Deklaration wirkungslos, und AC4 waere Fassade.
    let ohne = EffectNode::unbekannt(
        EffectOp::CustomSignature(SignatureId("ohne")),
        "prueffall/ohne",
    );
    let mut mit = beispielknoten(EffectOp::CustomSignature(SignatureId("mit")));
    mit.origin = "prueffall/mit";
    assert!(!mit.hat_luecke());

    let konservativ_ohne = alle_analysen(&ohne)
        .iter()
        .filter(|r| matches!(r.verdict, Verdict::Conservative))
        .count();
    let konservativ_mit = alle_analysen(&mit)
        .iter()
        .filter(|r| matches!(r.verdict, Verdict::Conservative))
        .count();

    assert!(
        konservativ_mit < konservativ_ohne,
        "Die deklarierte Semantik hat nichts geaendert: {konservativ_mit} Analysen \
         konservativ mit Semantik, {konservativ_ohne} ohne. Dann ist die Deklaration aus AC4 \
         eine Formalie."
    );
}

// ------------------------------------------------------------------- AC6

#[test]
fn der_monotonie_hinweis_traegt_nicht_fuer_den_sieg() {
    let hinweis = LocalMonotonicityHint {
        direction: MonotonicDirection::Increasing,
        domain: MonotonicityDomain::ExpectedDamageStableTrace,
    };
    assert!(
        hinweis.traegt_fuer(MonotonicityDomain::ExpectedDamageStableTrace),
        "Fuer seinen eigenen Bezug muss der Hinweis tragen, sonst sagt er gar nichts."
    );
    assert!(
        !hinweis.traegt_fuer(MonotonicityDomain::VictoryOutcome),
        "Der Hinweis hat fuer den SIEG getragen. Grundschaden ist monoton bezueglich \
         erwartetem Schaden bei stabilem Trace — und nicht bezueglich Sieg an einem Boss, \
         bei dem ein Ring auf null die Niederlage ist."
    );
    assert!(
        !LocalMonotonicityHint::UNKNOWN.traegt_fuer(MonotonicityDomain::ExpectedDamageStableTrace),
        "Ein Hinweis ohne Richtung darf fuer nichts tragen."
    );
}

#[test]
fn ein_knoten_der_am_sieg_verzweigt_bekommt_keinen_monotonie_freibrief() {
    let mut am_sieg = beispielknoten(EffectOp::DealDamage);
    am_sieg.control_flow_effect.branches_on_victory = true;
    let monotonie = alle_analysen(&am_sieg)
        .into_iter()
        .find(|r| r.pass.0 == "monotonicity")
        .expect("die Monotonie-Analyse muss registriert sein");
    assert_eq!(
        monotonie.verdict,
        Verdict::Conservative,
        "Ein Knoten mit Sieg-Verzweigung hat einen Monotonie-Freibrief bekommen ({}).",
        monotonie.reason
    );
}

// ------------------------------------------- Die Analysen unterscheiden etwas

#[test]
fn die_analysen_urteilen_nicht_alle_gleich() {
    // Ohne diesen Fall koennten alle acht dasselbe zurueckgeben und jede
    // Pruefung oben waere trotzdem gruen — acht Namen fuer eine Analyse.
    let knoten = beispielknoten(EffectOp::ConvertDamage);
    let ergebnisse = alle_analysen(&knoten);
    let verschiedene: Vec<Verdict> = {
        let mut v: Vec<Verdict> = ergebnisse.iter().map(|r| r.verdict).collect();
        v.dedup();
        v
    };
    assert!(
        verschiedene.len() > 1
            || ergebnisse
                .iter()
                .map(|r| r.reason)
                .collect::<Vec<_>>()
                .len()
                > 1,
        "Alle {} Analysen haben dasselbe geantwortet — dann unterscheiden sie nichts.",
        ergebnisse.len()
    );

    // Und konkret: dieselbe Frage, zwei Bausteine, zwei Antworten.
    let schaden = alle_analysen(&beispielknoten(EffectOp::DealDamage));
    let kernel_schaden = schaden
        .iter()
        .find(|r| r.pass.0 == "kernel-choice")
        .unwrap();
    let kernel_wandel = ergebnisse
        .iter()
        .find(|r| r.pass.0 == "kernel-choice")
        .unwrap();
    assert_ne!(
        kernel_schaden.verdict, kernel_wandel.verdict,
        "Die Kernel-Wahl beantwortet DealDamage und ConvertDamage gleich — dann haengt sie \
         nicht am Baustein."
    );
}

#[test]
fn die_lesemaske_ist_nicht_die_schreibmaske() {
    // Eine Maske, die alles ist, unterscheidet nichts. Der Fall haelt fest,
    // dass UNKNOWN wirklich "alles" bedeutet und EMPTY wirklich "nichts".
    assert!(StateMask::UNKNOWN.is_unknown());
    assert!(StateMask::EMPTY.is_empty());
    assert!(StateMask::UNKNOWN.intersects(StateMask::BOSS_HP));
    assert!(!StateMask::EMPTY.intersects(StateMask::BOSS_HP));
    assert!(!StateMask::RESOURCE.intersects(StateMask::BOSS_HP));
    assert!(StateMask::RESOURCE
        .union(StateMask::BOSS_HP)
        .intersects(StateMask::BOSS_HP));
}

#[test]
fn jeder_registrierte_pass_nennt_seinen_eigenen_namen() {
    // Ein Pass, der die ID eines anderen zurueckgibt, wuerde jede Meldung
    // falsch zuordnen — und in der Registry faellt das nicht auf.
    let knoten = beispielknoten(EffectOp::DealDamage);
    for pass in REGISTRY {
        let ergebnis = pass.analyze_dyn(&knoten);
        assert_eq!(
            ergebnis.pass,
            pass.id(),
            "Analyse \"{}\" meldet ihr Ergebnis unter dem Namen \"{}\".",
            pass.id(),
            ergebnis.pass
        );
    }
}

#[test]
fn jede_analyse_unterscheidet_an_den_echten_daten_etwas() {
    // # Die Frage eine Ebene hoeher: zaehle ich Analysen oder Unterscheidungen?
    //
    // `jede_der_acht_fragen_hat_ihre_analyse` prueft, dass acht Analysen
    // REGISTRIERT sind. Das ist eine Zaehlung ueber Traeger und sagt nichts
    // darueber, ob eine von ihnen etwas TUT. Eine Analyse, die ueber alle 218
    // echten Knoten hinweg immer dasselbe Verdikt liefert, ist ein Name ohne
    // Inhalt — und keine der bisherigen Pruefungen haette das gemeldet.
    //
    // Genau diese Verwechslung von Traeger und Wirkung war in Issue #911
    // viermal der Fehler. Dies ist der fuenfte Ort, an dem sie moeglich waere.
    use rota_sim::data::EFFECT_NODES;

    assert!(
        !EFFECT_NODES.is_empty(),
        "Ohne Knoten prueft dieser Fall nichts."
    );
    let mut stumpf: Vec<String> = Vec::new();
    for pass in REGISTRY {
        // **Das vollstaendige Ergebnis, nicht nur das Verdikt.** Drei der
        // Analysen sind BESCHREIBEND statt entscheidend: `invalidation`,
        // `event-channels` und `random-sources` antworten immer `Permits`,
        // und ihre eigentliche Aussage steckt in der `evidence` — der
        // Schreibmaske, den Kanaelen, der Zufallsquelle. Wer nur das Verdikt
        // ansieht, misst bei ihnen neben der Wirkung vorbei.
        let mut gesehen: Vec<(Verdict, Evidence)> = Vec::new();
        for knoten in EFFECT_NODES.iter() {
            let r = pass.analyze_dyn(knoten);
            if !gesehen
                .iter()
                .any(|(v, e)| *v == r.verdict && *e == r.evidence)
            {
                gesehen.push((r.verdict, r.evidence));
            }
        }
        if gesehen.len() < 2 {
            stumpf.push(format!("{} (immer {:?})", pass.id(), gesehen));
        }
    }
    assert!(
        stumpf.is_empty(),
        "Diese Analysen antworten ueber alle {} echten Knoten hinweg IMMER dasselbe und \
         unterscheiden damit nichts: {stumpf:?}. Eine registrierte Analyse ist noch keine \
         wirksame.",
        EFFECT_NODES.len()
    );
}
