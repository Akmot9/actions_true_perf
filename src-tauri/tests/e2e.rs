//! Test de bout en bout sur les fichiers réels présents à la racine du repo
//! (non versionnés : données personnelles). Ignoré silencieusement s'ils sont
//! absents, pour que `cargo test` reste vert sur une copie fraîche.

use actions_true_perf_lib::commands::{build_portfolio, do_import};
use actions_true_perf_lib::db;
use rust_decimal::Decimal;
use std::path::Path;
use std::str::FromStr;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn read_samples() -> Option<(String, String)> {
    let root = repo_root();
    let yahoo = std::fs::read_to_string(root.join("portfolio.csv")).ok()?;
    let bd = std::fs::read_dir(&root)
        .ok()?
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().starts_with("Releve_compte"))
        .and_then(|e| std::fs::read_to_string(e.path()).ok())?;
    Some((yahoo, bd))
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

#[test]
fn full_import_reconciles_transfers_and_keeps_per_order_lots() {
    let Some((yahoo, bd)) = read_samples() else {
        eprintln!("fichiers d'exemple absents, test sauté");
        return;
    };
    let conn = db::open(Path::new(":memory:")).unwrap();

    // Ordre volontairement « défavorable » : Bourse Direct d'abord, comme
    // dans la vraie vie ; l'historique BoursoBank arrive ensuite.
    let r1 = do_import(&conn, "releve_bd.csv", &bd).unwrap();
    assert!(r1.inserted > 0);

    // Avant l'import de l'historique : 141 Nexity toutes non rapprochées.
    let p = build_portfolio(&conn).unwrap();
    let nexity = p.positions.iter().find(|p| p.symbol.as_deref() == Some("NXI.PA")).unwrap();
    assert_eq!(nexity.unreconciled_quantity, dec("141"));

    let r2 = do_import(&conn, "portfolio.csv", &yahoo).unwrap();
    assert!(r2.inserted > 0);

    // Réimport à l'identique : zéro doublon.
    let r3 = do_import(&conn, "releve_bd.csv", &bd).unwrap();
    assert_eq!(r3.inserted, 0);
    assert_eq!(r3.duplicates, r1.inserted + r1.duplicates);
    let r4 = do_import(&conn, "portfolio.csv", &yahoo).unwrap();
    assert_eq!(r4.inserted, 0);

    let p = build_portfolio(&conn).unwrap();

    // Nexity : 22 issues de l'historique BoursoBank + 119 au PRU courtier,
    // plus les achats Bourse Direct (17+19+13+18+15+28 = 110) => 251.
    let nexity = p.positions.iter().find(|p| p.symbol.as_deref() == Some("NXI.PA")).unwrap();
    assert_eq!(nexity.quantity, dec("251"));
    assert_eq!(nexity.unreconciled_quantity, dec("119"));
    // 3 lots BoursoBank + 1 lot non rapproché + 6 achats Bourse Direct.
    assert_eq!(nexity.lots.len(), 10);

    // Dassault Systèmes : les 7 ordres d'origine restent visibles
    // individuellement, dates et prix intacts, malgré le transfert.
    let dsy = p.positions.iter().find(|p| p.symbol.as_deref() == Some("DSY.PA")).unwrap();
    assert_eq!(dsy.quantity, dec("33"));
    assert_eq!(dsy.unreconciled_quantity, dec("8"));
    assert_eq!(dsy.lots.len(), 8); // 7 ordres historiques + 1 non rapproché
    let first = dsy.lots.iter().find(|l| l.unit_cost == dec("50.75")).unwrap();
    assert_eq!(first.acquisition_date.unwrap().to_string(), "2022-01-05");
    assert_eq!(first.origin_broker, "BoursoBank");
    assert_eq!(first.account, "Bourse Direct"); // compte actuel après transfert

    // Atos : position clôturée par l'indemnisation OST, absente du portefeuille.
    assert!(p.positions.iter().all(|p| p.symbol.as_deref() != Some("ATO.PA")));
    assert!(p.total_realized_pnl < Decimal::ZERO); // moins-value Atos réalisée

    // Les cours embarqués dans portfolio.csv alimentent le cache de cotations.
    assert!(dsy.quote.is_some());
    assert_eq!(dsy.quote.as_ref().unwrap().price, dec("22.06"));
    // Performance par ordre calculée : l'ordre à 50,75 € est en forte perte.
    assert!(first.pnl_pct.unwrap() < dec("-50"));
}
