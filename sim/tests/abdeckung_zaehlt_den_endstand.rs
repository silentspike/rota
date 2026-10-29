//! Die beiden Abdeckungs-Läufer zählen, was sie berichten (Issue #550).
//!
//! # Zwei Mängel, beide beim Nachmessen am 2026-10-29 gefunden
//!
//! **`rotation-search` zählte die Listen VOR dem Nachlauf.** Die Zeile
//! „ueber alle N Bosse gewirkt" kam aus `irgendwo_gewirkt`, gefüllt in den
//! Boss-Abschnitten — und der Nachlauf aus #558 (`d3911c7a`, zwei Stunden
//! nach der Zählung aus `c5dd226c` eingebaut) verbesserte die Listen erst
//! danach. Der Kreuzvergleich darüber rechnete schon mit den neuen Listen, die
//! Zählung darunter nicht mehr. Bei `--max-len 6` folgenlos (Fixpunkt in
//! Runde 1 ohne Änderung), bei `--max-len 12` an der Basis `2f0c0d77` alle
//! vier Listen der ersten Instanz verändert, etwa `boss-1-1` 9446 -> 11552.
//!
//! **`zauber-abdeckung --mit-ausruestung` brach vor dem AC4-Teil ab.** Das
//! `exit(1)` der Untergrenze stand vor beiden Schalter-Teilen. Solange die
//! Untergrenze hielt, fiel das nicht auf; an der Basis `2f0c0d77` ist sie rot,
//! und die AC4-Messung war nicht mehr zu fahren: Ausgabe bytegleich mit dem
//! Lauf ohne Schalter, Exit 1.
//!
//! # Warum ein Syntaxbaum
//!
//! Beide Mängel sind eine REIHENFOLGE in `main`. Ein Zeilenmuster sähe einen
//! umbrochenen Aufruf nicht und hielte einen Kommentar für Code; `syn` sieht
//! Anweisungen.
//!
//! # Was hier NICHT geprüft wird
//!
//! Ob die Zahlen stimmen. Das zeigt der Lauf selbst; hier steht nur, dass er
//! den Endstand zählt und den AC4-Teil erreicht.

use std::path::PathBuf;
use syn::visit::Visit;

// Nur `wurzel()` wird gebraucht.
#[allow(dead_code)]
mod gemeinsam;

fn beispiel(name: &str) -> PathBuf {
    gemeinsam::wurzel().join("sim/examples").join(name)
}

/// Der Rumpf von `fn main` als Folge von Anweisungen.
fn main_rumpf(name: &str) -> Vec<syn::Stmt> {
    let pfad = beispiel(name);
    let text = std::fs::read_to_string(&pfad)
        .unwrap_or_else(|e| panic!("{} nicht lesbar: {e}", pfad.display()));
    let datei = syn::parse_file(&text)
        .unwrap_or_else(|e| panic!("{} ist kein gueltiges Rust: {e}", pfad.display()));
    datei
        .items
        .into_iter()
        .find_map(|item| match item {
            syn::Item::Fn(f) if f.sig.ident == "main" => Some(f.block.stmts),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{} hat kein fn main", pfad.display()))
}

/// Was eine Anweisung enthält: aufgerufene Funktionen (letztes Pfadsegment),
/// genannte Bezeichner und Zeichenketten-Literale.
#[derive(Default)]
struct Inhalt {
    aufrufe: Vec<String>,
    namen: Vec<String>,
    texte: Vec<String>,
}

impl<'ast> Visit<'ast> for Inhalt {
    fn visit_expr_call(&mut self, i: &'ast syn::ExprCall) {
        if let syn::Expr::Path(p) = &*i.func {
            if let Some(letztes) = p.path.segments.last() {
                self.aufrufe.push(letztes.ident.to_string());
            }
        }
        syn::visit::visit_expr_call(self, i);
    }

    fn visit_expr_path(&mut self, i: &'ast syn::ExprPath) {
        if let Some(ident) = i.path.get_ident() {
            self.namen.push(ident.to_string());
        }
        syn::visit::visit_expr_path(self, i);
    }

    fn visit_lit_str(&mut self, i: &'ast syn::LitStr) {
        self.texte.push(i.value());
        syn::visit::visit_lit_str(self, i);
    }
}

fn inhalt(stmt: &syn::Stmt) -> Inhalt {
    let mut aus = Inhalt::default();
    aus.visit_stmt(stmt);
    aus
}

fn ruft(stmt: &syn::Stmt, funktion: &str) -> bool {
    inhalt(stmt).aufrufe.iter().any(|a| a == funktion)
}

/// Der Name, an den eine `let`-Anweisung bindet, samt ihrem Anfangswert.
fn bindung(stmt: &syn::Stmt) -> Option<(String, &syn::Expr)> {
    let syn::Stmt::Local(local) = stmt else {
        return None;
    };
    let pat = match &local.pat {
        syn::Pat::Type(t) => &*t.pat,
        p => p,
    };
    let syn::Pat::Ident(ident) = pat else {
        return None;
    };
    let init = local.init.as_ref()?;
    Some((ident.ident.to_string(), &*init.expr))
}

/// Der erste Aufruf von `funktion` in einer Anweisung.
struct ErsterAufruf<'a, 'ast> {
    funktion: &'a str,
    gefunden: Option<&'ast syn::ExprCall>,
}

impl<'ast> Visit<'ast> for ErsterAufruf<'_, 'ast> {
    fn visit_expr_call(&mut self, i: &'ast syn::ExprCall) {
        if self.gefunden.is_none() {
            if let syn::Expr::Path(p) = &*i.func {
                if p.path
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == self.funktion)
                {
                    self.gefunden = Some(i);
                    return;
                }
            }
        }
        syn::visit::visit_expr_call(self, i);
    }
}

/// `rotation-search`: Die Abdeckung, die die Schlusszeile bekommt, entsteht
/// NACH dem Nachlauf und aus den Listen, die er hinterlässt (`gefundene`).
#[test]
fn rotation_search_zaehlt_die_listen_nach_dem_nachlauf() {
    let rumpf = main_rumpf("rotation-search.rs");

    // Träger: Ohne diese drei Stellen prüfte der Fall nichts.
    let nachlauf = rumpf
        .iter()
        .position(|s| ruft(s, "nachlauf"))
        .expect("main ruft nachlauf(...) nicht mehr auf — dann gibt es hier nichts zu pruefen");
    let schluss = rumpf
        .iter()
        .position(|s| ruft(s, "nirgends_die_beste_wahl"))
        .expect("main ruft nirgends_die_beste_wahl(...) nicht mehr auf");
    let mut suche = ErsterAufruf {
        funktion: "nirgends_die_beste_wahl",
        gefunden: None,
    };
    suche.visit_stmt(&rumpf[schluss]);
    let aufruf = suche
        .gefunden
        .expect("Aufruf gefunden, aber nicht greifbar");
    let erstes = aufruf
        .args
        .first()
        .expect("nirgends_die_beste_wahl ohne Argument");
    let erstes = match erstes {
        syn::Expr::Reference(r) => &*r.expr,
        e => e,
    };
    let syn::Expr::Path(p) = erstes else {
        panic!("das erste Argument von nirgends_die_beste_wahl ist kein Name — der Fall muss nachgezogen werden");
    };
    let name = p
        .path
        .get_ident()
        .expect("das erste Argument ist kein einfacher Name")
        .to_string();

    let (gebunden, init) = rumpf
        .iter()
        .enumerate()
        .find_map(|(i, s)| {
            bindung(s)
                .filter(|(n, _)| *n == name)
                .map(|(_, init)| (i, init))
        })
        .unwrap_or_else(|| panic!("`{name}` wird in main nicht mit let gebunden"));

    assert!(
        gebunden > nachlauf,
        "Die Schlusszeile zaehlt `{name}`, gebunden in Anweisung {gebunden} von main — \
         der Nachlauf laeuft erst in Anweisung {nachlauf}. Gezaehlt werden damit die \
         Listen VOR dem Nachlauf, berichtet wird ueber die danach."
    );

    let mut im_init = Inhalt::default();
    im_init.visit_expr(init);
    assert!(
        im_init.namen.iter().any(|n| n == "gefundene"),
        "`{name}` wird nach dem Nachlauf gebunden, aber nicht aus `gefundene` — \
         dann zaehlt es nicht die Listen, die der Nachlauf hinterlaesst"
    );
}

/// `zauber-abdeckung`: Kein `exit` vor den Schalter-Teilen. Eine rote
/// Untergrenze endet weiter mit 1 — aber erst, nachdem `--mit-ausruestung`,
/// `--toepfe-einzeln`, `--schulen` und `--pruefboss` (#550 AC3, seit
/// 2026-10-29) gelaufen sind.
#[test]
fn zauber_abdeckung_bricht_erst_nach_den_schalter_teilen_ab() {
    let rumpf = main_rumpf("zauber-abdeckung.rs");

    let schalter_stelle = |schalter: &str| {
        rumpf
            .iter()
            .position(|s| inhalt(s).texte.iter().any(|t| t == schalter))
            .unwrap_or_else(|| {
                panic!("main fragt {schalter} nicht mehr ab — der Fall muss nachgezogen werden")
            })
    };
    let letzter_schalter = [
        "--mit-ausruestung",
        "--toepfe-einzeln",
        "--schulen",
        "--pruefboss",
    ]
    .into_iter()
    .map(schalter_stelle)
    .max()
    .expect("vier Schalter");

    let abbrueche: Vec<usize> = rumpf
        .iter()
        .enumerate()
        .filter(|(_, s)| ruft(s, "exit"))
        .map(|(i, _)| i)
        .collect();
    assert!(
        !abbrueche.is_empty(),
        "main ruft kein exit mehr auf — dann endet eine rote Untergrenze mit 0, und der Nightly bleibt gruen"
    );

    let zu_frueh: Vec<usize> = abbrueche
        .iter()
        .copied()
        .filter(|&i| i < letzter_schalter)
        .collect();
    assert!(
        zu_frueh.is_empty(),
        "exit in Anweisung {zu_frueh:?} von main, vor dem letzten Schalter-Teil (Anweisung \
         {letzter_schalter}). Ist die Untergrenze rot, endet der Lauf dort, und die \
         Schalter-Teile laufen nie."
    );
}
