use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TxType {
    Buy,
    Sell,
    Dividend,
    TransferIn,
    TransferOut,
    Split,
    Cash,
    Fee,
    Tax,
    Other,
}

impl TxType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TxType::Buy => "BUY",
            TxType::Sell => "SELL",
            TxType::Dividend => "DIVIDEND",
            TxType::TransferIn => "TRANSFER_IN",
            TxType::TransferOut => "TRANSFER_OUT",
            TxType::Split => "SPLIT",
            TxType::Cash => "CASH",
            TxType::Fee => "FEE",
            TxType::Tax => "TAX",
            TxType::Other => "OTHER",
        }
    }

    pub fn parse(s: &str) -> Option<TxType> {
        Some(match s {
            "BUY" => TxType::Buy,
            "SELL" => TxType::Sell,
            "DIVIDEND" => TxType::Dividend,
            "TRANSFER_IN" => TxType::TransferIn,
            "TRANSFER_OUT" => TxType::TransferOut,
            "SPLIT" => TxType::Split,
            "CASH" => TxType::Cash,
            "FEE" => TxType::Fee,
            "TAX" => TxType::Tax,
            "OTHER" => TxType::Other,
            _ => return None,
        })
    }
}

/// Instrument tel qu'identifié pendant un import, avant insertion en base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstrumentRef {
    /// Symbole de marché (format Yahoo, ex: `DSY.PA`). None si le libellé
    /// courtier n'a pas pu être rapproché d'un instrument connu.
    pub symbol: Option<String>,
    pub name: String,
}

/// Transaction issue d'un fichier courtier, normalisée mais pas encore persistée.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedTransaction {
    pub broker: String,
    pub date: Option<NaiveDate>,
    pub tx_type: TxType,
    pub instrument: Option<InstrumentRef>,
    pub quantity: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub fees: Decimal,
    /// Mouvement d'espèces signé (crédit positif, débit négatif).
    pub amount: Option<Decimal>,
    pub raw_description: String,
    pub row: usize,
    /// Empreinte stable pour la déduplication (inclut un index d'occurrence
    /// pour distinguer deux opérations réellement identiques le même jour).
    pub fingerprint: String,
}

/// Transaction telle que vue par le moteur de lots (déjà persistée).
#[derive(Debug, Clone)]
pub struct EngineTx {
    pub id: i64,
    pub account_id: i64,
    pub broker: String,
    pub date: Option<NaiveDate>,
    pub tx_type: TxType,
    pub instrument_id: Option<i64>,
    pub quantity: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub fees: Decimal,
}
