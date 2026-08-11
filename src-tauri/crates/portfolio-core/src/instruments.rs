use crate::domain::InstrumentRef;

/// Correspondances libellé courtier → (symbole Yahoo, nom canonique).
/// Les libellés Bourse Direct n'incluent pas d'ISIN ; ce tableau seed couvre
/// les instruments du portefeuille. Les libellés inconnus produisent un
/// instrument sans symbole, signalé en avertissement à l'import.
const LABEL_ALIASES: &[(&str, &str, &str)] = &[
    ("ATOS", "ATO.PA", "Atos"),
    ("AIR LIQUIDE", "AI.PA", "Air Liquide"),
    ("AIR LIQUIDE PF28", "AI.PA", "Air Liquide"),
    ("TOTALENERGIES SE", "TTE.PA", "TotalEnergies"),
    ("TOTALENERGIES", "TTE.PA", "TotalEnergies"),
    ("VINCI", "DG.PA", "Vinci"),
    ("NEXITY", "NXI.PA", "Nexity"),
    ("FDJ", "FDJU.PA", "FDJ United"),
    ("FDJ UNITED", "FDJU.PA", "FDJ United"),
    ("DASSAULT SYSTEMES", "DSY.PA", "Dassault Systèmes"),
    ("STMICROELECTRONICS", "STMPA.PA", "STMicroelectronics"),
    ("STELLANTIS", "STLAP.PA", "Stellantis"),
];

/// Symboles devenus obsolètes (renommage FDJ -> FDJ United, codes Paris de
/// Yahoo différents des codes historiques) → symbole coté actuel.
/// Appliqué à tout symbole entrant pour éviter des instruments en double.
const SYMBOL_ALIASES: &[(&str, &str)] = &[
    ("FDJ.PA", "FDJU.PA"),
    ("STLA.PA", "STLAP.PA"),
    ("STM.PA", "STMPA.PA"),
];

pub fn canonical_symbol(symbol: &str) -> String {
    let up = symbol.trim().to_uppercase();
    for (old, new) in SYMBOL_ALIASES {
        if up == *old {
            return (*new).to_string();
        }
    }
    up
}

/// ISIN → (symbole Yahoo, nom canonique). Pour les titres non cotés à Paris,
/// on privilégie une place de cotation en euros (Tesla via XETRA, ETF via
/// XETRA) afin de ne pas mélanger les devises dans les performances.
const ISIN_ALIASES: &[(&str, &str, &str)] = &[
    ("FR0000120073", "AI.PA", "Air Liquide"),
    ("FR0000130650", "DSY.PA", "Dassault Systèmes"),
    ("FR0000120271", "TTE.PA", "TotalEnergies"),
    ("FR0000125486", "DG.PA", "Vinci"),
    ("FR0010112524", "NXI.PA", "Nexity"),
    ("FR0013451333", "FDJU.PA", "FDJ United"),
    ("NL0000226223", "STMPA.PA", "STMicroelectronics"),
    ("NL00150001Q9", "STLAP.PA", "Stellantis"),
    ("FR0000051732", "ATO.PA", "Atos"),
    ("US88160R1014", "TL0.DE", "Tesla"),
    (
        "IE00B3WJKG14",
        "QDVE.DE",
        "S&P 500 Information Technology (Acc)",
    ),
];

/// Cryptomonnaies : symbole courtier → paire Yahoo cotée en euros.
const CRYPTO_ALIASES: &[(&str, &str, &str)] = &[
    ("BTC", "BTC-EUR", "Bitcoin"),
    ("ETH", "ETH-EUR", "Ethereum"),
    ("SOL", "SOL-EUR", "Solana"),
];

/// Résout un libellé courtier (ex: "DASSAULT SYSTEMES") vers un instrument.
pub fn resolve_label(label: &str) -> InstrumentRef {
    let norm = label.trim().to_uppercase();
    for (alias, symbol, name) in LABEL_ALIASES {
        if norm == *alias {
            return InstrumentRef {
                symbol: Some((*symbol).to_string()),
                isin: None,
                name: (*name).to_string(),
            };
        }
    }
    InstrumentRef {
        symbol: None,
        isin: None,
        name: label.trim().to_string(),
    }
}

/// Résout un ISIN vers un instrument. Un ISIN inconnu produit un instrument
/// sans symbole de cotation (pas de cours, mais lots et PRU fonctionnent).
pub fn resolve_isin(isin: &str, fallback_name: &str) -> InstrumentRef {
    let up = isin.trim().to_uppercase();
    for (known, symbol, name) in ISIN_ALIASES {
        if up == *known {
            return InstrumentRef {
                symbol: Some((*symbol).to_string()),
                isin: Some(up),
                name: (*name).to_string(),
            };
        }
    }
    InstrumentRef {
        symbol: None,
        isin: Some(up),
        name: fallback_name.trim().to_string(),
    }
}

/// Normalise un symbole saisi à la main : majuscules, symboles obsolètes
/// canonisés, tickers crypto nus (BTC, ETH…) convertis en paire euro — pour
/// que la saisie manuelle fusionne avec les instruments importés au lieu de
/// créer des doublons sans cotation.
pub fn normalize_symbol_input(input: &str) -> String {
    let up = input.trim().to_uppercase();
    for (known, market, _) in CRYPTO_ALIASES {
        if up == *known {
            return (*market).to_string();
        }
    }
    canonical_symbol(&up)
}

/// Résout un symbole crypto (BTC, ETH…) vers sa paire euro Yahoo.
pub fn resolve_crypto(symbol: &str, fallback_name: &str) -> InstrumentRef {
    let up = symbol.trim().to_uppercase();
    for (known, market, name) in CRYPTO_ALIASES {
        if up == *known {
            return InstrumentRef {
                symbol: Some((*market).to_string()),
                isin: None,
                name: (*name).to_string(),
            };
        }
    }
    // Convention Yahoo : <symbole>-EUR pour une crypto cotée en euros.
    InstrumentRef {
        symbol: Some(format!("{up}-EUR")),
        isin: None,
        name: fallback_name.trim().to_string(),
    }
}

/// Nom canonique pour un symbole Yahoo (imports portfolio.csv).
pub fn name_for_symbol(symbol: &str) -> String {
    for (_, sym, name) in LABEL_ALIASES {
        if symbol.eq_ignore_ascii_case(sym) {
            return (*name).to_string();
        }
    }
    symbol.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_labels() {
        let dsy = resolve_label("DASSAULT SYSTEMES");
        assert_eq!(dsy.symbol.as_deref(), Some("DSY.PA"));
        // La ligne PF28 (changement de forme de détention) reste Air Liquide.
        let ai = resolve_label("AIR LIQUIDE PF28");
        assert_eq!(ai.symbol.as_deref(), Some("AI.PA"));
    }

    #[test]
    fn manual_symbol_input_is_normalized() {
        assert_eq!(normalize_symbol_input(" btc "), "BTC-EUR");
        assert_eq!(normalize_symbol_input("FDJ.PA"), "FDJU.PA");
        assert_eq!(normalize_symbol_input("ai.pa"), "AI.PA");
        assert_eq!(normalize_symbol_input("SOL-EUR"), "SOL-EUR");
    }

    #[test]
    fn obsolete_symbols_are_canonicalized() {
        assert_eq!(canonical_symbol("FDJ.PA"), "FDJU.PA");
        assert_eq!(canonical_symbol("STLA.PA"), "STLAP.PA");
        assert_eq!(canonical_symbol("STM.PA"), "STMPA.PA");
        assert_eq!(canonical_symbol("DSY.PA"), "DSY.PA");
        // Libellé courtier et symbole obsolète convergent vers le même instrument.
        assert_eq!(
            resolve_label("STELLANTIS").symbol.as_deref(),
            Some("STLAP.PA")
        );
    }

    #[test]
    fn unknown_label_has_no_symbol() {
        let x = resolve_label("SOCIETE INCONNUE");
        assert_eq!(x.symbol, None);
        assert_eq!(x.name, "SOCIETE INCONNUE");
    }
}
