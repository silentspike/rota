//! Austauschbarkeit von Treffern (Issue #915, AC2).
//!
//! # Warum das überhaupt eine Frage ist
//!
//! Die Zahl der kritischen Treffer ist binomialverteilt, also genügten `n+1`
//! Fälle statt `2ⁿ` Folgen — **wenn** die Treffer austauschbar sind. Bei
//! Lebenspunkt-Phasen sind sie es nicht:
//!
//! ```text
//! Krit auf Treffer 1  ->  Schwelle früher überschritten  ->  Phase wechselt VOR Treffer 2
//! Krit auf Treffer 3  ->  Treffer 2 läuft noch in der alten Phase
//! ```
//!
//! Gleiche Kritzahl, verschiedener Kontrollfluss. Eine Gruppierung ohne Nachweis
//! wäre eine stille Näherung, die sich „exakt" nennt.
//!
//! # Was hier steht und was noch nicht
//!
//! Von den sechs Bedingungen aus AC2 sind vier aus der Wirkungsbeschreibung
//! ableitbar. Zwei sind es nicht, und das ist gemessen, nicht vermutet:
//!
//! | Bedingung | ableitbar aus | Stand |
//! | --- | --- | --- |
//! | `same_transition_except_reward` | `op` | ja |
//! | `no_phase_or_kill_check_between_hits` | `control_flow_effect` | ja |
//! | `no_resource_or_timing_feedback` | `writes`, `timing` | ja |
//! | `no_interleaving_observable_event` | `writes` | ja, konservativ |
//! | `same_target_and_mitigation` | `target` — und für die Minderung **nichts** | **nicht ableitbar** |
//! | `no_crit_triggered_effect` | `trigger` — es gibt keinen Krit-Auslöser | **leer** |
//!
//! Die Minderung (`resistance`, `vulnerability`) hat in `crate::effects` kein
//! Gegenstück, variiert aber im Kampf: `vulnerability` über DECAY-Stapel,
//! `resistance` je Zauber.
//!
//! **Der Auftraggeber hat das am 2026-08-19 entschieden** (`f-915-ac2`,
//! konservativ): Das Zertifikat wird für die nicht ableitbaren Teile **nicht**
//! ausgestellt, die Gruppierung greift nirgends, die sequenzielle
//! Verteilungs-DP trägt allein. Der Gewinn aus AC2 (`n+1` statt `2ⁿ`) fällt
//! damit **absichtlich** aus — nicht weil die Umsetzung fehlt, sondern weil die
//! Beschreibung die nötige Angabe nicht hat.
//!
//! Das ist **keine offene Baustelle**. Wer die Gruppierung doch will, muss der
//! Wirkungsbeschreibung ein Minderungs-Feld geben (Erweiterung von #911) und
//! die Entscheidung neu stellen.
//!
//! Der leere Fall ist dagegen **bewacht**, siehe [`kann_durch_krit_ausgeloest_werden`].

use crate::effects::{ControlFlowEffect, EffectNode, EffectOp, StateMask, Trigger};

/// Kann dieser Auslöser durch einen kritischen Treffer ausgelöst werden?
///
/// # Der Wächter für eine Bedingung, die es noch nicht gibt
///
/// AC2 verlangt `no_crit_triggered_effect`. Heute ist die Bedingung **immer**
/// wahr, weil [`Trigger`] gar keinen Krit-Auslöser kennt — und eine Bedingung,
/// die nie falsch werden kann, ist keine Bedingung.
///
/// Das `match` hat deshalb **keinen Auffangzweig**: Führt jemand `OnCrit` ein,
/// bricht hier die Übersetzung, statt dass die Austauschbarkeit still
/// weiterbehauptet wird. Dasselbe Muster wie bei
/// [`crate::verteilung::Quellbereich::kennzahl`] und in `durchgaenge::zieht`.
///
/// `Unknown` antwortet **wahr**: Wer nichts angibt, darf nicht als harmlos
/// gelten.
#[must_use]
pub const fn kann_durch_krit_ausgeloest_werden(ausloeser: Trigger) -> bool {
    match ausloeser {
        Trigger::OnCast => false,
        Trigger::OnTick => false,
        Trigger::OnExpire => false,
        Trigger::OnTimeline => false,
        Trigger::OnPhaseChange => false,
        // Keine Angabe heisst nicht "harmlos".
        Trigger::Unknown => true,
    }
}

/// Was die Beschreibung über die Austauschbarkeit hergibt.
///
/// **Kein `bool`, sondern die Einzelbefunde.** Ein einzelnes Ja/Nein verlöre
/// genau die Auskunft, die man braucht, wenn es Nein lautet: welche der
/// Bedingungen gefallen ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Befunde {
    /// Alle Treffer machen denselben Übergang, bis auf den Betrag.
    pub gleicher_uebergang: bool,
    /// Zwischen den Treffern wird weder Phase noch Sieg geprüft.
    pub keine_phasen_oder_siegpruefung: bool,
    /// Kein Effekt hängt am Krit-Ergebnis.
    pub kein_krit_ausgeloester_effekt: bool,
    /// Keine Rückwirkung auf Ressource oder Zeit.
    pub keine_ressourcen_rueckwirkung: bool,
    /// Kein beobachtbares Ereignis dazwischen.
    pub kein_dazwischenliegendes_ereignis: bool,
    /// Gleiches Ziel **und gleiche Minderung** — die sechste Bedingung aus AC2.
    ///
    /// **Nicht ableitbar**, und `None` ist der **entschiedene** Zustand, keine
    /// offene Frage: Die Wirkungsbeschreibung hat für die Minderung kein
    /// Gegenstück, obwohl sie im Kampf variiert — `vulnerability` über
    /// DECAY-Stapel (`fight.rs:3164`, `:3330`), `resistance` je Zauber
    /// (`fight.rs:3792`).
    ///
    /// Der Auftraggeber hat am 2026-08-19 den **konservativen** Ausgang gewählt
    /// (`f-915-ac2`): Das Zertifikat wird für die nicht ableitbaren Teile nicht
    /// ausgestellt, die Gruppierung greift nirgends, die sequenzielle
    /// Verteilungs-DP trägt allein. Der AC2-Gewinn (`n+1` statt `2ⁿ`) fällt
    /// damit **absichtlich** aus.
    ///
    /// Das Feld steht in der Struktur und nicht nur im Kommentar, weil sonst
    /// niemand **sieht**, dass AC2 sechs Bedingungen kennt und dieser Verbund
    /// fünf beantwortet. Eine Lücke, die man nur bemerkt, wenn man die richtige
    /// Methode aufruft, ist keine sichtbare Lücke.
    pub same_target_and_mitigation: Option<bool>,
}

impl Befunde {
    /// Ob **alle** Bedingungen erfüllt sind, die sich ableiten lassen.
    ///
    /// **Das genügt für das Zertifikat NICHT.** `same_target_and_mitigation`
    /// fehlt hier, weil die Minderung in der Beschreibung kein Gegenstück hat —
    /// wer diese Methode für „austauschbar" nimmt, überspringt genau die
    /// Bedingung, die nicht geprüft werden kann.
    #[must_use]
    pub const fn alle_ableitbaren_erfuellt(&self) -> bool {
        self.gleicher_uebergang
            && self.keine_phasen_oder_siegpruefung
            && self.kein_krit_ausgeloester_effekt
            && self.keine_ressourcen_rueckwirkung
            && self.kein_dazwischenliegendes_ereignis
    }
}

/// Die ableitbaren Bedingungen für eine Gruppe von Knoten prüfen.
#[must_use]
pub fn befunde(knoten: &[EffectNode]) -> Befunde {
    let schreibt_ressource_oder_zeit = StateMask::RESOURCE
        .union(StateMask::COOLDOWNS)
        .union(StateMask::GCD);
    let beobachtbar = StateMask::PHASE
        .union(StateMask::TIMELINE)
        .union(StateMask::BOSS_SYSTEMS);

    Befunde {
        gleicher_uebergang: knoten
            .iter()
            .all(|n| matches!(n.op, EffectOp::DealDamage | EffectOp::ApplyDot)),
        keine_phasen_oder_siegpruefung: knoten.iter().all(|n| {
            let ControlFlowEffect {
                branches_on_hp,
                branches_on_victory,
                unknown,
                ..
            } = n.control_flow_effect;
            !branches_on_hp && !branches_on_victory && !unknown
        }),
        kein_krit_ausgeloester_effekt: knoten
            .iter()
            .all(|n| !kann_durch_krit_ausgeloest_werden(n.trigger)),
        keine_ressourcen_rueckwirkung: knoten
            .iter()
            .all(|n| !n.writes.intersects(schreibt_ressource_oder_zeit)),
        kein_dazwischenliegendes_ereignis: knoten.iter().all(|n| !n.writes.intersects(beobachtbar)),
        // Nicht ableitbar, siehe Feld-Dokumentation. Bewusst nicht `Some(true)`
        // aus Bequemlichkeit und nicht `Some(false)` aus Vorsicht — beides wäre
        // eine Behauptung über etwas, das die Beschreibung nicht hergibt.
        same_target_and_mitigation: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::EFFECT_NODES;

    /// **Der Wächter: heute ist die Bedingung leer, und das ist festgehalten.**
    #[test]
    fn kein_ausloeser_haengt_heute_am_krit() {
        let alle = [
            Trigger::OnCast,
            Trigger::OnTick,
            Trigger::OnExpire,
            Trigger::OnTimeline,
            Trigger::OnPhaseChange,
        ];
        for t in alle {
            assert!(
                !kann_durch_krit_ausgeloest_werden(t),
                "{t:?} gilt als krit-ausgelöst — dann stimmt die Zuordnung nicht"
            );
        }
        assert!(
            kann_durch_krit_ausgeloest_werden(Trigger::Unknown),
            "Unknown MUSS konservativ wahr sein — sonst gilt eine fehlende \
             Angabe als harmlos"
        );
    }

    /// **Die Gegenprobe zur Leere: Unknown macht die Bedingung falsch.**
    ///
    /// Ohne sie prüfte der Fall oben nur, dass eine Funktion immer `false`
    /// liefert — und genau das wäre die Bedingung, die nie falsch werden kann.
    #[test]
    fn mit_unbekanntem_ausloeser_faellt_die_bedingung() {
        let mut knoten = EFFECT_NODES
            .iter()
            .find(|n| matches!(n.op, EffectOp::DealDamage))
            .copied()
            .expect("es gibt Schadensknoten");

        knoten.trigger = Trigger::OnCast;
        assert!(
            befunde(&[knoten]).kein_krit_ausgeloester_effekt,
            "Trägerzusicherung: mit einem gewöhnlichen Auslöser hält die Bedingung"
        );

        knoten.trigger = Trigger::Unknown;
        assert!(
            !befunde(&[knoten]).kein_krit_ausgeloester_effekt,
            "mit unbekanntem Auslöser MUSS die Bedingung fallen"
        );
    }

    /// **Die sechste Bedingung bleibt `None` — das ist entschieden, nicht offen.**
    ///
    /// Der Fall hält den Zustand fest, statt ihn zu verstecken. Er wird rot,
    /// sobald jemand das Feld setzt — und das ist gewollt: Wer die Minderung
    /// ableitbar macht, kippt damit eine Entscheidung des Auftraggebers
    /// (`f-915-ac2`, konservativ) und muss diesen Fall bewusst anfassen.
    #[test]
    fn die_minderung_ist_nicht_ableitbar_und_bleibt_es() {
        let knoten = EFFECT_NODES
            .iter()
            .find(|n| matches!(n.op, EffectOp::DealDamage))
            .copied()
            .expect("es gibt Schadensknoten");

        let b = befunde(&[knoten]);
        assert_eq!(
            b.same_target_and_mitigation, None,
            "die Minderung ist nicht ableitbar, also MUSS das Feld None sein — \
             weder Some(true) noch Some(false). Wer das ändert, kippt die \
             Entscheidung f-915-ac2 (konservativ)"
        );

        // **Und die Bedingung fällt NICHT auf die anderen fünf durch.** Ein
        // Knoten, der alle ableitbaren Bedingungen erfüllt, ist deshalb noch
        // lange nicht austauschbar — genau das ist der Grund, warum die Methode
        // `alle_ableitbaren_erfuellt` heißt und nicht `austauschbar`.
        let mit_allen_ableitbaren = Befunde {
            gleicher_uebergang: true,
            keine_phasen_oder_siegpruefung: true,
            kein_krit_ausgeloester_effekt: true,
            keine_ressourcen_rueckwirkung: true,
            kein_dazwischenliegendes_ereignis: true,
            same_target_and_mitigation: None,
        };
        assert!(
            mit_allen_ableitbaren.alle_ableitbaren_erfuellt(),
            "die Methode antwortet ja, obwohl die sechste Bedingung ungeklärt ist"
        );
        assert_eq!(
            mit_allen_ableitbaren.same_target_and_mitigation, None,
            "und genau darin liegt der Unterschied zwischen 'alle ableitbaren' \
             und 'austauschbar'"
        );
    }

    /// Wie weit die ableitbaren Bedingungen im Bestand tragen.
    ///
    /// Die Zahlen stehen in der **Meldung**, nicht in der Bedingung — sie ändern
    /// sich mit den Daten, und ein festgenagelter Wert wäre still falsch.
    #[test]
    fn im_bestand_tragen_die_ableitbaren_bedingungen_nicht_ueberall() {
        let schadensknoten: Vec<_> = EFFECT_NODES
            .iter()
            .filter(|n| matches!(n.op, EffectOp::DealDamage | EffectOp::ApplyDot))
            .copied()
            .collect();
        assert!(
            !schadensknoten.is_empty(),
            "Trägerzusicherung: es gibt Schadensknoten"
        );

        let mut erfuellt = 0;
        for n in &schadensknoten {
            if befunde(&[*n]).alle_ableitbaren_erfuellt() {
                erfuellt += 1;
            }
        }

        // Zweiseitig: Wären ALLE erfüllt, prüfte die Ableitung nichts.
        assert!(
            erfuellt < schadensknoten.len(),
            "alle {} Schadensknoten erfüllen alles — dann unterscheidet die \
             Ableitung nicht",
            schadensknoten.len()
        );
        println!(
            "{erfuellt} von {} Schadensknoten erfüllen die ableitbaren Bedingungen",
            schadensknoten.len()
        );
    }
}
