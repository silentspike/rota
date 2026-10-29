//! Die Prioritätenliste einlesen — mit Fehlermeldungen, die weiterhelfen.
//!
//! Aus Issue #55: *„Lesbare Fehlermeldung bei fehlerhafter Liste, mit
//! Zeilenangabe"* — und ausdrücklich: *„statt still etwas anderes zu tun"*.
//!
//! Das ist keine Bequemlichkeit. Eine Liste, die sich vertippt hat und trotzdem
//! läuft, erzeugt ein Balance-Ergebnis, das niemand anzweifelt: Die Zahlen sehen
//! plausibel aus, nur die Rotation war eine andere als gemeint. Deshalb wird hier
//! jede Unklarheit zum Fehler — ein unbekannter Zauber, ein unbekannter Wert, ein
//! fehlender Vergleich. Nichts wird geraten.

use std::fmt;

use super::{spell_index, Apl, Compare, Condition, Number, Rule, Term, Value, OVERDRIVE_ACTION};

/// Was an einer Prioritätenliste nicht stimmt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AplError {
    /// Zeilennummer, ab 1 gezählt.
    pub line: usize,
    /// Spalte, ab 1 gezählt.
    pub column: usize,
    /// Was nicht stimmt, in einem Satz.
    pub message: String,
    /// Die betroffene Zeile im Wortlaut.
    pub source_line: String,
}

impl fmt::Display for AplError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Zeile {}, Spalte {}: {}",
            self.line, self.column, self.message
        )?;
        writeln!(f, "  {}", self.source_line)?;
        // Der Zeiger steht unter der Fundstelle. Bei langen Bedingungen ist das
        // der Unterschied zwischen „irgendwo in dieser Zeile" und „hier".
        write!(f, "  {}^", " ".repeat(self.column.saturating_sub(1)))
    }
}

impl std::error::Error for AplError {}

/// Ein Wort oder Zeichen aus einer Zeile, mit seiner Spalte.
#[derive(Debug, Clone)]
struct Token {
    text: String,
    column: usize,
}

/// Der **Name** einer Rotation — kein beliebiger Text (Issue #499).
///
/// # Warum drei Zeilen sich lohnen
///
/// `parse(AplName(name), text)` nahm zweimal `&str`. Ein vertauschter Aufruf
/// uebersetzte anstandslos, nahm den Dateiinhalt als Namen und parste das
/// Wort "gut" als Rotation — die Fehlermeldung sprach dann von einem
/// **unbekannten Zauber** und zeigte damit auf die falsche Stelle. Das ist
/// teurer als ein Absturz: Sie schickt den Leser in die Daten, obwohl der
/// Fehler im Aufruf steht.
///
/// # Der Beleg, nicht das Versprechen (AC5)
///
/// Der folgende Aufruf uebersetzt **nicht**. `compile_fail` heisst: Der
/// Doctest schlaegt fehl, wenn er es DOCH tut — die Zusicherung kann also
/// nicht still verfallen.
///
/// ```compile_fail
/// use rota_sim::apl::parse;
/// // Name und Text vertauscht — frueher lief das durch und meldete einen
/// // "unbekannten Zauber".
/// let _ = parse("kindle\n", "meine-rotation");
/// ```
///
/// So herum ist es richtig:
///
/// ```
/// use rota_sim::apl::{parse, AplName};
/// let apl = parse(AplName("meine-rotation"), "kindle\n").expect("gueltig");
/// assert_eq!(apl.name, "meine-rotation");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AplName<'a>(pub &'a str);

impl<'a> AplName<'a> {
    /// Der Name als Text.
    #[must_use]
    pub fn as_str(self) -> &'a str {
        self.0
    }
}

impl fmt::Display for AplName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Liest eine Prioritätenliste ein.
///
/// # Errors
///
/// Gibt den **ersten** Fehler zurück, mit Zeile, Spalte und Grund. Weitere
/// Fehler werden nicht gesammelt: Nach einem Tippfehler ist der Rest der Datei
/// oft Folgefehler, und eine lange Liste davon verdeckt die eigentliche Ursache.
pub fn parse(name: AplName<'_>, text: &str) -> Result<Apl, AplError> {
    let mut rules = Vec::new();
    let mut overdrive: Option<Condition> = None;

    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let content = strip_comment(raw);
        if content.trim().is_empty() {
            continue;
        }
        // **Overdrive ist keine Regel** (#1177, Abschnitt 2): Er hat keinen
        // Platz in der Rangfolge, siehe [`Apl::overdrive`]. Die Zeile sieht
        // aus wie eine, und sie wird mit derselben Bedingungs-Grammatik
        // gelesen — nur landet sie in ihrem eigenen Feld.
        if content.split_whitespace().next() == Some(OVERDRIVE_ACTION) {
            // **Zweimal ist ein Fehler, nicht „die letzte gewinnt".** Zwei
            // Zeilen mit verschiedenen Bedingungen hießen zwei Absichten, und
            // still eine davon zu nehmen wäre genau das „still etwas anderes
            // tun", das dieser Parser ausschließt.
            if overdrive.is_some() {
                return Err(AplError {
                    line,
                    column: 1,
                    message: format!(
                        "\"{OVERDRIVE_ACTION}\" steht zweimal in der Liste. Er geht einmal je \
                         Kampf; eine Bedingung genügt."
                    ),
                    source_line: raw.to_owned(),
                });
            }
            overdrive = Some(parse_overdrive(line, raw, content)?);
            continue;
        }
        rules.push(parse_rule(line, raw, content)?);
    }

    if rules.is_empty() {
        return Err(AplError {
            line: text.lines().count().max(1),
            column: 1,
            message: "Die Liste enthält keine einzige Regel.".to_owned(),
            source_line: String::new(),
        });
    }

    Ok(Apl {
        rules,
        name: name.as_str().to_owned(),
        overdrive,
    })
}

/// Liest eine `overdrive`-Zeile: das Wort allein oder mit `if <bedingung>`.
fn parse_overdrive(line: usize, raw: &str, content: &str) -> Result<Condition, AplError> {
    let tokens = tokenize(content);
    let fail = |column: usize, message: String| AplError {
        line,
        column,
        message,
        source_line: raw.to_owned(),
    };
    if tokens.len() == 1 {
        return Ok(Condition::default());
    }
    let keyword = &tokens[1];
    if keyword.text != "if" {
        return Err(fail(
            keyword.column,
            format!(
                "Nach \"{OVERDRIVE_ACTION}\" wird \"if\" erwartet oder das Zeilenende, nicht \"{}\".",
                keyword.text
            ),
        ));
    }
    let rest = &tokens[2..];
    if rest.is_empty() {
        return Err(fail(
            keyword.column + 2,
            "Nach \"if\" fehlt die Bedingung.".to_owned(),
        ));
    }
    parse_condition(rest, &fail)
}

/// Schneidet einen Kommentar ab. `#` gilt überall, auch am Zeilenende.
fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(at) => &line[..at],
        None => line,
    }
}

fn parse_rule(line: usize, raw: &str, content: &str) -> Result<Rule, AplError> {
    let tokens = tokenize(content);
    let fail = |column: usize, message: String| AplError {
        line,
        column,
        message,
        source_line: raw.to_owned(),
    };

    let Some(first) = tokens.first() else {
        return Err(fail(1, "Die Zeile enthält keine Regel.".to_owned()));
    };

    let spell = spell_index(&first.text).ok_or_else(|| {
        fail(
            first.column,
            format!(
                "Unbekannter Zauber \"{}\". Bekannt sind: {}.",
                first.text,
                // **Nur spielbare Zauber vorschlagen** (Issue #499). Vorher
                // standen die Pruef-Zauber mit in der Liste: Wer sich
                // vertippte, bekam `test-buff` angeboten und konnte damit
                // eine Balance-Messung verfaelschen, ohne dass etwas rot
                // wird. Die Liste bleibt — sie wird nur kuerzer.
                crate::apl::spielbare_zauber()
                    .map(|s| s.id)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
    })?;

    if tokens.len() == 1 {
        return Ok(Rule {
            spell,
            condition: Condition::default(),
            line,
        });
    }

    let keyword = &tokens[1];
    if keyword.text != "if" {
        return Err(fail(
            keyword.column,
            format!(
                "Nach dem Zauber wird \"if\" erwartet oder das Zeilenende, nicht \"{}\".",
                keyword.text
            ),
        ));
    }

    let rest = &tokens[2..];
    if rest.is_empty() {
        return Err(fail(
            keyword.column + 2,
            "Nach \"if\" fehlt die Bedingung.".to_owned(),
        ));
    }

    let condition = parse_condition(rest, &fail)?;
    Ok(Rule {
        spell,
        condition,
        line,
    })
}

fn parse_condition(
    tokens: &[Token],
    fail: &impl Fn(usize, String) -> AplError,
) -> Result<Condition, AplError> {
    let mut groups = Vec::new();
    let mut group = Vec::new();
    let mut index = 0;

    loop {
        let (term, next) = parse_term(tokens, index, fail)?;
        group.push(term);
        index = next;

        let Some(joiner) = tokens.get(index) else {
            groups.push(group);
            break;
        };
        match joiner.text.as_str() {
            "and" => index += 1,
            "or" => {
                groups.push(std::mem::take(&mut group));
                index += 1;
            }
            other => {
                return Err(fail(
                    joiner.column,
                    format!(
                        "Erwartet wurde \"and\", \"or\" oder das Zeilenende, nicht \"{other}\"."
                    ),
                ))
            }
        }
        if index >= tokens.len() {
            return Err(fail(
                joiner.column,
                format!("Nach \"{}\" fehlt der nächste Vergleich.", joiner.text),
            ));
        }
    }

    Ok(Condition { groups })
}

/// Liest einen Vergleich. Ohne Vergleichszeichen gilt der Wert als Wahrheitswert.
fn parse_term(
    tokens: &[Token],
    index: usize,
    fail: &impl Fn(usize, String) -> AplError,
) -> Result<(Term, usize), AplError> {
    let Some(left_token) = tokens.get(index) else {
        let column = tokens.last().map_or(1, |t| t.column);
        return Err(fail(column, "Hier fehlt ein Vergleich.".to_owned()));
    };
    let left = parse_value(left_token, fail)?;

    let Some(operator) = tokens.get(index + 1) else {
        // Alleinstehend, etwa `cooldown.pyre.ready` — als „ist wahr" gelesen.
        return Ok((boolean_term(left, left_token, fail)?, index + 1));
    };

    let Some(compare) = parse_compare(&operator.text) else {
        // Kein Vergleichszeichen: Der Wert steht für sich, das nächste Wort
        // gehört schon zur Verknüpfung.
        return Ok((boolean_term(left, left_token, fail)?, index + 1));
    };

    let Some(right_token) = tokens.get(index + 2) else {
        return Err(fail(
            operator.column + operator.text.len(),
            format!(
                "Nach \"{}\" fehlt der Wert, mit dem verglichen wird.",
                operator.text
            ),
        ));
    };
    let right = parse_value(right_token, fail)?;

    Ok((
        Term {
            left,
            compare,
            right,
        },
        index + 3,
    ))
}

/// Macht aus einem alleinstehenden Wert den Vergleich „ist wahr".
fn boolean_term(
    value: Value,
    token: &Token,
    fail: &impl Fn(usize, String) -> AplError,
) -> Result<Term, AplError> {
    // Eine nackte Zahl als Bedingung ist fast immer ein Tippfehler — etwa ein
    // vergessenes Vergleichszeichen. Sie stillschweigend als „wahr" zu lesen
    // wäre genau das „still etwas anderes tun", das dieses Issue ausschließt.
    if matches!(value, Value::Literal(_)) {
        return Err(fail(
            token.column,
            format!(
                "\"{}\" ist eine Zahl und keine Bedingung. Fehlt ein Vergleich wie \">= {}\"?",
                token.text, token.text
            ),
        ));
    }
    Ok(Term {
        left: value,
        compare: Compare::NotEqual,
        right: Value::Literal(0),
    })
}

fn parse_compare(text: &str) -> Option<Compare> {
    match text {
        "<" => Some(Compare::Less),
        "<=" => Some(Compare::LessEqual),
        ">" => Some(Compare::Greater),
        ">=" => Some(Compare::GreaterEqual),
        "==" => Some(Compare::Equal),
        "!=" => Some(Compare::NotEqual),
        _ => None,
    }
}

fn parse_value(
    token: &Token,
    fail: &impl Fn(usize, String) -> AplError,
) -> Result<Value, AplError> {
    let text = token.text.as_str();

    if text.starts_with(|c: char| c.is_ascii_digit()) {
        return parse_number(token, fail).map(Value::Literal);
    }

    if super::resource_names().contains(&text) {
        return Ok(Value::Resource);
    }

    match text {
        "gcd" => return Ok(Value::Gcd),
        "time" => return Ok(Value::Time),
        // Die Kernmechaniken der vier uebrigen Schulen (Issue #526). Bis
        // hierher konnte nur CALCINATION bedingen — ueber `heat`.
        "decay" => return Ok(Value::Decay),
        "shield" => return Ok(Value::Shield),
        "doom" => return Ok(Value::Doom),
        "doom_charges" => return Ok(Value::DoomCharges),
        "afflicted" => return Ok(Value::Afflicted),
        "race_progress" => return Ok(Value::RaceProgress),
        "control" => return Ok(Value::Control),
        _ => {}
    }

    let parts: Vec<&str> = text.split('.').collect();
    match parts.as_slice() {
        ["boss", "hp"] => return Ok(Value::BossHp),
        ["boss", "hp_pct"] => return Ok(Value::BossHpPercent),
        ["boss", "casting"] => return Ok(Value::BossCasting),
        ["boss", "cast_remains"] => return Ok(Value::BossCastRemains),
        ["boss", "next_strike_in"] => return Ok(Value::BossNextStrikeIn),
        ["fight", "remains"] => return Ok(Value::FightRemains),
        ["cooldown", spell, field] => {
            let index = lookup_spell(spell, token, fail)?;
            return match *field {
                "ready" => Ok(Value::CooldownReady(index)),
                "remains" => Ok(Value::CooldownRemains(index)),
                other => Err(fail(
                    token.column,
                    format!("\"cooldown.{spell}.{other}\" gibt es nicht. Möglich: ready, remains."),
                )),
            };
        }
        ["buff", spell, field] => {
            let index = lookup_spell(spell, token, fail)?;
            return match *field {
                "active" => Ok(Value::BuffActive(index)),
                "remains" => Ok(Value::BuffRemains(index)),
                other => Err(fail(
                    token.column,
                    format!("\"buff.{spell}.{other}\" gibt es nicht. Möglich: active, remains."),
                )),
            };
        }
        ["dot", spell, field] => {
            let index = lookup_spell(spell, token, fail)?;
            return match *field {
                "active" => Ok(Value::DotActive(index)),
                "remains" => Ok(Value::DotRemains(index)),
                "ticks" => Ok(Value::DotTicks(index)),
                other => Err(fail(
                    token.column,
                    format!(
                        "\"dot.{spell}.{other}\" gibt es nicht. Möglich: active, remains, ticks."
                    ),
                )),
            };
        }
        _ => {}
    }

    Err(fail(
        token.column,
        format!(
            "Unbekannter Wert \"{text}\". Möglich sind: {}, resource, decay, shield, control, \
             gcd, time, boss.hp, boss.hp_pct, fight.remains, cooldown.<zauber>.ready, \
             cooldown.<zauber>.remains, dot.<zauber>.active, dot.<zauber>.remains, \
             dot.<zauber>.ticks, buff.<zauber>.active, buff.<zauber>.remains.",
            crate::data::RESOURCE.id
        ),
    ))
}

fn lookup_spell(
    name: &str,
    token: &Token,
    fail: &impl Fn(usize, String) -> AplError,
) -> Result<usize, AplError> {
    spell_index(name).ok_or_else(|| {
        fail(
            token.column,
            format!(
                "Unbekannter Zauber \"{name}\". Bekannt sind: {}.",
                // **Nur spielbare Zauber vorschlagen** (Issue #499). Vorher
                // standen die Pruef-Zauber mit in der Liste: Wer sich
                // vertippte, bekam `test-buff` angeboten und konnte damit
                // eine Balance-Messung verfaelschen, ohne dass etwas rot
                // wird. Die Liste bleibt — sie wird nur kuerzer.
                crate::apl::spielbare_zauber()
                    .map(|s| s.id)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
    })
}

/// Liest eine Zahl, wahlweise mit Zeit-Einheit.
///
/// `1500` und `1500ms` sind dasselbe; `1.5s` ergibt ebenfalls 1500. Gerechnet
/// wird durchweg ganzzahlig — eine Zeit, die als Fließkommazahl durch den Parser
/// liefe, wäre die erste Determinismus-Regel aus 18.4 gebrochen, bevor der
/// Kampf überhaupt beginnt.
fn parse_number(
    token: &Token,
    fail: &impl Fn(usize, String) -> AplError,
) -> Result<Number, AplError> {
    let text = token.text.as_str();
    let (digits, unit) = split_unit(text);

    let (whole, fraction) = match digits.split_once('.') {
        Some((w, f)) => (w, f),
        None => (digits, ""),
    };

    let bad = |why: &str| {
        fail(
            token.column,
            format!("\"{text}\" ist keine gültige Zahl: {why}"),
        )
    };

    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("vor dem Punkt stehen keine Ziffern."));
    }
    if !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("nach dem Punkt stehen keine Ziffern."));
    }

    let whole: Number = whole.parse().map_err(|_| bad("die Zahl ist zu groß."))?;

    match unit {
        "s" => {
            if fraction.len() > 3 {
                return Err(bad(
                    "Sekunden werden auf Millisekunden genau angegeben, also höchstens drei Stellen nach dem Punkt.",
                ));
            }
            // Auf drei Stellen auffüllen: "5" -> 500 ms, "05" -> 50 ms.
            let mut millis: Number = 0;
            let mut padded = fraction.to_owned();
            while padded.len() < 3 {
                padded.push('0');
            }
            if !padded.is_empty() {
                millis = padded
                    .parse()
                    .map_err(|_| bad("der Nachkommateil ist ungültig."))?;
            }
            Ok(whole * Number::from(1000_u16) + millis)
        }
        "ms" | "" => {
            if !fraction.is_empty() {
                return Err(bad(
                    "Millisekunden sind ganzzahlig. Für Bruchteile die Einheit \"s\" verwenden, etwa \"1.5s\".",
                ));
            }
            Ok(whole)
        }
        other => Err(bad(&format!(
            "\"{other}\" ist keine bekannte Einheit. Möglich: ms, s — oder gar keine."
        ))),
    }
}

fn split_unit(text: &str) -> (&str, &str) {
    if let Some(rest) = text.strip_suffix("ms") {
        return (rest, "ms");
    }
    if let Some(rest) = text.strip_suffix('s') {
        return (rest, "s");
    }
    (text, "")
}

/// Zerlegt eine Zeile in Wörter und Vergleichszeichen.
///
/// Vergleichszeichen dürfen am Wort kleben (`heat>=15`) oder frei stehen
/// (`heat >= 15`) — beides kommt vor, wenn Listen von Hand geschrieben werden.
fn tokenize(line: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        if matches!(c, '<' | '>' | '=' | '!') {
            let mut text = String::from(c);
            if chars.get(i + 1) == Some(&'=') {
                text.push('=');
            }
            tokens.push(Token {
                column: i + 1,
                text,
            });
            i += tokens.last().map_or(1, |t| t.text.len());
            continue;
        }

        let start = i;
        while i < chars.len()
            && !chars[i].is_whitespace()
            && !matches!(chars[i], '<' | '>' | '=' | '!')
        {
            i += 1;
        }
        tokens.push(Token {
            text: chars[start..i].iter().collect(),
            column: start + 1,
        });
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zerlegt_klebende_vergleichszeichen() {
        let tokens = tokenize("pyre if heat>=15");
        let texts: Vec<&str> = tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, vec!["pyre", "if", "heat", ">=", "15"]);
        assert_eq!(tokens[3].column, 13);
    }

    #[test]
    fn zahlen_mit_und_ohne_einheit() {
        let probe = |text: &str| {
            let token = Token {
                text: text.to_owned(),
                column: 1,
            };
            parse_number(&token, &|column, message| AplError {
                line: 1,
                column,
                message,
                source_line: String::new(),
            })
        };
        assert_eq!(probe("1500"), Ok(1500));
        assert_eq!(probe("1500ms"), Ok(1500));
        assert_eq!(probe("1.5s"), Ok(1500));
        assert_eq!(probe("12s"), Ok(12_000));
        assert_eq!(probe("0.05s"), Ok(50));
        assert!(probe("1.5").is_err(), "Millisekunden sind ganzzahlig");
        assert!(probe("1.2345s").is_err(), "höchstens drei Nachkommastellen");
        assert!(probe("5min").is_err(), "unbekannte Einheit");
    }

    #[test]
    fn kommentare_und_leerzeilen_werden_uebergangen() {
        let apl = parse(
            AplName("test"),
            "# Kopfzeile\n\nkindle   # Fueller\n\n# Ende\n",
        )
        .expect("gültig");
        assert_eq!(apl.rules.len(), 1);
        assert_eq!(apl.rules[0].line, 3);
    }
}
