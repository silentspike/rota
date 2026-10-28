//! Welche CPU-Nummern ein Lauf benutzt — aus der Topologie, nicht gerechnet
//! (Issue #914, AC4).
//!
//! # Warum die Anzahl nicht genügt
//!
//! [`CpuReservation`] beantwortet „wie viele Stränge": zwölf. Welche zwölf, sagt
//! sie nicht, und die naheliegende Antwort „CPUs 0–11" stimmt nur auf Maschinen,
//! deren Kernel die SMT-Geschwister nebeneinander nummeriert. Viele nummerieren
//! erst alle ersten Stränge und dann alle zweiten (`0,8`, `1,9`, …) — dort
//! hiesse „CPUs 0–11" sechs ganze Kerne plus die zweiten Stränge von vier
//! weiteren, also genau die Lesart, die AC4 ausschliesst.
//!
//! Deshalb prüft diese Datei beide Nummerierungen nebeneinander. Der Fall
//! [`verschraenkte_nummerierung_reserviert_dieselben_kerne_unter_anderen_nummern`]
//! ist die Gegenprobe zum Laptop-Fall: Wer aus der Anzahl rechnet statt aus der
//! Topologie, besteht den einen und fällt beim anderen durch.
//!
//! Die echte Maschine prüft das Beispiel `e21-0-cpus` mit `taskset` — eine
//! Prüfstrecke, die auf dem Buildserver läuft, sähe dessen Topologie und nicht
//! die des Laptops.

use std::path::{Path, PathBuf};

use rota_sim::hostcaps::kernliste;
use rota_sim::kampagne::{cpu_list, CpuReservation, CpuTopology, LogicalCpu, TopologyError};

/// Die Topologie des Laptops, wie sysfs sie am 2026-10-28 meldete: acht Kerne,
/// Geschwister nebeneinander (`thread_siblings_list` 0-1, 2-3, …, 14-15).
fn laptop() -> Vec<LogicalCpu> {
    (0..16)
        .map(|cpu| LogicalCpu {
            cpu,
            package: 0,
            core: cpu / 2,
        })
        .collect()
}

/// Dieselben acht Kerne, verschränkt nummeriert: Kern k hat die CPUs k und k+8.
fn verschraenkt() -> Vec<LogicalCpu> {
    (0..16)
        .map(|cpu| LogicalCpu {
            cpu,
            package: 0,
            core: cpu % 8,
        })
        .collect()
}

fn bereich(von: u32, bis: u32) -> Vec<u32> {
    (von..=bis).collect()
}

/// **AC4** — die Anzeige des Issues, wörtlich, aus der Laptop-Topologie.
#[test]
fn laptop_compute_cpus_0_bis_11_und_reserviert_kerne_6_und_7() {
    let topologie = CpuTopology::from_cpus(&laptop()).expect("gültige Topologie");
    let belegung = topologie.assign(2).expect("zwei von acht Kernen");

    assert_eq!(belegung.compute_cpus(), bereich(0, 11).as_slice());
    assert_eq!(belegung.reserved_cpus(), bereich(12, 15));
    let kerne: Vec<u32> = belegung.reserved_cores().iter().map(|k| k.core).collect();
    assert_eq!(
        kerne,
        vec![6, 7],
        "reserviert werden die beiden letzten Kerne"
    );

    let text = belegung.describe();
    assert!(text.contains("Compute:     CPUs 0–11"), "{text}");
    assert!(
        text.contains("Reserviert:  physische Kerne 6–7 samt SMT-Geschwistern (CPUs 12–15)"),
        "{text}"
    );
}

/// **AC4, Gegenprobe** — dieselbe Reservierung, andere Nummerierung.
///
/// Zwölf Stränge bleiben zwölf Stränge, aber es sind andere Nummern. Eine
/// Belegung, die „die ersten zwölf CPUs" nähme, reservierte hier die Kerne 4–7
/// nur halb und nutzte ihre zweiten Stränge mit.
#[test]
fn verschraenkte_nummerierung_reserviert_dieselben_kerne_unter_anderen_nummern() {
    let topologie = CpuTopology::from_cpus(&verschraenkt()).expect("gültige Topologie");
    let belegung = topologie.assign(2).expect("zwei von acht Kernen");

    let mut erwartet = bereich(0, 5);
    erwartet.extend(bereich(8, 13));
    assert_eq!(belegung.compute_cpus(), erwartet.as_slice());
    assert_eq!(belegung.reserved_cpus(), vec![6, 7, 14, 15]);
    assert_eq!(belegung.compute_cpus().len(), 12, "zwölf, nicht vierzehn");

    let text = belegung.describe();
    assert!(text.contains("Compute:     CPUs 0–5, 8–13"), "{text}");
    assert!(
        text.contains("physische Kerne 6–7 samt SMT-Geschwistern (CPUs 6–7, 14–15)"),
        "{text}"
    );
    // Die kernelübliche Liste für `taskset -c` ist dieselbe Menge.
    assert_eq!(cpu_list(belegung.compute_cpus()), "0-5,8-13");
}

/// Die Anzahl aus der Topologie ist die Anzahl aus [`CpuReservation`] — zwei
/// Wege zu derselben Zahl, damit keiner von beiden still abweicht.
#[test]
fn die_topologie_liefert_dieselbe_anzahl_wie_die_rechenvorgabe() {
    for cpus in [laptop(), verschraenkt()] {
        let topologie = CpuTopology::from_cpus(&cpus).expect("gültige Topologie");
        let vorgabe = topologie.reservation(2).expect("homogen");
        assert_eq!(
            vorgabe,
            CpuReservation {
                physical_cores: 8,
                threads_per_core: 2,
                reserved_physical_cores: 2,
            }
        );
        let belegung = topologie.assign(2).expect("zwei von acht");
        assert_eq!(
            belegung.compute_cpus().len() as u32,
            vorgabe.compute_threads()
        );
    }
}

/// Ohne SMT nimmt ein reservierter Kern genau eine CPU mit.
#[test]
fn ohne_smt_nimmt_ein_kern_nur_sich_selbst_mit() {
    let cpus: Vec<LogicalCpu> = (0..8)
        .map(|cpu| LogicalCpu {
            cpu,
            package: 0,
            core: cpu,
        })
        .collect();
    let belegung = CpuTopology::from_cpus(&cpus)
        .expect("gültige Topologie")
        .assign(2)
        .expect("zwei von acht");
    assert_eq!(belegung.compute_cpus(), bereich(0, 5).as_slice());
    assert_eq!(belegung.reserved_cpus(), vec![6, 7]);
}

/// P- und E-Kerne: Die Nummern sind eindeutig, die Anzahl je Kern nicht.
///
/// [`CpuReservation`] setzt gleich viele Stränge je Kern voraus und kann diese
/// Maschine nicht beschreiben — das muss ein Fehler sein, keine Mittelung.
#[test]
fn ungleiche_kerne_haben_nummern_aber_keine_einheitliche_anzahl() {
    // Zwei P-Kerne mit je zwei Strängen, vier E-Kerne mit je einem.
    let mut cpus = vec![
        LogicalCpu {
            cpu: 0,
            package: 0,
            core: 0,
        },
        LogicalCpu {
            cpu: 1,
            package: 0,
            core: 0,
        },
        LogicalCpu {
            cpu: 2,
            package: 0,
            core: 1,
        },
        LogicalCpu {
            cpu: 3,
            package: 0,
            core: 1,
        },
    ];
    cpus.extend((4..8).map(|cpu| LogicalCpu {
        cpu,
        package: 0,
        core: cpu - 2,
    }));
    let topologie = CpuTopology::from_cpus(&cpus).expect("gültige Topologie");
    assert_eq!(topologie.reservation(1), Err(TopologyError::Heterogeneous));
    let belegung = topologie.assign(1).expect("Nummern bleiben eindeutig");
    assert_eq!(belegung.compute_cpus(), bereich(0, 6).as_slice());
    assert_eq!(belegung.reserved_cpus(), vec![7]);
}

/// Zwei Pakete: `core_id` wiederholt sich, die Anzeige muss das Paket nennen.
#[test]
fn zwei_pakete_werden_nicht_zu_einem_zusammengeworfen() {
    let cpus: Vec<LogicalCpu> = (0..8)
        .map(|cpu| LogicalCpu {
            cpu,
            package: cpu / 4,
            core: cpu % 4,
        })
        .collect();
    let topologie = CpuTopology::from_cpus(&cpus).expect("gültige Topologie");
    assert_eq!(
        topologie.cores().len(),
        8,
        "core_id 0 in Paket 0 und in Paket 1 sind zwei Kerne"
    );
    let text = topologie.assign(2).expect("zwei von acht").describe();
    assert!(text.contains("physische Kerne 1/2, 1/3"), "{text}");
}

#[test]
fn zu_grosse_leere_und_doppelte_angaben_werden_abgelehnt() {
    let topologie = CpuTopology::from_cpus(&laptop()).expect("gültige Topologie");
    for reserviert in [8, 9, u16::MAX] {
        assert_eq!(
            topologie.assign(reserviert),
            Err(TopologyError::ReservationTooLarge {
                reserved: reserviert,
                physical_cores: 8,
            }),
            "Alle Kerne zu reservieren lässt keine CPU zum Rechnen"
        );
    }
    // GEGENPROBE: eine Reservierung von null ist gültig und lässt alles frei.
    assert_eq!(
        topologie
            .assign(0)
            .expect("nichts reserviert")
            .compute_cpus(),
        bereich(0, 15).as_slice()
    );

    assert_eq!(CpuTopology::from_cpus(&[]), Err(TopologyError::Empty));
    let mut doppelt = laptop();
    doppelt.push(doppelt[3]);
    assert_eq!(
        CpuTopology::from_cpus(&doppelt),
        Err(TopologyError::DuplicateCpu { cpu: 3 })
    );
}

/// Die Liste für `taskset -c` ist die kernelübliche Form und wird von
/// [`kernliste`] — dem Leser von `Cpus_allowed_list` — genau zurückgelesen.
#[test]
fn cpu_liste_ist_die_kernelform_und_wird_zurueckgelesen() {
    assert_eq!(cpu_list(&bereich(0, 11)), "0-11");
    assert_eq!(cpu_list(&[0, 1, 2, 4, 5, 8]), "0-2,4-5,8");
    assert_eq!(cpu_list(&[7]), "7");
    assert_eq!(cpu_list(&[]), "");
    for liste in [bereich(0, 11), vec![0, 1, 2, 4, 5, 8], vec![3, 9, 10]] {
        let zurueck: Vec<u32> = kernliste(&cpu_list(&liste))
            .expect("lesbar")
            .into_iter()
            .map(|k| u32::try_from(k).expect("klein"))
            .collect();
        assert_eq!(zurueck, liste);
    }
}

/// Der Bericht trägt die Nummern, nicht nur ihre Anzahl.
#[test]
fn der_bericht_nennt_die_cpu_nummern() {
    let belegung = CpuTopology::from_cpus(&verschraenkt())
        .expect("gültige Topologie")
        .assign(2)
        .expect("zwei von acht");
    let v: serde_json::Value = serde_json::from_str(&belegung.to_json()).expect("gültiges JSON");
    let mut erwartet = bereich(0, 5);
    erwartet.extend(bereich(8, 13));
    assert_eq!(v["computeCpus"], serde_json::json!(erwartet));
    assert_eq!(v["computeCpuList"], "0-5,8-13");
    assert_eq!(v["reservedCpuList"], "6-7,14-15");
    assert_eq!(v["reservedCores"][0]["core"], 6);
    assert_eq!(v["reservedCores"][0]["cpus"], serde_json::json!([6, 14]));
    assert_eq!(v["reservedCores"][1]["core"], 7);
}

// --------------------------------------------------------------- sysfs lesen

/// Ein Nachbau von `/sys/devices/system/cpu` mit den drei Dateien je CPU, die
/// [`CpuTopology::from_sysfs_at`] liest.
fn sysfs_baum(name: &str, eintraege: &[(u32, u32, u32, &str)], online: &str) -> PathBuf {
    let wurzel = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&wurzel);
    std::fs::create_dir_all(&wurzel).expect("Verzeichnis");
    std::fs::write(wurzel.join("online"), format!("{online}\n")).expect("online");
    for &(cpu, paket, kern, geschwister) in eintraege {
        let topo = wurzel.join(format!("cpu{cpu}/topology"));
        std::fs::create_dir_all(&topo).expect("topology");
        std::fs::write(topo.join("physical_package_id"), format!("{paket}\n")).expect("paket");
        std::fs::write(topo.join("core_id"), format!("{kern}\n")).expect("kern");
        std::fs::write(
            topo.join("thread_siblings_list"),
            format!("{geschwister}\n"),
        )
        .expect("geschwister");
    }
    wurzel
}

#[test]
fn sysfs_wird_gelesen_wie_der_kernel_es_schreibt() {
    let eintraege: Vec<(u32, u32, u32, String)> = (0..16)
        .map(|cpu| {
            let erster = cpu - cpu % 2;
            (cpu, 0, cpu / 2, format!("{}-{}", erster, erster + 1))
        })
        .collect();
    let refs: Vec<(u32, u32, u32, &str)> = eintraege
        .iter()
        .map(|(c, p, k, g)| (*c, *p, *k, g.as_str()))
        .collect();
    let wurzel = sysfs_baum("sysfs-laptop", &refs, "0-15");
    let gelesen = CpuTopology::from_sysfs_at(&wurzel).expect("lesbar");
    assert_eq!(
        gelesen,
        CpuTopology::from_cpus(&laptop()).expect("gültig"),
        "Die gelesene Topologie ist nicht die geschriebene"
    );
}

/// Nur `online` zählt: Eine abgeschaltete CPU hat ein Verzeichnis, aber keinen
/// Platz in der Belegung.
#[test]
fn abgeschaltete_cpus_gehoeren_nicht_dazu() {
    let wurzel = sysfs_baum(
        "sysfs-offline",
        &[
            (0, 0, 0, "0-1"),
            (1, 0, 0, "0-1"),
            (2, 0, 1, "2"),
            (3, 0, 1, "3"),
        ],
        "0-2",
    );
    let topologie = CpuTopology::from_sysfs_at(&wurzel).expect("lesbar");
    let alle: Vec<u32> = topologie
        .cores()
        .iter()
        .flat_map(|k| k.cpus.clone())
        .collect();
    assert_eq!(alle, vec![0, 1, 2]);
}

/// `thread_siblings_list` ist die Aussage des Kernels, welche CPUs sich einen
/// Kern teilen. Widerspricht sie `core_id`, ist eine der beiden Angaben falsch
/// — und dann ist keine Belegung daraus eindeutig.
#[test]
fn widerspruechliche_geschwisterlisten_werden_abgelehnt() {
    let wurzel = sysfs_baum(
        "sysfs-widerspruch",
        &[
            (0, 0, 0, "0-1"),
            (1, 0, 0, "0-1"),
            (2, 0, 1, "2-3"),
            // Kern 1 laut core_id, aber laut Geschwisterliste allein.
            (3, 0, 1, "3"),
        ],
        "0-3",
    );
    assert_eq!(
        CpuTopology::from_sysfs_at(&wurzel),
        Err(TopologyError::SiblingMismatch { cpu: 3 })
    );
}

#[test]
fn fehlende_und_kaputte_dateien_werden_benannt() {
    let wurzel = sysfs_baum("sysfs-fehlt", &[(0, 0, 0, "0")], "0-1");
    match CpuTopology::from_sysfs_at(&wurzel) {
        Err(TopologyError::Unreadable { path }) => {
            assert!(
                path.ends_with("cpu1/topology/physical_package_id"),
                "{path}"
            );
        }
        anders => panic!("erwartet Unreadable, bekam {anders:?}"),
    }

    let wurzel = sysfs_baum("sysfs-kaputt", &[(0, 0, 0, "0")], "0");
    std::fs::write(wurzel.join("cpu0/topology/core_id"), "kein Kern\n").expect("schreiben");
    match CpuTopology::from_sysfs_at(&wurzel) {
        Err(TopologyError::Malformed { path }) => {
            assert!(path.ends_with("cpu0/topology/core_id"), "{path}");
        }
        anders => panic!("erwartet Malformed, bekam {anders:?}"),
    }
}
