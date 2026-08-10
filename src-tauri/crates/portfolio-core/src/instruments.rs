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

/// Résout un libellé courtier (ex: "DASSAULT SYSTEMES") vers un instrument.
pub fn resolve_label(label: &str) -> InstrumentRef {
    let norm = label.trim().to_uppercase();
    for (alias, symbol, name) in LABEL_ALIASES {
        if norm == *alias {
            return InstrumentRef {
                symbol: Some((*symbol).to_string()),
                name: (*name).to_string(),
            };
        }
    }
    InstrumentRef {
        symbol: None,
        name: label.trim().to_string(),
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
    fn obsolete_symbols_are_canonicalized() {
        assert_eq!(canonical_symbol("FDJ.PA"), "FDJU.PA");
        assert_eq!(canonical_symbol("STLA.PA"), "STLAP.PA");
        assert_eq!(canonical_symbol("STM.PA"), "STMPA.PA");
        assert_eq!(canonical_symbol("DSY.PA"), "DSY.PA");
        // Libellé courtier et symbole obsolète convergent vers le même instrument.
        assert_eq!(resolve_label("STELLANTIS").symbol.as_deref(), Some("STLAP.PA"));
    }

    #[test]
    fn unknown_label_has_no_symbol() {
        let x = resolve_label("SOCIETE INCONNUE");
        assert_eq!(x.symbol, None);
        assert_eq!(x.name, "SOCIETE INCONNUE");
    }
}
