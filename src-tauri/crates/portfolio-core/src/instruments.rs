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
    ("FDJ", "FDJ.PA", "FDJ United"),
    ("FDJ UNITED", "FDJ.PA", "FDJ United"),
    ("DASSAULT SYSTEMES", "DSY.PA", "Dassault Systèmes"),
    ("STMICROELECTRONICS", "STM.PA", "STMicroelectronics"),
    ("STELLANTIS", "STLA.PA", "Stellantis"),
];

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
    fn unknown_label_has_no_symbol() {
        let x = resolve_label("SOCIETE INCONNUE");
        assert_eq!(x.symbol, None);
        assert_eq!(x.name, "SOCIETE INCONNUE");
    }
}
