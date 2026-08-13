//! Welches oeffentliche Symbol in `sim/src` benutzt niemand? (Issue #815)
//!
//! # Warum `dead_code` das nicht leistet
//!
//! `rota-sim` ist eine reine Bibliothek — `src/lib.rs`, kein `main.rs`, keine
//! `[[bin]]`-Sektion. Der Uebersetzer haelt jedes `pub` fuer von aussen
//! nutzbar und schweigt. Gemessen:
//!
//! ```text
//!   pub fn  unbenutzt   cargo clippy --all-targets -- -D warnings   Exit 0
//!   fn      unbenutzt   derselbe Befehl                             Exit 101
//! ```
//!
//! Der private Fall belegt, dass die Kette lief; der Unterschied liegt allein
//! am `pub`. Genau diese Luecke schliesst diese Datei — das Gegenstueck zu
//! `tests/unit/tote-symbole-waechter.test.ts` auf der TypeScript-Seite (#813).
//!
//! # Warum ein Syntaxbaum und kein Zeilenmuster
//!
//! Bei der Messung zu diesem Issue hat eine Textsuche **dreimal** falsch
//! gezaehlt, und alle drei stehen unten als eigener Prueffall:
//!
//! ```text
//!   1  `examples/` wurde gar nicht gelesen           21 statt 20
//!   2  eine lokale Variable `label` galt als Nutzung
//!      von `Origin::label`                           19 statt 20
//!   3  das Dateimuster [a-z/._-]+ kannte keine
//!      Ziffern, parity_instance2_boss4.rs fiel raus  113 statt 126
//! ```
//!
//! Fall 2 loest ein Syntaxbaum **nicht von selbst**: Er kennt Bezeichner, aber
//! nicht ihre Aufloesung. Deshalb zaehlt dieser Waechter nicht Namen, sondern
//! **Stellen, an denen ein Name benutzt wird** — Methodenaufruf, Pfadsegment,
//! Aufruf. Eine Bindung (`let label = …`, `for (label, …) in …`) ist keine.
//!
//! # Was er NICHT leistet, benannt statt verschwiegen
//!
//! Er loest keine Typen auf. Ruft eine andere Kiste eine gleichnamige Methode
//! eines fremden Typs, zaehlt er das als Nutzung — die Richtung des Irrtums
//! ist bewusst gewaehlt: lieber ein totes Symbol uebersehen als eines melden,
//! das lebt. Ein Fehlalarm kostet Vertrauen, eine Luecke kostet einen zweiten
//! Fund.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use syn::visit::Visit;

/// Wo ein oeffentliches Symbol steht.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Stelle {
    datei: String,
    name: String,
    art: &'static str,
}

impl Stelle {
    fn schluessel(&self) -> String {
        format!("{}#{}", self.datei, self.name)
    }
}

/// Der Grund der vier Werkzeuge fuer Messung 1 — **einmal, nicht viermal**.
///
/// Vier Symbole mit demselben Grund sind ein Muster, kein Zufall. Stuende er
/// vier Mal wortgleich da, muesste er auch vier Mal geaendert werden, wenn die
/// Messung laeuft — und beim dritten Mal vergisst es jemand. Die Gruppe traegt
/// ihn deshalb gemeinsam.
const MESSUNG_1: &str = "Werkzeug fuer Messung 1 (bester Satz- gegen besten Affix-Aufbau, \
     GAME-DESIGN 13.5). Die Blockade aus dem Kopf von gear_balance.rs ist aufgehoben: \
     CATALYSIS wirkt (combat.ts:3608, fight.rs:3455), PURITY wirkt (gear.rs:431, \
     fight.rs:3517), und das Spiel speist worn aus getragener Ausruestung \
     (app.ts:232). Gebraucht fuer #791 — ohne die Ausruestungs-Skalierung ist \
     instance-3 rechnerisch unbezwingbar.";

/// Was heute niemand benutzt und trotzdem stehen bleibt — mit seinem Grund.
///
/// **Jede Zeile nennt ihren Vorgang.** Eine Ausnahmeliste ohne Begruendung
/// wird zur Halde: Nach dem dritten Eintrag weiss niemand mehr, ob eine Zeile
/// ein offener Rest oder ein vergessener Fehler ist.
///
/// # Die vier aus der Gruppe waeren um ein Haar geloescht worden
///
/// Bei #815 galten sie als toter Code, und die Begruendung war ein Kommentar
/// im Kopf von `gear_balance.rs`, der einen Zustand von vor neun Tagen
/// beschrieb. Sie waren bereits entfernt, als die Behauptung nachgemessen
/// wurde. Deshalb steht in [`MESSUNG_1`] nicht „wird noch gebraucht", sondern
/// **wo** die Verdrahtung steht, die es belegt.
fn ausnahmen() -> BTreeMap<&'static str, &'static str> {
    [
        // ---- Werkzeuge fuer Messung 1, ein gemeinsamer Grund ----------------
        //
        // **Es sind DREI, nicht vier.** `set_build_candidate` stand in der
        // Vorgabe mit auf der Liste — es war tot, solange `tipping_set_bonus`
        // geloescht war, und lebt wieder, seit dieses zurueck ist. Gemessen,
        // nicht gezaehlt: Der Fall „die Ausnahmeliste ueberlebt nicht, was sie
        // beschreibt" haette es sonst sofort gemeldet.
        ("src/best_build.rs#best_affix_build", MESSUNG_1),
        ("src/best_build.rs#tipping_set_bonus", MESSUNG_1),
        ("src/sources.rs#set_build", MESSUNG_1),
        // ---- Gemeinsamer Namensraum mit dem Spiel ---------------------------
        //
        // Jede traegt im Code den Verweis auf ihr Gegenstueck in
        // `src/gear/sets.ts`. Die Zeichenkette IST die Schnittstelle: Wer sie
        // hier entfernt, bricht den Abgleich, ohne dass etwas rot wird.
        (
            "src/sets.rs#SET_EFFECT_WHOLE_WORK",
            "Gegenstueck in src/gear/sets.ts",
        ),
        (
            "src/sets.rs#SET_EFFECT_SLOW_ROT",
            "Gegenstueck in src/gear/sets.ts",
        ),
        (
            "src/sets.rs#SET_EFFECT_SEALED_VESSEL",
            "Gegenstueck in src/gear/sets.ts",
        ),
        (
            "src/sets.rs#SET_EFFECT_LONG_GRIP",
            "Gegenstueck in src/gear/sets.ts",
        ),
        (
            "src/sets.rs#SET_EFFECT_PERPETUAL_DRAFT",
            "Gegenstueck in src/gear/sets.ts",
        ),
        // ---- Glied einer vollstaendigen Reihe -------------------------------
        //
        // Sie stehen neben Geschwistern, die benutzt werden. Eine Reihe, bei
        // der ein Glied fehlt, laedt zum Nachrechnen ein — und das Nachrechnen
        // ist die Fehlerquelle, nicht die ungenutzte Zeile.
        (
            "src/gear.rs#POOL_YIELD",
            "Glied der Topf-Reihe, neben POOL_ESSENCE",
        ),
        (
            "src/gear.rs#POOL_ACTIVE",
            "Glied der Topf-Reihe, neben POOL_ESSENCE",
        ),
        (
            "src/gear.rs#QUALITY_RARE",
            "Glied der Qualitaets-Reihe, neben QUALITY_EPIC",
        ),
        (
            "src/gear.rs#QUALITY_LEGENDARY",
            "Glied der Qualitaets-Reihe, neben QUALITY_EPIC",
        ),
        (
            "src/sources.rs#SOURCE_TALENTS",
            "Glied der Quellen-Reihe, neben SOURCE_AFFIXES",
        ),
        (
            "src/sources.rs#SOURCE_LEGENDARY",
            "Glied der Quellen-Reihe, neben SOURCE_AFFIXES",
        ),
        (
            "src/sources.rs#SOURCE_SETS",
            "Glied der Quellen-Reihe, neben SOURCE_AFFIXES",
        ),
        (
            "src/sources.rs#SET_VALUE_TIERS",
            "Glied der Satz-Reihe, neben SET_TIERS",
        ),
    ]
    .into_iter()
    .collect()
}

// ---------------------------------------------------------------- Einsammeln

/// Sammelt die oeffentlichen Symbole einer Datei.
struct Oeffentliche {
    datei: String,
    gefunden: Vec<Stelle>,
}

impl<'ast> Visit<'ast> for Oeffentliche {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        if matches!(i.vis, syn::Visibility::Public(_)) {
            self.nimm(i.sig.ident.to_string(), "fn");
        }
        syn::visit::visit_item_fn(self, i);
    }

    fn visit_item_const(&mut self, i: &'ast syn::ItemConst) {
        if matches!(i.vis, syn::Visibility::Public(_)) {
            self.nimm(i.ident.to_string(), "const");
        }
        syn::visit::visit_item_const(self, i);
    }

    fn visit_item_static(&mut self, i: &'ast syn::ItemStatic) {
        if matches!(i.vis, syn::Visibility::Public(_)) {
            self.nimm(i.ident.to_string(), "static");
        }
        syn::visit::visit_item_static(self, i);
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        // **Nur Methoden in inhaerenten `impl`-Bloecken.** Eine Methode aus
        // `impl Trait for Typ` wird ueber das Trait gerufen; ihr Name kaeme
        // im Quelltext womoeglich gar nicht vor, und der Waechter meldete sie
        // faelschlich. `visit_item_impl` unten filtert das.
        if matches!(i.vis, syn::Visibility::Public(_)) {
            self.nimm(i.sig.ident.to_string(), "methode");
        }
    }

    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        if i.trait_.is_none() {
            syn::visit::visit_item_impl(self, i);
        }
    }
}

impl Oeffentliche {
    fn nimm(&mut self, name: String, art: &'static str) {
        self.gefunden.push(Stelle {
            datei: self.datei.clone(),
            name,
            art,
        });
    }
}

/// Sammelt die **Nutzungsstellen** einer Datei.
///
/// Der Unterschied zu „alle Bezeichner" ist der Kern dieses Waechters. Gezaehlt
/// wird nur, was eine Benutzung IST:
///
/// * `x.name()` — Methodenaufruf
/// * `Typ::name` oder `modul::name` — Pfad mit mehr als einem Segment
/// * `name(…)` — Aufruf ueber einen einfachen Pfad
///
/// **Nicht gezaehlt** wird eine Bindung: `let label = …`, `for (label, …) in …`,
/// ein Parametername. Genau daran ist meine erste Messung gescheitert.
#[derive(Default)]
struct Nutzungen {
    namen: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for Nutzungen {
    fn visit_expr_method_call(&mut self, i: &'ast syn::ExprMethodCall) {
        self.namen.insert(i.method.to_string());
        syn::visit::visit_expr_method_call(self, i);
    }

    fn visit_path(&mut self, i: &'ast syn::Path) {
        // Jedes Segment zaehlt: `crate::sets::SET_EFFECT_WHOLE_WORK` nennt das
        // Modul UND die Konstante, und beide sind Nutzungen.
        //
        // Ein einsegmentiger Pfad ist ebenfalls eine Nutzung — `spread_factor(…)`
        // nach `use`. Er ist NICHT dasselbe wie eine Bindung: Eine Bindung ist
        // ein `Pat`, und `visit_path` sieht sie nicht.
        for segment in &i.segments {
            self.namen.insert(segment.ident.to_string());
        }
        syn::visit::visit_path(self, i);
    }

    fn visit_expr_field(&mut self, i: &'ast syn::ExprField) {
        if let syn::Member::Named(name) = &i.member {
            self.namen.insert(name.to_string());
        }
        syn::visit::visit_expr_field(self, i);
    }

    /// **Ein `use` ist eine Nutzung — und das hat gefehlt.**
    ///
    /// `use rota_sim::rng::{RandomStream, Streams, STREAM_COUNT};` ist ein
    /// `UseTree`, **kein** `Path`. `visit_path` sieht ihn nicht.
    ///
    /// Gefunden hat es die Träger-Zusicherung beim ersten Lauf: Sie meldete
    /// „STREAM_COUNT lebt und wird trotzdem gemeldet". Die Konstante wird in
    /// `tests/parity_primitives.rs` importiert und dort über den blossen Namen
    /// benutzt — der Import war die einzige Stelle, an der ihr voller Name
    /// steht, und genau die war unsichtbar.
    ///
    /// Ohne diese Ergänzung meldete der Wächter 21 Symbole statt 16, darunter
    /// ein Dutzend Methoden, die über importierte Typen gerufen werden.
    /// **Makro-Argumente sind Tokens, kein Baum — und das hat gefehlt.**
    ///
    /// `assert_eq!(buffs.active_count(now), …)` und
    /// `println!("{}", origin.label())` sind für `syn` ein `Macro` mit einem
    /// undurchdrungenen `TokenStream`. Kein `visit_expr_method_call` greift
    /// dort.
    ///
    /// Der erste Lauf meldete deshalb **zwölf lebende Methoden als tot** —
    /// alle zwölf werden ausschliesslich in Makros gerufen, weil Tests
    /// `assert_eq!` benutzen und Beispiele `println!`.
    ///
    /// Gelesen wird die Token-Ebene, **nicht der Text**: `TokenTree::Ident`
    /// ist ein echter Bezeichner, kein Treffer in einer Zeichenkette. Ein
    /// Name in einem Zeichenketten-Literal (`"active_count"`) zählt damit
    /// weiterhin nicht — genau der Unterschied, an dem die Messung zu #809
    /// gescheitert war.
    fn visit_macro(&mut self, i: &'ast syn::Macro) {
        fn sammle(strom: proc_macro2::TokenStream, aus: &mut BTreeSet<String>) {
            for baum in strom {
                match baum {
                    proc_macro2::TokenTree::Ident(id) => {
                        aus.insert(id.to_string());
                    }
                    proc_macro2::TokenTree::Group(g) => sammle(g.stream(), aus),
                    _ => {}
                }
            }
        }
        sammle(i.tokens.clone(), &mut self.namen);
        syn::visit::visit_macro(self, i);
    }

    fn visit_use_tree(&mut self, i: &'ast syn::UseTree) {
        match i {
            syn::UseTree::Path(p) => self.namen.insert(p.ident.to_string()),
            syn::UseTree::Name(n) => self.namen.insert(n.ident.to_string()),
            syn::UseTree::Rename(r) => self.namen.insert(r.ident.to_string()),
            syn::UseTree::Glob(_) | syn::UseTree::Group(_) => false,
        };
        syn::visit::visit_use_tree(self, i);
    }
}

// ------------------------------------------------------------------- Messung

fn wurzel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Alle Rust-Dateien unter `sim/`, **ohne `target/`**.
///
/// Aus dem Verzeichnis gelesen und nicht aufgezaehlt. Die erste Fassung dieser
/// Messung zaehlte `src`, `tests` und `build.rs` auf und liess `examples/`
/// weg — zehn Dateien, in denen drei Nutzungen von `scaling` standen. Eine
/// Aufzaehlung muss jemand pflegen; ein neues Verzeichnis kommt hier von
/// selbst mit.
fn alle_dateien() -> Vec<PathBuf> {
    let mut aus = Vec::new();
    sammle(&wurzel(), &mut aus);
    aus.sort();
    aus
}

fn sammle(ordner: &Path, aus: &mut Vec<PathBuf>) {
    let Ok(eintraege) = std::fs::read_dir(ordner) else {
        return;
    };
    for eintrag in eintraege.flatten() {
        let pfad = eintrag.path();
        if pfad.is_dir() {
            if pfad.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            sammle(&pfad, aus);
        } else if pfad.extension().is_some_and(|e| e == "rs") {
            aus.push(pfad);
        }
    }
}

fn kurz(pfad: &Path) -> String {
    pfad.strip_prefix(wurzel())
        .unwrap_or(pfad)
        .to_string_lossy()
        .replace('\\', "/")
}

struct Ergebnis {
    kandidaten: Vec<Stelle>,
    tot: Vec<Stelle>,
    nur_intern: Vec<Stelle>,
    gelesene: Vec<String>,
}

fn messen() -> Ergebnis {
    let dateien = alle_dateien();
    let mut kandidaten = Vec::new();
    let mut nutzungen: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut gelesene = Vec::new();

    for pfad in &dateien {
        let name = kurz(pfad);
        gelesene.push(name.clone());
        let Ok(text) = std::fs::read_to_string(pfad) else {
            continue;
        };
        let Ok(baum) = syn::parse_file(&text) else {
            continue;
        };

        if name.starts_with("src/") {
            let mut sammler = Oeffentliche {
                datei: name.clone(),
                gefunden: Vec::new(),
            };
            sammler.visit_file(&baum);
            kandidaten.extend(sammler.gefunden);
        }

        let mut benutzt = Nutzungen::default();
        benutzt.visit_file(&baum);
        nutzungen.insert(name, benutzt.namen);
    }

    let leer = BTreeSet::new();
    let mut tot = Vec::new();
    let mut nur_intern = Vec::new();
    for stelle in &kandidaten {
        let woanders = nutzungen
            .iter()
            .filter(|(datei, _)| *datei != &stelle.datei)
            .any(|(_, namen)| namen.contains(&stelle.name));
        if woanders {
            continue;
        }
        if nutzungen
            .get(&stelle.datei)
            .unwrap_or(&leer)
            .contains(&stelle.name)
        {
            nur_intern.push(stelle.clone());
        } else {
            tot.push(stelle.clone());
        }
    }

    Ergebnis {
        kandidaten,
        tot,
        nur_intern,
        gelesene,
    }
}

// ------------------------------------------------------------------ Pruefung

#[test]
fn es_gibt_ueberhaupt_etwas_zu_pruefen() {
    // **Die Zusicherung vor der Messung.** Faende die Suche keine Symbole oder
    // keine Nutzungen, waere alles darunter gruen und pruefte nichts.
    let e = messen();
    assert!(
        e.kandidaten.len() > 200,
        "nur {} oeffentliche Symbole gefunden — die Suche greift nicht",
        e.kandidaten.len()
    );
    assert!(
        e.gelesene.len() > 100,
        "nur {} Dateien gelesen",
        e.gelesene.len()
    );
}

#[test]
fn fall_1_examples_wird_gelesen() {
    // **Mein erster Messfehler, als Prueffall.** Die Messung zaehlte `src`,
    // `tests` und `build.rs` auf. `sim/examples/` fehlte, und darin standen
    // drei Nutzungen von `scaling` — es galt als tot und war es nicht.
    let e = messen();
    let beispiele: Vec<_> = e
        .gelesene
        .iter()
        .filter(|d| d.starts_with("examples/"))
        .collect();
    assert!(
        beispiele.len() >= 5,
        "nur {} Dateien aus examples/ gelesen — genau die Luecke, die 21 statt 20 ergab",
        beispiele.len()
    );
}

#[test]
fn fall_2_eine_gleichnamige_bindung_ist_keine_nutzung() {
    // **Mein zweiter Messfehler, und der einzige, den ein Syntaxbaum nicht von
    // selbst loest.** `examples/resistance-threshold.rs` bindet eine lokale
    // Variable `label`; die Zaehlung hielt das fuer eine Nutzung von
    // `Origin::label` und meldete 19 statt 20.
    //
    // Geprueft wird an einem gebauten Fall statt am Bestand: Der Bestand
    // aendert sich, die Frage nicht.
    // **Der Rumpf ist leer, und das ist der Punkt.** Mein erster Entwurf
    // schrieb `let _ = (label, x);` hinein — das IST eine Nutzung, und der
    // Fall wurde zu Recht rot. Ein Prüffall, der das Gegenteil dessen
    // enthält, was er prüft, misst nichts.
    let quelle = r#"
        fn f() {
            for (label, _x) in [("a", 1)] {}
        }
    "#;
    let baum = syn::parse_file(quelle).expect("Beispiel muss parsen");
    let mut n = Nutzungen::default();
    n.visit_file(&baum);
    assert!(
        !n.namen.contains("label"),
        "eine Schleifen-Bindung zaehlt als Nutzung — genau der Fehler aus der Messung"
    );

    // Und die Gegenrichtung: ein echter Aufruf MUSS zaehlen, sonst waere der
    // Fall oben nur deshalb gruen, weil gar nichts gezaehlt wird.
    let echt = syn::parse_file("fn f(o: Origin) { let _ = o.label(); }").expect("parst");
    let mut m = Nutzungen::default();
    m.visit_file(&echt);
    assert!(
        m.namen.contains("label"),
        "ein Methodenaufruf zaehlt NICHT als Nutzung — dann meldet der Waechter alles"
    );
}

#[test]
fn fall_3_dateien_mit_ziffern_im_namen_werden_gelesen() {
    // **Mein dritter Messfehler.** Beim Auswerten stand ein Dateimuster
    // `[a-z/._-]+` ohne Ziffern; `parity_instance2_boss4.rs` fiel heraus, und
    // die spezifischere Zaehlung lieferte MEHR Treffer als die allgemeinere —
    // 126 gegen 113, was unmoeglich ist.
    let e = messen();
    let mit_ziffern: Vec<_> = e
        .gelesene
        .iter()
        .filter(|d| d.chars().any(|c| c.is_ascii_digit()))
        .collect();
    assert!(
        !mit_ziffern.is_empty(),
        "keine Datei mit Ziffern im Namen gelesen — das Muster verschluckt sie wieder"
    );
}

#[test]
fn traeger_ein_lebendes_symbol_wird_nicht_gemeldet() {
    // Ohne diesen Fall bliebe offen, ob der Waechter ueberhaupt zwischen
    // lebend und tot unterscheidet. Eine Zaehlung, die alles fuer benutzt
    // haelt, waere ebenso gruen wie eine richtige.
    let e = messen();
    let gemeldet: BTreeSet<_> = e
        .tot
        .iter()
        .chain(&e.nur_intern)
        .map(|s| s.name.clone())
        .collect();
    for lebend in ["Fight", "STREAM_COUNT", "spread_factor", "run"] {
        assert!(
            !gemeldet.contains(lebend),
            "{lebend} lebt und wird trotzdem gemeldet"
        );
    }
}

#[test]
fn kein_oeffentliches_symbol_ohne_nutzung_und_ohne_ausnahme() {
    let e = messen();
    let liste = ausnahmen();
    let neu: Vec<String> = e
        .tot
        .iter()
        .filter(|s| !liste.contains_key(s.schluessel().as_str()))
        .map(|s| format!("{} ({})", s.schluessel(), s.art))
        .collect();

    // **Die Diagnose gehört in die Meldung, nicht in einen zweiten Lauf.**
    // Jeder Versuch kostet hier eine Warteschlange am Build-Server; eine
    // Meldung, die nur Namen nennt, zwingt zu einem weiteren.
    let testdateien = e
        .gelesene
        .iter()
        .filter(|d| d.starts_with("tests/"))
        .count();
    let beispiele = e
        .gelesene
        .iter()
        .filter(|d| d.starts_with("examples/"))
        .count();

    assert!(
        neu.is_empty(),
        "Diese oeffentlichen Symbole benutzt NIEMAND: nicht ausserhalb ihrer \
         Datei, nicht darin. `dead_code` schweigt dazu, weil sie `pub` sind.\n\n\
         Entweder anschliessen oder loeschen. Ist es ein bewusster offener Rest, \
         gehoert das Symbol mit seinem Vorgang in `ausnahmen()`.\n\n\
         {neu:#?}\n\n\
         Gelesen: {} Dateien ({testdateien} in tests/, {beispiele} in examples/), \
         {} Kandidaten, {} nur intern benutzt.",
        e.gelesene.len(),
        e.kandidaten.len(),
        e.nur_intern.len()
    );
}

#[test]
fn die_ausnahmeliste_ueberlebt_nicht_was_sie_beschreibt() {
    // **Die Gegenrichtung.** Wird ein Symbol angeschlossen oder entfernt,
    // verschwindet es aus der Messung — die Zeile behauptete sonst weiter, es
    // gebe einen offenen Rest. Bei #811 hat genau dieser Fall die
    // Ausnahmeliste des TypeScript-Waechters geschlossen.
    let e = messen();
    let jetzt_tot: BTreeSet<String> = e.tot.iter().map(|s| s.schluessel()).collect();
    let ueberholt: Vec<&str> = ausnahmen()
        .keys()
        .filter(|k| !jetzt_tot.contains(**k))
        .copied()
        .collect();

    assert!(
        ueberholt.is_empty(),
        "Diese Eintraege in `ausnahmen()` sind erledigt — die Symbole sind \
         angeschlossen oder entfernt. Die Zeilen gehoeren geloescht, sonst \
         beschreibt die Liste einen Zustand, den es nicht mehr gibt:\n{ueberholt:#?}"
    );
}
