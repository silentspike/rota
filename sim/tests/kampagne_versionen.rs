//! Eine neue Zielzeit erzeugt ein NEUES Profil (Issue #914, AC7).
//!
//! # Was diese Datei schützt
//!
//! Ein versioniertes Profil hat feste Werteinheiten und wird **nicht**
//! nachträglich umgerechnet. Die Felder von [`CampaignDefinition`] sind
//! öffentlich — jeder kann eine Definition mit der Version 1 und einem anderen
//! Budget bauen. Der Schutz kann deshalb nicht im Typ liegen, sondern nur in
//! einem Katalog, der zu jeder vergebenen Version den Fingerabdruck festhält
//! und eine abweichende Fassung zurückweist.
//!
//! Und die Zielzeit gehört zur Definition, nicht zum Lauf: Ein Vertrag
//! übernimmt sie aus seiner Kampagne. Wer zwei Stunden statt vier will, braucht
//! ein abgeleitetes Profil mit eigener Version — eine andere Zahl im Vertrag
//! erkennt [`ProfileCatalog::origin`] als Abweichung.

use std::time::Duration;

use rota_sim::kampagne::{
    AuditPolicy, CampaignDefinition, CampaignRunContract, ConfidenceSpec, CoverageSpec,
    DeadlinePolicy, Mission, ProfileCatalog, ProfileError, ProjectionSet, QualityTargets,
    ReferenceProfile, ResultGuarantee, WorkUnits, REFERENCE_PROFILE_STANDARD,
};

const ZWEI_STUNDEN: Duration = Duration::from_secs(2 * 3600);
const VIER_STUNDEN: Duration = Duration::from_secs(4 * 3600);

fn kampagne() -> CampaignDefinition {
    CampaignDefinition {
        profile_version: 1,
        reference_profile: REFERENCE_PROFILE_STANDARD,
        target_wall_time: ZWEI_STUNDEN,
        canonical_work_budget: WorkUnits(1_000_000),
        exact_obligations: ProjectionSet { required: 12 },
        coverage_spec: CoverageSpec { strength: 2 },
        quality_targets: QualityTargets {
            minimum_niche_coverage: 0.95,
            independent_restarts: 8,
            plateau_work_units: WorkUnits(50_000),
            maximum_bound_gap: None,
            confidence_requirement: ConfidenceSpec { level: 0.99 },
            final_audit: AuditPolicy::ScalarReplayAll,
        },
    }
}

fn katalog() -> ProfileCatalog {
    let mut k = ProfileCatalog::new();
    k.freeze(kampagne()).expect("erste Fassung");
    k
}

/// **AC7** — die neue Zielzeit bekommt eine neue Version, die alte bleibt.
#[test]
fn eine_neue_zielzeit_erzeugt_ein_neues_profil() {
    let mut k = katalog();
    let vorher = kampagne().fingerprint();

    let neu = k
        .derive_for_target_time(1, VIER_STUNDEN, WorkUnits(2_000_000))
        .expect("abgeleitet");

    assert_eq!(neu.profile_version, 2, "die nächste freie Version");
    assert_eq!(neu.target_wall_time, VIER_STUNDEN);
    assert_eq!(neu.canonical_work_budget, WorkUnits(2_000_000));
    assert_ne!(neu.fingerprint(), vorher);

    // Das alte Profil ist unverändert — Inhalt UND Fingerabdruck.
    let alt = k.get(1).expect("Version 1 bleibt");
    assert_eq!(alt.target_wall_time, ZWEI_STUNDEN);
    assert_eq!(alt.canonical_work_budget, WorkUnits(1_000_000));
    assert_eq!(alt.fingerprint(), vorher);
    assert_eq!(k.get(2).map(CampaignDefinition::fingerprint), Some(neu.fingerprint()));
}

/// **AC7** — ein versioniertes Profil wird nicht umgerechnet.
///
/// Jede der beiden Änderungen, die ein „Umrechnen" ausmachen, wird unter der
/// alten Version abgelehnt: eine andere Zielzeit und ein anderes Budget.
#[test]
fn ein_versioniertes_profil_wird_nicht_umgerechnet() {
    let mut k = katalog();
    let eingefroren = kampagne().fingerprint();

    let mut andere_zeit = kampagne();
    andere_zeit.target_wall_time = VIER_STUNDEN;
    let mut anderes_budget = kampagne();
    anderes_budget.canonical_work_budget = WorkUnits(2_000_000);

    for umgerechnet in [andere_zeit, anderes_budget] {
        assert_eq!(
            k.freeze(umgerechnet),
            Err(ProfileError::VersionFrozen {
                profile_version: 1,
                frozen: eingefroren,
                presented: umgerechnet.fingerprint(),
            })
        );
        assert_eq!(
            k.verify(&umgerechnet),
            Err(ProfileError::VersionFrozen {
                profile_version: 1,
                frozen: eingefroren,
                presented: umgerechnet.fingerprint(),
            })
        );
    }
    // Der Katalog hat die abgelehnten Fassungen nicht übernommen.
    assert_eq!(k.get(1).map(CampaignDefinition::fingerprint), Some(eingefroren));

    // GEGENPROBE: Dieselbe Fassung noch einmal ist kein Fehler. Ohne sie wäre
    // der Fall oben auch dann grün, wenn `freeze` jede zweite Fassung ablehnte.
    assert_eq!(k.freeze(kampagne()), Ok(eingefroren));
    assert_eq!(k.verify(&kampagne()), Ok(()));
}

#[test]
fn unbekannte_versionen_und_herkunft_werden_benannt() {
    let mut k = katalog();
    let mut fremd = kampagne();
    fremd.profile_version = 7;
    assert_eq!(
        k.verify(&fremd),
        Err(ProfileError::UnknownVersion { profile_version: 7 })
    );
    assert_eq!(
        k.derive_for_target_time(7, VIER_STUNDEN, WorkUnits(1)),
        Err(ProfileError::UnknownVersion { profile_version: 7 })
    );

    let vertrag = CampaignRunContract::from_campaign(&fremd, Mission::FullBalance);
    assert_eq!(
        k.origin(&vertrag),
        Err(ProfileError::UnknownOrigin {
            campaign_definition_hash: fremd.fingerprint(),
        })
    );
}

#[test]
fn die_letzte_version_laesst_sich_nicht_ueberschreiten() {
    let mut k = ProfileCatalog::new();
    let mut letzte = kampagne();
    letzte.profile_version = u32::MAX;
    k.freeze(letzte).expect("höchste Version");
    assert_eq!(
        k.derive_for_target_time(u32::MAX, VIER_STUNDEN, WorkUnits(1)),
        Err(ProfileError::VersionsExhausted)
    );
}

/// Ein Vertrag nennt seine Herkunft — und jede übernommene Zahl muss dazu
/// passen.
///
/// Das ist der Leser von `campaign_definition_hash`: Ohne ihn stünde der
/// Fingerabdruck im Vertrag, und niemand prüfte, ob die Arbeitsmenge daneben
/// noch die der Kampagne ist.
#[test]
fn die_herkunft_eines_vertrags_ist_pruefbar() {
    let k = katalog();
    let vertrag = CampaignRunContract::from_campaign(&kampagne(), Mission::FullBalance);
    assert_eq!(
        k.origin(&vertrag).map(CampaignDefinition::fingerprint),
        Ok(kampagne().fingerprint())
    );

    // Eine Frist ist Laufsache und keine Abweichung von der Kampagne.
    let mit_frist = vertrag.with_hard_deadline(DeadlinePolicy::ActiveRuntime {
        remaining: Duration::from_secs(60),
    });
    assert!(k.origin(&mit_frist).is_ok());

    let mut abweichungen: Vec<(&str, CampaignRunContract)> = Vec::new();
    let mut v = vertrag;
    v.target_wall_time = VIER_STUNDEN;
    abweichungen.push(("target_wall_time", v));
    let mut v = vertrag;
    v.deterministic_work_budget = WorkUnits(1_000_001);
    abweichungen.push(("deterministic_work_budget", v));
    let mut v = vertrag;
    v.exact_obligations.required = 13;
    abweichungen.push(("exact_obligations", v));
    let mut v = vertrag;
    v.interaction_coverage.strength = 3;
    abweichungen.push(("interaction_coverage", v));
    let mut v = vertrag;
    v.minimum_niche_coverage = 0.96;
    abweichungen.push(("minimum_niche_coverage", v));
    let mut v = vertrag;
    v.independent_restarts = 9;
    abweichungen.push(("independent_restarts", v));
    let mut v = vertrag;
    v.plateau_work_units = WorkUnits(50_001);
    abweichungen.push(("plateau_work_units", v));
    let mut v = vertrag;
    v.maximum_bound_gap = Some(0.0);
    abweichungen.push(("maximum_bound_gap", v));
    let mut v = vertrag;
    v.confidence_requirement.level = 0.999;
    abweichungen.push(("confidence_requirement", v));
    let mut v = vertrag;
    v.final_audit = AuditPolicy::ScalarReplayDecisive;
    abweichungen.push(("final_audit", v));

    for (feld, v) in &abweichungen {
        assert_eq!(
            k.origin(v),
            Err(ProfileError::ContractDiverges { field: *feld }),
            "Das Feld {feld} weicht ab, ohne dass die Herkunftsprüfung es meldet."
        );
    }
    assert_eq!(
        abweichungen.len(),
        10,
        "Es wurden {} Felder geprüft. Jedes Feld, das der Vertrag aus der \
         Kampagne übernimmt, gehört hier dazu.",
        abweichungen.len()
    );
}

/// **AC5** — ein Teilergebnis gibt es nur, wenn ein Zeitabbruch verlangt war,
/// und seine geplante Arbeit ist die des Vertrags.
#[test]
fn ein_teilergebnis_setzt_eine_verlangte_frist_voraus() {
    let ohne = CampaignRunContract::from_campaign(&kampagne(), Mission::FullBalance);
    assert_eq!(
        ohne.anytime_partial(WorkUnits(400_000)),
        None,
        "Ohne verlangte Frist darf kein Lauf als abgeschnitten gelten."
    );

    let mit = ohne.with_hard_deadline(DeadlinePolicy::ActiveRuntime {
        remaining: ZWEI_STUNDEN,
    });
    assert_eq!(
        mit.anytime_partial(WorkUnits(400_000)),
        Some(ResultGuarantee::AnytimePartial {
            work_completed: WorkUnits(400_000),
            work_planned: WorkUnits(1_000_000),
        })
    );
}

/// Das Referenzprofil ist Teil der Identität: Dieselbe Version mit einem
/// anderen Referenzprofil ist eine andere Fassung.
#[test]
fn ein_anderes_referenzprofil_ist_eine_andere_fassung() {
    let k = katalog();
    let mut anders = kampagne();
    anders.reference_profile = ReferenceProfile {
        version: REFERENCE_PROFILE_STANDARD.version + 1,
        ..REFERENCE_PROFILE_STANDARD
    };
    assert!(matches!(
        k.verify(&anders),
        Err(ProfileError::VersionFrozen { .. })
    ));
}
