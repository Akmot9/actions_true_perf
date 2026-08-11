//! Import du fichier `portfolio.csv` (export « portfolio » Yahoo Finance),
//! utilisé comme source de l'historique d'achats BoursoBank.
//!
//! Chaque ligne est un ordre d'achat individuel : `Trade Date` (AAAAMMJJ),
//! `Purchase Price`, `Quantity`. La colonne `Current Price` fournit en prime
//! un cours récent, conservé comme cache de cotation hors-ligne.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::{parse_decimal, FingerprintBuilder, ImportError, ParsedFile};
use crate::domain::{ImportedTransaction, InstrumentRef, TxType};
use crate::instruments::{canonical_symbol, name_for_symbol};

pub const BROKER: &str = "BoursoBank";

pub fn detect(headers: &[String]) -> bool {
    let has = |name: &str| headers.iter().any(|h| h == name);
    has("Symbol") && has("Trade Date") && has("Purchase Price")
}

pub fn parse(content: &str) -> Result<ParsedFile, ImportError> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(content.as_bytes());
    let headers: Vec<String> = reader
        .headers()?
        .iter()
        .map(|h| h.trim().to_string())
        .collect();
    let col = |name: &str| headers.iter().position(|h| h == name);
    let (c_symbol, c_price_now, c_trade_date, c_price, c_qty, c_commission, c_type) = (
        col("Symbol"),
        col("Current Price"),
        col("Trade Date"),
        col("Purchase Price"),
        col("Quantity"),
        col("Commission"),
        col("Transaction Type"),
    );

    let mut txs = Vec::new();
    let mut warnings = Vec::new();
    let mut quotes = Vec::new();
    let mut fp = FingerprintBuilder::new();

    for (i, record) in reader.records().enumerate() {
        let record = record?;
        let row = i + 2;
        let field = |c: Option<usize>| {
            c.and_then(|c| record.get(c))
                .unwrap_or("")
                .trim()
                .to_string()
        };

        let raw_symbol = field(c_symbol);
        if raw_symbol.is_empty() {
            continue;
        }
        let symbol = canonical_symbol(&raw_symbol);
        let tx_type_s = field(c_type);
        if !tx_type_s.is_empty() && tx_type_s != "BUY" {
            warnings.push(format!(
                "ligne {row}: type « {tx_type_s} » non géré pour {symbol}, ignoré"
            ));
            continue;
        }

        if let Some(p) = parse_decimal(&field(c_price_now)) {
            if !quotes.iter().any(|(s, _)| s == &symbol) {
                quotes.push((symbol.clone(), p));
            }
        }

        let quantity = parse_decimal(&field(c_qty));
        let unit_price = parse_decimal(&field(c_price));
        let (Some(quantity), Some(unit_price)) = (quantity, unit_price) else {
            warnings.push(format!("ligne {row}: {symbol} sans quantité/prix, ignorée"));
            continue;
        };

        let trade_date_s = field(c_trade_date);
        let date = NaiveDate::parse_from_str(&trade_date_s, "%Y%m%d").ok();
        if date.is_none() {
            warnings.push(format!(
                "ligne {row}: {symbol} sans date d'achat — lot conservé, daté « inconnu » (traité comme le plus ancien)"
            ));
        }

        let fees = parse_decimal(&field(c_commission)).unwrap_or(Decimal::ZERO);
        // Empreinte calculée sur le symbole BRUT du fichier : la
        // canonicalisation peut évoluer, les empreintes déjà en base non.
        let fingerprint = fp.fingerprint(&[
            BROKER,
            &trade_date_s,
            "BUY",
            &raw_symbol,
            &field(c_qty),
            &field(c_price),
        ]);

        txs.push(ImportedTransaction {
            broker: BROKER.to_string(),
            date,
            tx_type: TxType::Buy,
            instrument: Some(InstrumentRef {
                symbol: Some(symbol.clone()),
                isin: None,
                name: name_for_symbol(&symbol),
            }),
            quantity: Some(quantity),
            unit_price: Some(unit_price),
            fees,
            amount: Some(-(quantity * unit_price + fees)),
            raw_description: format!("BUY {symbol} (portfolio.csv)"),
            row,
            fingerprint,
        });
    }

    Ok(ParsedFile {
        broker: BROKER.to_string(),
        account_type: "PEA".to_string(),
        transactions: txs,
        warnings,
        quotes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const SAMPLE: &str = "\
Symbol,Current Price,Date,Time,Change,Open,High,Low,Volume,Trade Date,Purchase Price,Quantity,Commission,High Limit,Low Limit,Comment,Transaction Type
FDJ.PA,,,,,,,,,,38.0,4.0,,,,,BUY
DSY.PA,22.06,2026/08/10,10:11 CEST,-0.19,22.21,22.21,21.99,102349,20230201,34.0,5.0,,,,,BUY
DSY.PA,22.06,2026/08/10,10:11 CEST,-0.19,22.21,22.21,21.99,102349,20221223,33.4,4.0,,,,,BUY
";

    #[test]
    fn each_row_is_a_distinct_buy_lot() {
        let parsed = parse(SAMPLE).unwrap();
        assert_eq!(parsed.transactions.len(), 3);
        assert!(parsed.transactions.iter().all(|t| t.tx_type == TxType::Buy));
        let dsy1 = &parsed.transactions[1];
        assert_eq!(
            dsy1.date,
            Some(NaiveDate::from_ymd_opt(2023, 2, 1).unwrap())
        );
        assert_eq!(dsy1.unit_price, Some(dec!(34.0)));
        assert_eq!(dsy1.quantity, Some(dec!(5.0)));
    }

    #[test]
    fn missing_trade_date_kept_with_warning() {
        let parsed = parse(SAMPLE).unwrap();
        assert_eq!(parsed.transactions[0].date, None);
        assert!(parsed
            .warnings
            .iter()
            .any(|w| w.contains("sans date d'achat")));
    }

    #[test]
    fn current_prices_become_quotes() {
        let parsed = parse(SAMPLE).unwrap();
        assert_eq!(parsed.quotes, vec![("DSY.PA".to_string(), dec!(22.06))]);
    }

    #[test]
    fn obsolete_symbols_canonicalized_at_import() {
        let parsed = parse(SAMPLE).unwrap();
        // La ligne FDJ.PA du fichier doit produire l'instrument FDJU.PA actuel.
        let fdj = &parsed.transactions[0];
        assert_eq!(
            fdj.instrument.as_ref().unwrap().symbol.as_deref(),
            Some("FDJU.PA")
        );
        assert_eq!(fdj.instrument.as_ref().unwrap().name, "FDJ United");
    }
}
