//! `boss.casting` und `boss.cast_remains` (Issue #797).
//!
//! **Was ohne diese Werte fehlte.** Die Suchsprache kannte keinen laufenden
//! Boss-Zauber. Damit konnte keine Rotation — weder eine gefundene noch eine
//! von Hand geschriebene — gezielt unterbrechen: `QUENCH` und `PIN` waren nur
//! über `time >= 15000` ansteuerbar, also über eine fest verdrahtete Zahl aus
//! der Zeitleiste eines einzelnen Bosses. Der Spieler dagegen sieht den Cast
//! (`src/render/hud.ts`) und kann antworten.
//!
//! **Die Prüfung fasst den Wert am Gegner an, nicht am Parser.** Dass sich
//! `boss.casting` schreiben lässt, sagt nichts darüber, ob dort jemals eine 1
//! steht. Deshalb läuft hier ein echter Kampf gegen einen Boss mit einem
//! unterbrechbaren Zauber, und die Rotation wird an ihrer WIRKUNG gemessen:
//! Sie unterbricht, oder sie tut es nicht.

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::{Fight, FightConfig};

/// `boss-1-2` trägt „Fusion Call" mit 3000 ms Wirkzeit — sonst gäbe es nichts
/// zu messen. Steht hier je ein Boss ohne solchen Zauber, bleiben alle Zahlen
/// null und die Datei wäre grün, ohne etwas zu prüfen.
///
/// **Der erste Entwurf stand auf `test-caster`** — dem Namen nach der
/// naheliegende Boss, und er trägt keinen einzigen Zauber mit Wirkzeit. Gefunden
/// hat das nicht ein roter Wirkungsnachweis, sondern die Träger-Zusicherung
/// unten: Ohne sie hätten hier vier Nullen gestanden, jede korrekt gemessen und
/// keine mit Aussage.
const BOSS: &str = "boss-1-2";

/// Wie oft ein Zauber gewirkt wurde.
///
/// **`kindle` und nicht `quench`.** Der erste Entwurf maß mit dem Unterbrecher,
/// weil er die Sache betrifft — und bekam überall Null. Ursache war nicht der
/// neue Wert, sondern `quench` selbst: Es trägt `triggersGcd: false` und
/// verhält sich im Cast-Fenster anders. Ein Zauber mit Sonderverhalten taugt
/// nicht als Messfühler für etwas anderes.
fn wuerfe(text: &str, spell: &str) -> u32 {
    let apl: Apl = parse(AplName("probe"), text)
        .unwrap_or_else(|error| panic!("Prioritätenliste ist fehlerhaft:\n{error}"));
    let index = SPELLS
        .iter()
        .position(|s| s.id == spell)
        .unwrap_or_else(|| panic!("Zauber {spell} gibt es nicht"));
    Fight::new(FightConfig {
        boss: boss_by_id(BOSS).expect("Prüf-Boss fehlt"),
        stop_on_kill: false,
        ..FightConfig::default()
    })
    .run_apl(&apl)
    .casts[index]
}

/// **Träger-Zusicherung, und sie steht bewusst vorn.**
///
/// Alles Weitere misst, ob eine Bedingung auf den Boss-Zauber anspricht. Wirkt
/// der Boss in diesem Aufbau nie, wäre jede folgende Zahl richtig gemessen und
/// trotzdem nichtssagend.
///
/// **Der erste Entwurf stand auf `test-caster`** — dem Namen nach der
/// naheliegende Boss, und er trägt keinen einzigen Zauber mit Wirkzeit. Gefunden
/// hat das nicht ein roter Wirkungsnachweis, sondern dieser Fall.
#[test]
fn ac1_der_pruefboss_wirkt_ueberhaupt_einen_unterbrechbaren_zauber() {
    let boss = boss_by_id(BOSS).expect("Prüf-Boss fehlt");
    let unterbrechbar = boss.timeline.iter().filter(|e| e.cast_time_ms > 0).count();
    assert!(
        unterbrechbar > 0,
        "{BOSS} trägt keinen Zauber mit Wirkzeit — dann kann keine Zeile dieser \
         Datei etwas belegen, und alle Zahlen weiter unten wären wertlos."
    );
}

/// `boss.casting` schränkt ein — sonst wäre der Wert immer wahr und die
/// Bedingung folgenlos.
#[test]
fn ac1_boss_casting_schraenkt_auf_das_cast_fenster_ein() {
    let immer = wuerfe("kindle\n", "kindle");
    let im_fenster = wuerfe("kindle if boss.casting\npurge\n", "kindle");
    assert!(
        im_fenster > 0 && im_fenster < immer,
        "Ohne Bedingung {immer} Würfe, mit `if boss.casting` {im_fenster}. \
         Gleich viele hieße: der Wert ist immer wahr. Null hieße: er ist es nie. \
         Beides wäre kein Nachweis."
    );
}

/// **Die Kontrolle, ohne die der Nachweis nichts wert wäre.**
///
/// Dass `boss.casting` einschränkt, könnte auch an einer beliebigen anderen
/// Ursache liegen. Erst der Vergleich mit einem von Hand gesetzten Zeitfenster
/// um denselben Cast zeigt, dass der Wert genau DAS liest.
///
/// Die Kontrolle ist bewusst die Fassung, die #797 als Auflage verbietet
/// („keine fest verdrahteten Zeiten"): Sie darf messen, was gebaut werden soll,
/// aber sie darf es nicht ersetzen.
#[test]
fn ac1_beide_werte_treffen_dasselbe_fenster_wie_die_zeitangabe() {
    let ueber_casting = wuerfe("kindle if boss.casting\npurge\n", "kindle");
    let ueber_restzeit = wuerfe("kindle if boss.cast_remains > 0\npurge\n", "kindle");
    // "Fusion Call" beginnt bei 25000 ms und läuft 3000 ms.
    let ueber_zeit = wuerfe(
        "kindle if time >= 25000 and time < 28000\npurge\n",
        "kindle",
    );
    assert_eq!(
        (ueber_casting, ueber_restzeit),
        (ueber_zeit, ueber_zeit),
        "casting {ueber_casting}, cast_remains {ueber_restzeit}, Zeitfenster \
         {ueber_zeit}. Weichen sie ab, liest mindestens einer der beiden Werte \
         etwas anderes als den laufenden Boss-Zauber."
    );
}

/// **0, wenn nichts läuft** — die unbequeme Übereinkunft aus `Value`, hier
/// festgehalten. Ändert sie jemand auf einen großen Ersatzwert, bricht dieser
/// Fall, und das ist beabsichtigt: Jede bestehende Zeile mit
/// `cast_remains < X` bekäme sonst stillschweigend eine andere Bedeutung.
#[test]
fn ac1_cast_remains_ist_null_wenn_nichts_laeuft() {
    let fight = Fight::new(FightConfig {
        boss: boss_by_id(BOSS).expect("Prüf-Boss fehlt"),
        ..FightConfig::default()
    });
    assert_eq!(
        fight.boss_cast_remaining_ms(),
        0,
        "Vor dem ersten Zauber muss die Restzeit 0 sein, wie bei einem \
         erloschenen Brand. Ein großer Ersatzwert wäre bequemer, brächte aber \
         zwei Bedeutungen von \"remains\" in dieselbe Sprache."
    );
}
