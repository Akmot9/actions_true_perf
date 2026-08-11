//! Import des relevés de compte Bourse Direct.
//!
//! Format observé : CSV UTF-8, séparateur virgule, décimales à point,
//! en-têtes `Date,Désignation,Qté,Cours,Crédit (€),Débit (€)`.
//! La classification repose sur le préfixe de la désignation ; toute ligne
//! non reconnue devient OTHER avec un avertissement — jamais ignorée en
//! silence.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::{parse_decimal, FingerprintBuilder, ImportError, ParsedFile};
use crate::domain::{ImportedTransaction, InstrumentRef, TxType};
use crate::instruments::resolve_label;

pub const BROKER: &str = "Bourse Direct";

pub fn detect(headers: &[String]) -> bool {
    let has = |name: &str| headers.iter().any(|h| h.starts_with(name));
    has("Date") && has("Désignation") && has("Crédit")
}

pub fn parse(content: &str) -> Result<ParsedFile, ImportError> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(content.as_bytes());
    let mut txs = Vec::new();
    let mut warnings = Vec::new();
    let mut fp = FingerprintBuilder::new();

    for (i, record) in reader.records().enumerate() {
        let record = record?;
        let row = i + 2; // 1-indexé, après l'en-tête
        let field = |n: usize| record.get(n).unwrap_or("").trim().to_string();
        let (date_s, label) = (field(0), field(1));
        let qty = parse_decimal(&field(2));
        let price = parse_decimal(&field(3));
        let credit = parse_decimal(&field(4));
        let debit = parse_decimal(&field(5));

        if label.is_empty() && date_s.is_empty() {
            continue;
        }
        let date = NaiveDate::parse_from_str(&date_s, "%d/%m/%Y").ok();
        if date.is_none() {
            warnings.push(format!("ligne {row}: date invalide « {date_s} »"));
        }

        let (tx_type, instrument, quantity, unit_price, fees, amount) =
            classify(&label, qty, price, credit, debit, row, &mut warnings);

        let fingerprint = fp.fingerprint(&[
            BROKER,
            &date_s,
            tx_type.as_str(),
            &label,
            &field(2),
            &field(3),
            &field(4),
            &field(5),
        ]);

        txs.push(ImportedTransaction {
            broker: BROKER.to_string(),
            date,
            tx_type,
            instrument,
            quantity,
            unit_price,
            fees,
            amount,
            raw_description: label,
            row,
            fingerprint,
        });
    }

    Ok(ParsedFile {
        broker: BROKER.to_string(),
        account_type: "PEA".to_string(),
        transactions: txs,
        warnings,
        quotes: Vec::new(),
    })
}

#[allow(clippy::type_complexity)]
fn classify(
    label: &str,
    qty: Option<Decimal>,
    price: Option<Decimal>,
    credit: Option<Decimal>,
    debit: Option<Decimal>,
    row: usize,
    warnings: &mut Vec<String>,
) -> (
    TxType,
    Option<InstrumentRef>,
    Option<Decimal>,
    Option<Decimal>,
    Decimal,
    Option<Decimal>,
) {
    let cash_amount = credit.unwrap_or(Decimal::ZERO) - debit.unwrap_or(Decimal::ZERO);
    let instr = |rest: &str| Some(resolve_label(rest));

    if let Some(rest) = label.strip_prefix("VIRT TITRES ") {
        // Transfert de titres entre courtiers : surtout pas un achat.
        // Le cours indiqué est le PRU calculé par le courtier, conservé comme
        // valeur de repli pour les quantités sans historique.
        return (
            TxType::TransferIn,
            instr(rest),
            qty,
            price,
            Decimal::ZERO,
            None,
        );
    }
    if let Some(rest) = label.strip_prefix("ACH CPT ") {
        // Frais = débit total - quantité x cours.
        let fees = match (qty, price, debit) {
            (Some(q), Some(p), Some(d)) => (d - q * p).max(Decimal::ZERO),
            _ => Decimal::ZERO,
        };
        return (
            TxType::Buy,
            instr(rest),
            qty,
            price,
            fees,
            debit.map(|d| -d),
        );
    }
    if let Some(rest) = label.strip_prefix("VTE CPT ") {
        let fees = match (qty, price, credit) {
            (Some(q), Some(p), Some(c)) => (q * p - c).max(Decimal::ZERO),
            _ => Decimal::ZERO,
        };
        return (TxType::Sell, instr(rest), qty, price, fees, credit);
    }
    if let Some(rest) = label.strip_prefix("COUPONS ") {
        return (
            TxType::Dividend,
            instr(rest),
            None,
            None,
            Decimal::ZERO,
            credit,
        );
    }
    if let Some(rest) = label.strip_prefix("INDEMNISATION ") {
        // OST : retrait forcé de titres contre indemnité (ex: Atos).
        // Traité comme une cession au prix d'indemnisation pour clôturer les
        // lots avec la moins-value réalisée correspondante.
        if let Some(q) = qty {
            if q < Decimal::ZERO {
                warnings.push(format!(
                    "ligne {row}: OST « {label} » traitée comme cession de {} titres",
                    -q
                ));
                return (
                    TxType::Sell,
                    instr(rest),
                    Some(-q),
                    price,
                    Decimal::ZERO,
                    None,
                );
            }
        }
        return (
            TxType::Other,
            instr(rest),
            qty,
            price,
            Decimal::ZERO,
            Some(cash_amount),
        );
    }
    if label.starts_with("TRANSFERT VALEUR ") {
        // Changement de ligne interne (ex: passage AIR LIQUIDE -> PF28) :
        // sortie et entrée se compensent dans le même compte, aucun effet
        // sur les lots. Conservé pour la timeline.
        return (TxType::Other, None, qty, price, Decimal::ZERO, None);
    }
    if let Some(rest) = label.strip_prefix("ROMPU ") {
        // Vente de rompus lors d'une OST : espèces reçues, quantité inconnue.
        return (
            TxType::Other,
            instr(rest),
            None,
            price,
            Decimal::ZERO,
            credit,
        );
    }
    if label.contains("FRAIS") {
        return (
            TxType::Fee,
            None,
            None,
            None,
            debit.unwrap_or(Decimal::ZERO),
            Some(cash_amount),
        );
    }
    if label.starts_with("INVESTISSEMENT ESPECES")
        || label.starts_with("REGULARISATION")
        || label.starts_with("ESPECES SUR OST")
    {
        return (
            TxType::Cash,
            None,
            None,
            None,
            Decimal::ZERO,
            Some(cash_amount),
        );
    }

    warnings.push(format!(
        "ligne {row}: opération non reconnue « {label} », classée OTHER"
    ));
    (
        TxType::Other,
        None,
        qty,
        price,
        Decimal::ZERO,
        Some(cash_amount),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const SAMPLE: &str = "\
Date,Désignation,Qté,Cours,Crédit (€),Débit (€)
04/03/2025,VIRT TITRES DASSAULT SYSTEMES,33,38.0703,,
01/04/2025,COUPONS TOTALENERGIES SE,,,5.53,
05/05/2025,INDEMNISATION ATOS,-15,0.00368,,
16/05/2025,ACH CPT NEXITY,17,9.6,,164.02
04/08/2025,REGULARISATION PEA INT. FRAIS NOMI Juin25,,,,18
02/01/2026,TRANSFERT VALEUR AIR LIQUIDE,-3,,,
28/08/2025,INVESTISSEMENT ESPECES VIRT CYPRIEN AVICO,,,150,
";

    #[test]
    fn detects_headers() {
        let headers: Vec<String> = [
            "Date",
            "Désignation",
            "Qté",
            "Cours",
            "Crédit (€)",
            "Débit (€)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert!(detect(&headers));
    }

    #[test]
    fn virt_titres_is_transfer_in_never_buy() {
        let parsed = parse(SAMPLE).unwrap();
        let t = &parsed.transactions[0];
        assert_eq!(t.tx_type, TxType::TransferIn);
        assert_eq!(
            t.instrument.as_ref().unwrap().symbol.as_deref(),
            Some("DSY.PA")
        );
        assert_eq!(t.quantity, Some(dec!(33)));
        assert_eq!(t.unit_price, Some(dec!(38.0703)));
    }

    #[test]
    fn classifies_all_row_kinds() {
        let parsed = parse(SAMPLE).unwrap();
        let types: Vec<TxType> = parsed.transactions.iter().map(|t| t.tx_type).collect();
        assert_eq!(
            types,
            vec![
                TxType::TransferIn,
                TxType::Dividend,
                TxType::Sell, // indemnisation Atos => cession forcée
                TxType::Buy,
                TxType::Fee,
                TxType::Other, // transfert valeur interne
                TxType::Cash,
            ]
        );
    }

    #[test]
    fn buy_fees_derived_from_debit() {
        let parsed = parse(SAMPLE).unwrap();
        let buy = &parsed.transactions[3];
        // 164.02 - 17*9.6 = 0.82 € de frais
        assert_eq!(buy.fees, dec!(0.82));
    }

    #[test]
    fn indemnisation_becomes_positive_quantity_sell() {
        let parsed = parse(SAMPLE).unwrap();
        let sell = &parsed.transactions[2];
        assert_eq!(sell.quantity, Some(dec!(15)));
        assert_eq!(sell.unit_price, Some(dec!(0.00368)));
    }
}
