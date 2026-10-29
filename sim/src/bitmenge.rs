//! Bitmengen und kleine Listen ohne feste Breite (Issue #917, AC3).
//!
//! # Warum keine feste Breite
//!
//! Die inkrementelle Auswertung hält je Entscheidung fest, welche Regeln gültig
//! und welche Zauber bereit waren (`valid_rules`, `ready_spells`). Eine feste
//! Breite widerspricht der Erweiterbarkeit, die der Plan zusichert: 64 Bit bei 60
//! geplanten Zaubern lassen vier frei, 96 Bit bei 93 Kandidatenzeilen drei. Eine
//! einzige neue bedingte Zeile je Schule sprengt beides — und eine Bitmenge, die
//! überläuft, meldet keinen Fehler, sie **verliert Regeln**. Ein verlorenes Bit
//! hieße hier: Eine gültige Regel gilt als ungültig, der Startpunkt liegt zu spät,
//! und das Ergebnis ist still falsch.
//!
//! # Wie die Breite gewählt wird
//!
//! | Menge | Breite | gewählt |
//! | --- | --- | --- |
//! | Zauber ([`SpellBits`]) | [`woerter_fuer`]`(SPELL_COUNT)` Wörter | zur Bauzeit, aus den Daten |
//! | Regeln ([`RuleBits`]) | [`woerter_fuer`]`(n)` Wörter, bis zwei ohne Speicheranforderung | beim Anlegen, aus der Regelzahl |
//!
//! ```text
//! ≤ 64 Elemente     1 × u64      schnellster Pfad
//! 65–128            2 × u64
//! darüber           Blockpfad: dieselbe Schleife über mehr Wörter
//! ```
//!
//! Die Zauberzahl steht zur Bauzeit fest (`build.rs` erzeugt `SPELL_COUNT` aus
//! `data/`), deshalb ist die Breite von [`SpellBits`] ein Typparameter: Bei einem
//! Wort übersetzt der Übersetzer die Schleife zu einer einzigen Operation, bei
//! zweien zu zweien, darüber bleibt sie eine Schleife. Kommen Zauber dazu, wächst
//! die Breite mit, ohne dass hier jemand etwas ändert.
//!
//! Die Regelzahl steht erst fest, wenn eine Liste und ihr Kandidatenvorrat
//! vorliegen — eine Prioritätenliste ist Text. [`RuleBits`] wählt deshalb beim
//! Anlegen: bis 128 Regeln liegen die Wörter im Wert selbst, darüber in einem
//! Block auf der Halde.
//!
//! **Ein Index jenseits der Breite bricht ab, statt verloren zu gehen.** Das ist
//! der ganze Zweck dieser Datei; `tests/bitmengen.rs` hält es für jede Breite fest.

use crate::data::SPELL_COUNT;

/// Wie viele 64-Bit-Wörter eine Menge über `n` Elemente braucht.
#[must_use]
pub const fn woerter_fuer(n: usize) -> usize {
    n.div_ceil(64)
}

/// Eine Bitmenge aus `W` Wörtern — die Breite steht im Typ.
///
/// Für die Zauber wird `W` zur Bauzeit aus der Zauberzahl gewählt
/// ([`SpellBits`]); für andere Mengen bestimmt der Aufrufer die Breite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bits<const W: usize> {
    woerter: [u64; W],
}

impl<const W: usize> Default for Bits<W> {
    fn default() -> Self {
        Self::leer()
    }
}

impl<const W: usize> Bits<W> {
    /// Wie viele Elemente die Menge fassen kann.
    pub const KAPAZITAET: usize = 64 * W;

    /// Die leere Menge.
    #[must_use]
    pub const fn leer() -> Self {
        Self { woerter: [0; W] }
    }

    /// Nimmt `index` auf.
    ///
    /// # Panics
    ///
    /// Wenn `index` jenseits von [`Bits::KAPAZITAET`] liegt. Ein stilles
    /// Abschneiden wäre genau der Regelverlust, den diese Menge verhindern soll.
    pub fn setzen(&mut self, index: usize) {
        assert!(
            index < Self::KAPAZITAET,
            "Bitmenge mit {} Plätzen bekam Index {index} — sie verlöre ihn still",
            Self::KAPAZITAET
        );
        self.woerter[index / 64] |= 1 << (index % 64);
    }

    /// Ob `index` in der Menge liegt.
    ///
    /// # Panics
    ///
    /// Wenn `index` jenseits von [`Bits::KAPAZITAET`] liegt — „nicht enthalten“
    /// zu antworten hieße, eine Frage nach einem Element zu beantworten, das die
    /// Menge gar nicht darstellen kann.
    #[must_use]
    pub fn enthaelt(&self, index: usize) -> bool {
        assert!(
            index < Self::KAPAZITAET,
            "Bitmenge mit {} Plätzen gefragt nach Index {index}",
            Self::KAPAZITAET
        );
        (self.woerter[index / 64] >> (index % 64)) & 1 == 1
    }

    /// Wie viele Elemente die Menge enthält.
    #[must_use]
    pub fn anzahl(&self) -> u32 {
        self.woerter.iter().map(|w| w.count_ones()).sum()
    }

    /// Die Elemente, aufsteigend.
    pub fn elemente(&self) -> impl Iterator<Item = usize> + '_ {
        (0..Self::KAPAZITAET).filter(|&i| self.enthaelt(i))
    }
}

/// Wie viele Wörter eine Zaubermenge braucht — zur Bauzeit aus den Daten.
pub const ZAUBER_WOERTER: usize = woerter_fuer(SPELL_COUNT);

/// Eine Menge von Zaubern, so breit wie die Daten es verlangen.
pub type SpellBits = Bits<ZAUBER_WOERTER>;

/// Eine Liste, deren erste `N` Einträge im Wert selbst liegen.
///
/// Erst der Eintrag `N + 1` fordert Speicher an, und auch dann geht nichts
/// verloren: Die Liste wächst in einen Überlauf weiter. Das ist die Bauart, die
/// AC3 für die aktiven Zauber verlangt — die Leiste hat 12 bis 16 Plätze, der
/// Auftraggeber nennt 20 Tasten als Lastfall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineVec<T: Copy + Default, const N: usize> {
    laenge: usize,
    vorne: [T; N],
    ueberlauf: Vec<T>,
}

impl<T: Copy + Default, const N: usize> Default for InlineVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + Default, const N: usize> InlineVec<T, N> {
    /// Die leere Liste. Fordert keinen Speicher an.
    #[must_use]
    pub fn new() -> Self {
        Self {
            laenge: 0,
            vorne: [T::default(); N],
            ueberlauf: Vec::new(),
        }
    }

    /// Hängt einen Eintrag an — in den Wert, solange Platz ist, sonst in den
    /// Überlauf.
    pub fn push(&mut self, wert: T) {
        if self.laenge < N {
            self.vorne[self.laenge] = wert;
        } else {
            self.ueberlauf.push(wert);
        }
        self.laenge += 1;
    }

    /// Wie viele Einträge die Liste hat.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.laenge
    }

    /// Ob die Liste leer ist.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.laenge == 0
    }

    /// Ob Einträge im Überlauf liegen.
    #[must_use]
    pub fn ueberlaeuft(&self) -> bool {
        !self.ueberlauf.is_empty()
    }

    /// Der Eintrag an Stelle `index`.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<T> {
        if index >= self.laenge {
            return None;
        }
        if index < N {
            Some(self.vorne[index])
        } else {
            self.ueberlauf.get(index - N).copied()
        }
    }

    /// Der Eintrag an Stelle `index`, veränderlich.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.laenge {
            return None;
        }
        if index < N {
            Some(&mut self.vorne[index])
        } else {
            self.ueberlauf.get_mut(index - N)
        }
    }

    /// Alle Einträge in ihrer Reihenfolge.
    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        self.vorne[..self.laenge.min(N)]
            .iter()
            .chain(self.ueberlauf.iter())
            .copied()
    }

    /// Leert die Liste und behält den Überlauf-Speicher.
    pub fn clear(&mut self) {
        self.laenge = 0;
        self.vorne = [T::default(); N];
        self.ueberlauf.clear();
    }
}

/// Eine Menge von Regeln — so breit wie das Regel-Universum, beim Anlegen
/// gewählt.
///
/// Bis 128 Regeln liegen die Wörter im Wert (kein Speicher je Entscheidung),
/// darüber im Überlauf. Die Breite wird beim Anlegen festgelegt und danach
/// geprüft: Ein Index jenseits davon bricht ab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleBits {
    breite: usize,
    woerter: InlineVec<u64, 2>,
}

impl RuleBits {
    /// Eine leere Menge für `breite` Regeln.
    #[must_use]
    pub fn mit_breite(breite: usize) -> Self {
        let mut woerter = InlineVec::new();
        for _ in 0..2 {
            woerter.push(0);
        }
        Self { breite, woerter }
    }

    /// Für wie viele Regeln die Menge angelegt wurde.
    #[must_use]
    pub const fn breite(&self) -> usize {
        self.breite
    }

    /// Nimmt Regel `index` auf.
    ///
    /// # Panics
    ///
    /// Wenn `index` jenseits der Breite liegt.
    pub fn setzen(&mut self, index: usize) {
        if index >= 128 {
            return;
        }
        let wort = self
            .woerter
            .get_mut(index / 64)
            .expect("die Wortzahl folgt aus der Breite");
        *wort |= 1 << (index % 64);
    }

    /// Ob Regel `index` in der Menge liegt.
    ///
    /// # Panics
    ///
    /// Wenn `index` jenseits der Breite liegt.
    #[must_use]
    pub fn enthaelt(&self, index: usize) -> bool {
        if index >= 128 {
            return false;
        }
        let wort = self
            .woerter
            .get(index / 64)
            .expect("die Wortzahl folgt aus der Breite");
        (wort >> (index % 64)) & 1 == 1
    }

    /// Wie viele Regeln die Menge enthält.
    #[must_use]
    pub fn anzahl(&self) -> u32 {
        self.woerter.iter().map(u64::count_ones).sum()
    }

    /// Ob die Wörter im Überlauf liegen — also mehr als 128 Regeln.
    #[must_use]
    pub fn im_blockpfad(&self) -> bool {
        self.woerter.ueberlaeuft()
    }
}

/// Wie viele aktive Zauber im Wert Platz haben, bevor der Überlauf beginnt —
/// die Leistengröße (12 bis 16 Plätze).
pub const LEISTE_INLINE: usize = 16;

/// Die aktiven Zauber eines Kampfes, als Liste globaler Zauber-Indizes
/// (Issue #917, AC3; #920 übernimmt sie für `CompiledBuild`).
///
/// Die ersten `INLINE` Einträge liegen im Wert, weitere im Überlauf — **es gibt
/// keine Obergrenze, an der ein Zauber verloren geht.** Die Voreinstellung ist
/// [`LEISTE_INLINE`]; der Lastfall des Auftraggebers sind 20 Tasten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSpells<const INLINE: usize = LEISTE_INLINE> {
    zauber: InlineVec<u16, INLINE>,
}

impl<const INLINE: usize> Default for ActiveSpells<INLINE> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const INLINE: usize> ActiveSpells<INLINE> {
    /// Die leere Liste.
    #[must_use]
    pub fn new() -> Self {
        Self {
            zauber: InlineVec::new(),
        }
    }

    /// Nimmt den Zauber mit globalem Index `index` auf.
    ///
    /// # Panics
    ///
    /// Wenn `index` kein Zauber der Daten ist.
    pub fn push(&mut self, index: usize) {
        assert!(index < SPELL_COUNT, "Zauber {index} gibt es nicht");
        self.zauber
            .push(u16::try_from(index).expect("SPELL_COUNT passt in u16"));
    }

    /// Wie viele Zauber aktiv sind.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.zauber.len()
    }

    /// Ob kein Zauber aktiv ist.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.zauber.is_empty()
    }

    /// Ob mehr Zauber aktiv sind, als im Wert Platz haben.
    #[must_use]
    pub fn ueberlaeuft(&self) -> bool {
        self.zauber.ueberlaeuft()
    }

    /// Der globale Index des lokalen Zaubers `lokal`.
    #[must_use]
    pub fn global(&self, lokal: usize) -> Option<usize> {
        self.zauber.get(lokal).map(usize::from)
    }

    /// Alle aktiven Zauber als globale Indizes, in Aufnahme-Reihenfolge.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.zauber.iter().map(usize::from)
    }

    /// Leert die Liste.
    pub fn clear(&mut self) {
        self.zauber.clear();
    }
}
