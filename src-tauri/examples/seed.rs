//! Utilitaire de développement : importe les fichiers CSV présents à la
//! racine du repo dans la base locale de l'application, via le même chemin
//! de code que l'UI (`do_import`). La déduplication rend l'opération
//! idempotente.
//!
//! Usage : `cargo run --example seed`

use actions_true_perf_lib::commands::do_import;
use actions_true_perf_lib::db;
use std::path::Path;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let data_dir = dirs_path();
    std::fs::create_dir_all(&data_dir).expect("création du dossier de données");
    let db_path = data_dir.join("portfolio.db");
    let conn = db::open(&db_path).expect("ouverture de la base");
    println!("Base : {}", db_path.display());

    let mut files: Vec<std::path::PathBuf> = vec![root.join("portfolio.csv")];
    if let Ok(entries) = std::fs::read_dir(&root) {
        files.extend(
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name().is_some_and(|n| {
                        let n = n.to_string_lossy();
                        n.starts_with("Releve_compte") || n.starts_with("transactions_")
                    })
                }),
        );
    }

    for path in files {
        let Ok(content) = std::fs::read_to_string(&path) else {
            println!("· {} absent, ignoré", path.display());
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        match do_import(&conn, &name, &content) {
            Ok(r) => println!(
                "· {name} : {} opérations, {} nouvelles, {} doublons, {} avertissements",
                r.total,
                r.inserted,
                r.duplicates,
                r.warnings.len()
            ),
            Err(e) => println!("· {name} : ERREUR {e}"),
        }
    }
}

/// Reproduit `app_data_dir` de Tauri pour l'identifiant de l'application
/// (Linux : ~/.local/share/<identifier>).
fn dirs_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").expect("HOME non défini");
    Path::new(&home).join(".local/share/com.akmot9.suiviordres")
}
