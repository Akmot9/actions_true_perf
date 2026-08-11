//! Import de l'export CSV officiel Trade Republic (disponible depuis
//! avril 2026 : Profil → Relevés de compte → Export des transactions).
//!
//! Format : CSV UTF-8 entre guillemets, décimales à point, dates ISO.
//! La colonne `symbol` contient l'ISIN pour les titres (clé de rapprochement
//! avec les instruments des autres courtiers) et le ticker pour les cryptos.
//! `transaction_id` (UUID) sert d'empreinte de déduplication.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::{parse_decimal, FingerprintBuilder, ImportError, ParsedFile};
use crate::domain::{ImportedTransaction, TxType};
use crate::instruments::{resolve_crypto, resolve_isin};

pub const BROKER: &str = "Trade Republic";

pub fn detect(headers: &[String]) -> bool {
    let has = |name: &str| headers.iter().any(|h| h == name);
    has("category") && has("asset_class") && has("transaction_id")
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
    let (
        c_date,
        c_category,
        c_type,
        c_asset,
        c_name,
        c_symbol,
        c_shares,
        c_price,
        c_amount,
        c_fee,
        c_tax,
        c_desc,
        c_txid,
    ) = (
        col("date"),
        col("category"),
        col("type"),
        col("asset_class"),
        col("name"),
        col("symbol"),
        col("shares"),
        col("price"),
        col("amount"),
        col("fee"),
        col("tax"),
        col("description"),
        col("transaction_id"),
    );

    let mut txs = Vec::new();
    let mut warnings = Vec::new();
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

        let date_s = field(c_date);
        if date_s.is_empty() {
            continue;
        }
        let date = NaiveDate::parse_from_str(&date_s, "%Y-%m-%d").ok();
        if date.is_none() {
            warnings.push(format!("ligne {row}: date invalide « {date_s} »"));
        }

        let category = field(c_category);
        let ttype = field(c_type);
        let asset_class = field(c_asset);
        let name = field(c_name);
        let symbol = field(c_symbol);
        let shares = parse_decimal(&field(c_shares)).map(|d| d.abs());
        let price = parse_decimal(&field(c_price));
        let amount = parse_decimal(&field(c_amount));
        // Frais et taxes (TTF) apparaissent en négatif ; ils font partie du
        // coût d'acquisition et sont regroupés dans `fees`.
        let fees = parse_decimal(&field(c_fee))
            .map(|d| d.abs())
            .unwrap_or(Decimal::ZERO)
            + parse_decimal(&field(c_tax))
                .map(|d| d.abs())
                .unwrap_or(Decimal::ZERO);

        let instrument = if symbol.is_empty() {
            None
        } else if asset_class == "CRYPTO" {
            Some(resolve_crypto(&symbol, &name))
        } else {
            Some(resolve_isin(&symbol, &name))
        };

        let (tx_type, instrument, quantity, unit_price) = match (category.as_str(), ttype.as_str())
        {
            ("TRADING", "BUY") => (TxType::Buy, instrument, shares, price),
            ("TRADING", "SELL") => (TxType::Sell, instrument, shares, price),
            // Récompenses de staking versées en crypto : dividende en nature,
            // valorisé au prix de marché du jour de réception.
            ("DELIVERY", "FREE_RECEIPT") => (
                TxType::Dividend,
                instrument,
                shares,
                price.or(Some(Decimal::ZERO)),
            ),
            // Attribution gratuite d'actions : lot à coût nul, le coût total
            // de la position est inchangé.
            ("CORPORATE_ACTION", "BONUS_ISSUE") => {
                (TxType::Buy, instrument, shares, Some(Decimal::ZERO))
            }
            ("CASH", "DIVIDEND") => (TxType::Dividend, instrument, None, None),
            // Les achats sur le marché privé apparaissent deux fois (ligne
            // CASH + ligne TRADING) : seule la ligne TRADING crée le lot.
            ("CASH", _) => (TxType::Cash, None, None, None),
            _ => {
                warnings.push(format!(
                    "ligne {row}: opération non reconnue « {category}/{ttype} », classée OTHER"
                ));
                (TxType::Other, instrument, shares, price)
            }
        };

        let transaction_id = field(c_txid);
        let fingerprint = if transaction_id.is_empty() {
            fp.fingerprint(&[
                BROKER,
                &date_s,
                &category,
                &ttype,
                &name,
                &field(c_shares),
                &field(c_amount),
            ])
        } else {
            fp.fingerprint(&[BROKER, &transaction_id])
        };

        let description = field(c_desc);
        let raw_description = if description.is_empty() {
            format!("{category}/{ttype} {name}").trim().to_string()
        } else {
            description
        };

        txs.push(ImportedTransaction {
            broker: BROKER.to_string(),
            date,
            tx_type,
            instrument,
            quantity,
            unit_price,
            fees,
            amount,
            raw_description,
            row,
            fingerprint,
        });
    }

    Ok(ParsedFile {
        broker: BROKER.to_string(),
        account_type: "CTO".to_string(),
        transactions: txs,
        warnings,
        quotes: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const SAMPLE: &str = r#""datetime","date","account_type","category","type","asset_class","name","symbol","shares","price","amount","fee","tax","currency","original_amount","original_currency","fx_rate","description","transaction_id","counterparty_name","counterparty_iban","payment_reference","mcc_code"
"2025-08-25T08:39:40.933Z","2025-08-25","DEFAULT","TRADING","BUY","STOCK","Air Liquide","FR0000120073","0.0198550000","182.320000","-3.62","","-0.01","EUR","","","","Savings plan execution","b29126a1-0000-0000-0000-000000000001","","","",""
"2026-01-16T10:00:00Z","2026-01-16","DEFAULT","TRADING","SELL","CRYPTO","Bitcoin","BTC","-0.0000130000","81948.0200000000","1.07","","","EUR","","","","Sell trade","b29126a1-0000-0000-0000-000000000002","","","",""
"2025-12-17T10:00:00Z","2025-12-17","DEFAULT","DELIVERY","FREE_RECEIPT","CRYPTO","Ethereum","ETH","0.0000020000","2488.4600000000","","","","EUR","","","","FREE_RECEIPT ETH","b29126a1-0000-0000-0000-000000000003","","","",""
"2026-06-08T10:00:00Z","2026-06-08","DEFAULT","CORPORATE_ACTION","BONUS_ISSUE","STOCK","Air Liquide","FR0000120073","0.1205770000","","","","","EUR","","","","BONUS_ISSUE FR0000120073","b29126a1-0000-0000-0000-000000000004","","","",""
"2026-05-20T10:00:00Z","2026-05-20","DEFAULT","CASH","DIVIDEND","STOCK","Air Liquide","FR0000120073","","","1.23","","","EUR","","","","Dividend","b29126a1-0000-0000-0000-000000000005","","","",""
"2025-08-23T12:53:05Z","2025-08-23","DEFAULT","CASH","CARD_TRANSACTION","","CHURCH BEER","","","","-14.000000","","","EUR","","","","CHURCH BEER","b29126a1-0000-0000-0000-000000000006","","","","5813"
"2025-09-01T10:00:00Z","2025-09-01","DEFAULT","MYSTERY","THING","","X","","","","1.00","","","EUR","","","","","b29126a1-0000-0000-0000-000000000007","","","",""
"#;

    #[test]
    fn detects_headers() {
        let headers: Vec<String> = ["date", "category", "type", "asset_class", "transaction_id"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(detect(&headers));
    }

    #[test]
    fn classifies_types_and_merges_isin_with_known_instruments() {
        let parsed = parse(SAMPLE).unwrap();
        assert_eq!(parsed.account_type, "CTO");
        let types: Vec<TxType> = parsed.transactions.iter().map(|t| t.tx_type).collect();
        assert_eq!(
            types,
            vec![
                TxType::Buy,
                TxType::Sell,
                TxType::Dividend, // FREE_RECEIPT => staking au prix de marché
                TxType::Buy,      // BONUS_ISSUE => lot à coût nul
                TxType::Dividend,
                TxType::Cash,
                TxType::Other,
            ]
        );
        // L'ISIN Air Liquide converge vers le même symbole que le PEA.
        let buy = &parsed.transactions[0];
        let inst = buy.instrument.as_ref().unwrap();
        assert_eq!(inst.symbol.as_deref(), Some("AI.PA"));
        assert_eq!(inst.isin.as_deref(), Some("FR0000120073"));
        // Frais = |fee| + |taxe TTF|.
        assert_eq!(buy.fees, dec!(0.01));
        // Achat fractionné.
        assert_eq!(buy.quantity, Some(dec!(0.0198550000)));
        // Le staking conserve quantité et valeur de réception.
        let staking = &parsed.transactions[2];
        assert_eq!(staking.quantity, Some(dec!(0.0000020000)));
        assert_eq!(staking.unit_price, Some(dec!(2488.4600000000)));
    }

    #[test]
    fn crypto_sell_has_positive_quantity_and_eur_pair() {
        let parsed = parse(SAMPLE).unwrap();
        let sell = &parsed.transactions[1];
        assert_eq!(sell.quantity, Some(dec!(0.0000130000)));
        assert_eq!(
            sell.instrument.as_ref().unwrap().symbol.as_deref(),
            Some("BTC-EUR")
        );
    }

    #[test]
    fn bonus_issue_creates_zero_cost_lot() {
        let parsed = parse(SAMPLE).unwrap();
        let bonus = &parsed.transactions[3];
        assert_eq!(bonus.unit_price, Some(Decimal::ZERO));
        assert_eq!(bonus.quantity, Some(dec!(0.1205770000)));
    }

    #[test]
    fn unknown_category_warns_and_becomes_other() {
        let parsed = parse(SAMPLE).unwrap();
        assert!(parsed.warnings.iter().any(|w| w.contains("MYSTERY/THING")));
    }

    #[test]
    fn fingerprints_use_transaction_id() {
        let p1 = parse(SAMPLE).unwrap();
        let p2 = parse(SAMPLE).unwrap();
        assert_eq!(
            p1.transactions[0].fingerprint,
            p2.transactions[0].fingerprint
        );
        let unique: std::collections::HashSet<_> = p1
            .transactions
            .iter()
            .map(|t| t.fingerprint.clone())
            .collect();
        assert_eq!(unique.len(), p1.transactions.len());
    }
}
