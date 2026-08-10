pub mod bourse_direct;
pub mod yahoo_portfolio;

use rust_decimal::Decimal;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::domain::ImportedTransaction;

/// Résultat brut d'un import de fichier, avant persistance.
#[derive(Debug, Serialize)]
pub struct ParsedFile {
    pub broker: String,
    pub transactions: Vec<ImportedTransaction>,
    pub warnings: Vec<String>,
    /// Cours actuels trouvés dans le fichier (portfolio.csv en contient),
    /// utilisables comme cache de cotations hors-ligne.
    pub quotes: Vec<(String, Decimal)>,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("format de fichier non reconnu (en-têtes inconnus)")]
    UnknownFormat,
    #[error("erreur CSV: {0}")]
    Csv(#[from] csv::Error),
}

/// Détecte le format via les en-têtes et parse le contenu.
pub fn parse_any(content: &str) -> Result<ParsedFile, ImportError> {
    let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(content.as_bytes());
    let headers: Vec<String> = reader.headers()?.iter().map(|h| h.trim().to_string()).collect();

    if bourse_direct::detect(&headers) {
        bourse_direct::parse(content)
    } else if yahoo_portfolio::detect(&headers) {
        yahoo_portfolio::parse(content)
    } else {
        Err(ImportError::UnknownFormat)
    }
}

/// Empreinte stable d'une opération pour la déduplication entre imports.
/// `occurrence` distingue deux opérations réellement identiques le même jour
/// (2 ordres identiques = 2 transactions ; réimporter le fichier = 0 doublon).
pub struct FingerprintBuilder {
    seen: HashMap<String, u32>,
}

impl FingerprintBuilder {
    pub fn new() -> Self {
        Self { seen: HashMap::new() }
    }

    pub fn fingerprint(&mut self, parts: &[&str]) -> String {
        let key = parts.join("|");
        let occurrence = self.seen.entry(key.clone()).or_insert(0);
        *occurrence += 1;
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hasher.update(b"|#");
        hasher.update(occurrence.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

impl Default for FingerprintBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn parse_decimal(s: &str) -> Option<Decimal> {
    let cleaned = s.trim().replace('\u{a0}', "").replace(' ', "").replace(',', ".");
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_rows_get_distinct_fingerprints_but_stable_across_files() {
        let mut f1 = FingerprintBuilder::new();
        let a1 = f1.fingerprint(&["BD", "2025-01-01", "BUY", "NXI.PA", "10", "9.6"]);
        let a2 = f1.fingerprint(&["BD", "2025-01-01", "BUY", "NXI.PA", "10", "9.6"]);
        assert_ne!(a1, a2, "deux ordres identiques le même jour restent distincts");

        let mut f2 = FingerprintBuilder::new();
        let b1 = f2.fingerprint(&["BD", "2025-01-01", "BUY", "NXI.PA", "10", "9.6"]);
        let b2 = f2.fingerprint(&["BD", "2025-01-01", "BUY", "NXI.PA", "10", "9.6"]);
        assert_eq!(a1, b1, "réimport du même fichier => mêmes empreintes");
        assert_eq!(a2, b2);
    }

    #[test]
    fn parses_french_decimals() {
        assert_eq!(parse_decimal("12,25773"), Some("12.25773".parse().unwrap()));
        assert_eq!(parse_decimal("1 234,5"), Some("1234.5".parse().unwrap()));
        assert_eq!(parse_decimal(""), None);
    }
}
