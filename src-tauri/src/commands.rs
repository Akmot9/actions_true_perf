use chrono::NaiveDate;
use portfolio_core::import::parse_any;
use portfolio_core::replay;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;
use tauri::State;

use crate::db;
use crate::market;

pub struct AppState {
    pub conn: Mutex<rusqlite::Connection>,
}

#[derive(Serialize)]
pub struct ImportReport {
    pub file_name: String,
    pub broker: String,
    pub file_already_imported: bool,
    pub total: usize,
    pub inserted: usize,
    pub duplicates: usize,
    pub by_type: Vec<(String, usize)>,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub fn import_csv(
    state: State<AppState>,
    file_name: String,
    content: String,
) -> Result<ImportReport, String> {
    let conn = state.conn.lock().unwrap();
    do_import(&conn, &file_name, &content)
}

pub fn do_import(
    conn: &rusqlite::Connection,
    file_name: &str,
    content: &str,
) -> Result<ImportReport, String> {
    let parsed = parse_any(content).map_err(|e| e.to_string())?;
    let file_hash = format!("{:x}", Sha256::digest(content.as_bytes()));

    let account_id = db::get_or_create_account(conn, &parsed.broker, &parsed.account_type)
        .map_err(|e| e.to_string())?;
    let (file_id, file_already_imported) =
        db::record_import_file(conn, file_name, &file_hash, &parsed.broker, content)
            .map_err(|e| e.to_string())?;

    let mut inserted = 0;
    let mut duplicates = 0;
    let mut by_type: HashMap<&'static str, usize> = HashMap::new();
    for t in &parsed.transactions {
        let instrument_id = match &t.instrument {
            Some(i) => Some(
                db::get_or_create_instrument(conn, i.symbol.as_deref(), i.isin.as_deref(), &i.name)
                    .map_err(|e| e.to_string())?,
            ),
            None => None,
        };
        if db::insert_transaction(conn, account_id, instrument_id, file_id, t)
            .map_err(|e| e.to_string())?
        {
            inserted += 1;
        } else {
            duplicates += 1;
        }
        *by_type.entry(t.tx_type.as_str()).or_default() += 1;
    }

    // Les cours embarqués dans le fichier servent de cache hors-ligne ; un
    // rafraîchissement Yahoo ultérieur les écrasera.
    for (symbol, price) in &parsed.quotes {
        db::upsert_quote(conn, symbol, price, "import").map_err(|e| e.to_string())?;
    }

    let mut warnings = parsed.warnings.clone();
    for t in &parsed.transactions {
        if let Some(i) = &t.instrument {
            if i.symbol.is_none() {
                warnings.push(format!(
                    "instrument non reconnu : « {} » (ligne {})",
                    i.name, t.row
                ));
            }
        }
    }

    let mut by_type: Vec<(String, usize)> = by_type
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    by_type.sort_by_key(|item| std::cmp::Reverse(item.1));

    Ok(ImportReport {
        file_name: file_name.to_string(),
        broker: parsed.broker,
        file_already_imported,
        total: parsed.transactions.len(),
        inserted,
        duplicates,
        by_type,
        warnings,
    })
}

#[derive(Serialize)]
pub struct LotView {
    pub id: usize,
    /// Transaction sous-jacente (achat, ou transfert pour un lot non
    /// rapproché) : c'est elle qui est éditable.
    pub tx_id: Option<i64>,
    pub edited: bool,
    pub manual: bool,
    pub acquisition_date: Option<NaiveDate>,
    pub origin_broker: String,
    pub account: String,
    pub initial_quantity: Decimal,
    pub remaining_quantity: Decimal,
    pub unit_cost: Decimal,
    pub fees: Decimal,
    pub invested: Decimal,
    pub market_value: Option<Decimal>,
    pub pnl: Option<Decimal>,
    pub pnl_pct: Option<Decimal>,
    pub unreconciled: bool,
    pub income_events: u32,
}

#[derive(Serialize)]
pub struct PositionView {
    pub instrument_id: i64,
    pub symbol: Option<String>,
    pub name: String,
    pub quantity: Decimal,
    pub invested: Decimal,
    pub avg_cost: Option<Decimal>,
    pub quote: Option<db::QuoteRow>,
    pub market_value: Option<Decimal>,
    pub pnl: Option<Decimal>,
    pub pnl_pct: Option<Decimal>,
    pub dividends: Decimal,
    pub realized_pnl: Decimal,
    pub unreconciled_quantity: Decimal,
    pub lots: Vec<LotView>,
}

#[derive(Serialize)]
pub struct PortfolioView {
    pub has_demo_data: bool,
    pub positions: Vec<PositionView>,
    pub total_invested: Decimal,
    pub total_market_value: Decimal,
    pub total_pnl: Decimal,
    pub total_dividends: Decimal,
    pub total_realized_pnl: Decimal,
    pub warnings: Vec<String>,
}

fn pct(pnl: Decimal, invested: Decimal) -> Option<Decimal> {
    if invested.is_zero() {
        None
    } else {
        Some((pnl / invested * Decimal::ONE_HUNDRED).round_dp(2))
    }
}

#[tauri::command]
pub fn get_portfolio(state: State<AppState>) -> Result<PortfolioView, String> {
    let conn = state.conn.lock().unwrap();
    build_portfolio(&conn)
}

pub fn build_portfolio(conn: &rusqlite::Connection) -> Result<PortfolioView, String> {
    let txs = db::load_engine_txs(conn).map_err(|e| e.to_string())?;
    let instruments = db::load_instruments(conn).map_err(|e| e.to_string())?;
    let quotes = db::load_quotes(conn).map_err(|e| e.to_string())?;
    let dividends = db::load_dividends(conn).map_err(|e| e.to_string())?;
    let edited_ids = db::load_edited_ids(conn).map_err(|e| e.to_string())?;
    let manual_ids = db::load_manual_ids(conn).map_err(|e| e.to_string())?;

    let out = replay(&txs);

    let mut accounts: HashMap<i64, String> = HashMap::new();
    for t in &txs {
        accounts
            .entry(t.account_id)
            .or_insert_with(|| t.broker.clone());
    }

    let mut realized_by_instrument: HashMap<i64, Decimal> = HashMap::new();
    for d in &out.disposals {
        let instrument_id = out.lots[d.lot_id].instrument_id;
        *realized_by_instrument.entry(instrument_id).or_default() += d.realized_pnl;
    }

    let mut by_instrument: HashMap<i64, Vec<&portfolio_core::Lot>> = HashMap::new();
    for lot in out.lots.iter().filter(|l| !l.remaining_quantity.is_zero()) {
        by_instrument
            .entry(lot.instrument_id)
            .or_default()
            .push(lot);
    }

    let mut positions = Vec::new();
    for (instrument_id, mut lots) in by_instrument {
        lots.sort_by_key(|l| (l.acquisition_date.unwrap_or(NaiveDate::MIN), l.id));
        let inst = instruments.get(&instrument_id);
        let symbol = inst.and_then(|i| i.symbol.clone());
        let quote = symbol.as_ref().and_then(|s| quotes.get(s)).cloned();

        let mut lot_views = Vec::new();
        let mut quantity = Decimal::ZERO;
        let mut invested = Decimal::ZERO;
        let mut market_value = Decimal::ZERO;
        let mut unreconciled_quantity = Decimal::ZERO;
        for lot in &lots {
            let lot_invested = lot.invested_remaining();
            let lot_value = quote.as_ref().map(|q| lot.remaining_quantity * q.price);
            let lot_pnl = lot_value.map(|v| v - lot_invested);
            quantity += lot.remaining_quantity;
            invested += lot_invested;
            if let Some(v) = lot_value {
                market_value += v;
            }
            if lot.unreconciled {
                unreconciled_quantity += lot.remaining_quantity;
            }
            let manual = lot.buy_tx_id.is_some_and(|id| manual_ids.contains(&id));
            lot_views.push(LotView {
                id: lot.id,
                tx_id: lot.buy_tx_id,
                edited: !manual && lot.buy_tx_id.is_some_and(|id| edited_ids.contains(&id)),
                manual,
                acquisition_date: lot.acquisition_date,
                origin_broker: lot.origin_broker.clone(),
                account: accounts.get(&lot.account_id).cloned().unwrap_or_default(),
                initial_quantity: lot.initial_quantity,
                remaining_quantity: lot.remaining_quantity,
                unit_cost: lot.unit_cost,
                fees: lot.fees,
                invested: lot_invested.round_dp(2),
                market_value: lot_value.map(|v| v.round_dp(2)),
                pnl: lot_pnl.map(|p| p.round_dp(2)),
                pnl_pct: lot_pnl.and_then(|p| pct(p, lot_invested)),
                unreconciled: lot.unreconciled,
                income_events: lot.income_events,
            });
        }

        let has_quote = quote.is_some();
        let pnl = has_quote.then(|| market_value - invested);
        positions.push(PositionView {
            instrument_id,
            symbol,
            name: inst
                .map(|i| i.name.clone())
                .unwrap_or_else(|| format!("instrument #{instrument_id}")),
            quantity,
            invested: invested.round_dp(2),
            avg_cost: if quantity.is_zero() {
                None
            } else {
                Some((invested / quantity).round_dp(4))
            },
            market_value: has_quote.then(|| market_value.round_dp(2)),
            pnl: pnl.map(|p| p.round_dp(2)),
            pnl_pct: pnl.and_then(|p| pct(p, invested)),
            dividends: dividends.get(&instrument_id).copied().unwrap_or_default(),
            realized_pnl: realized_by_instrument
                .get(&instrument_id)
                .copied()
                .unwrap_or_default()
                .round_dp(2),
            quote,
            unreconciled_quantity,
            lots: lot_views,
        });
    }

    positions.sort_by(|a, b| {
        b.market_value
            .unwrap_or(b.invested)
            .cmp(&a.market_value.unwrap_or(a.invested))
    });

    let total_invested: Decimal = positions.iter().map(|p| p.invested).sum();
    let total_market_value: Decimal = positions.iter().filter_map(|p| p.market_value).sum();
    let total_pnl: Decimal = positions.iter().filter_map(|p| p.pnl).sum();
    // Les revenus restent acquis même si la dernière part de l'instrument a
    // été vendue : le total ne doit donc pas dépendre des seules positions
    // encore ouvertes.
    let total_dividends: Decimal = dividends.values().copied().sum();
    let total_realized_pnl: Decimal = realized_by_instrument
        .values()
        .copied()
        .sum::<Decimal>()
        .round_dp(2);

    Ok(PortfolioView {
        has_demo_data: db::has_demo_data(conn).map_err(|e| e.to_string())?,
        positions,
        total_invested: total_invested.round_dp(2),
        total_market_value: total_market_value.round_dp(2),
        total_pnl: total_pnl.round_dp(2),
        total_dividends: total_dividends.round_dp(2),
        total_realized_pnl,
        warnings: out.warnings,
    })
}

fn parse_dec(value: &str, label: &str) -> Result<Decimal, String> {
    Decimal::from_str(value.trim().replace(',', ".").as_str())
        .map_err(|_| format!("{label} invalide : « {value} »"))
}

#[derive(Debug, Deserialize)]
pub struct ManualTransactionInput {
    pub operation: String,
    pub date: String,
    pub instrument_name: String,
    pub symbol: Option<String>,
    pub broker: String,
    pub account_type: String,
    pub quantity: Option<String>,
    pub unit_price: Option<String>,
    pub fees: String,
    pub amount: Option<String>,
}

struct ValidatedManualTransaction {
    date: NaiveDate,
    tx_type: &'static str,
    operation_label: &'static str,
    instrument_name: String,
    symbol: Option<String>,
    broker: String,
    account_type: String,
    quantity: Option<Decimal>,
    unit_price: Option<Decimal>,
    fees: Decimal,
    amount: Option<Decimal>,
}

fn optional_dec(value: Option<String>, label: &str) -> Result<Option<Decimal>, String> {
    value
        .filter(|v| !v.trim().is_empty())
        .map(|v| parse_dec(&v, label))
        .transpose()
}

fn validate_manual_transaction(
    input: ManualTransactionInput,
) -> Result<ValidatedManualTransaction, String> {
    let date = NaiveDate::parse_from_str(input.date.trim(), "%Y-%m-%d")
        .map_err(|_| format!("date invalide : « {} »", input.date))?;
    let instrument_name = input.instrument_name.trim().to_string();
    if instrument_name.is_empty() {
        return Err("le nom de l'instrument est obligatoire".to_string());
    }
    let broker = input.broker.trim().to_string();
    if broker.is_empty() {
        return Err("le courtier ou le compte est obligatoire".to_string());
    }
    let account_type = input.account_type.trim().to_string();
    if account_type.is_empty() {
        return Err("le type de compte est obligatoire".to_string());
    }
    let symbol = input
        .symbol
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty());
    let fees = if input.fees.trim().is_empty() {
        Decimal::ZERO
    } else {
        parse_dec(&input.fees, "frais")?
    };
    if fees < Decimal::ZERO {
        return Err("les frais ne peuvent pas être négatifs".to_string());
    }

    let quantity = optional_dec(input.quantity, "quantité")?;
    let unit_price = optional_dec(input.unit_price, "prix")?;
    let amount = optional_dec(input.amount, "montant")?;
    let operation = input.operation.trim().to_uppercase();
    let (tx_type, operation_label, quantity, unit_price, amount) = match operation.as_str() {
        "BUY" | "SELL" | "STAKING" => {
            let quantity = quantity
                .filter(|q| *q > Decimal::ZERO)
                .ok_or_else(|| "la quantité doit être strictement positive".to_string())?;
            let unit_price = unit_price
                .filter(|p| *p >= Decimal::ZERO)
                .ok_or_else(|| "le prix ne peut pas être négatif ou vide".to_string())?;
            let (tx_type, label) = match operation.as_str() {
                "BUY" => ("BUY", "Achat manuel"),
                "SELL" => ("SELL", "Vente manuelle"),
                _ => ("DIVIDEND", "Staking manuel"),
            };
            (tx_type, label, Some(quantity), Some(unit_price), None)
        }
        "DIVIDEND" => {
            let amount = amount
                .filter(|a| *a > Decimal::ZERO)
                .ok_or_else(|| "le montant doit être strictement positif".to_string())?;
            ("DIVIDEND", "Dividende manuel", None, None, Some(amount))
        }
        _ => {
            return Err(format!(
                "type d'opération non pris en charge : « {} »",
                input.operation
            ));
        }
    };

    Ok(ValidatedManualTransaction {
        date,
        tx_type,
        operation_label,
        instrument_name,
        symbol,
        broker,
        account_type,
        quantity,
        unit_price,
        fees,
        amount,
    })
}

fn save_manual_transaction(
    conn: &rusqlite::Connection,
    id: Option<i64>,
    input: ManualTransactionInput,
) -> Result<i64, String> {
    let input = validate_manual_transaction(input)?;
    let account_id = db::get_or_create_account(conn, &input.broker, &input.account_type)
        .map_err(|e| e.to_string())?;
    let instrument_id =
        db::get_or_create_instrument(conn, input.symbol.as_deref(), None, &input.instrument_name)
            .map_err(|e| e.to_string())?;
    let description = format!("[MANUEL] {}", input.operation_label);

    if let Some(id) = id {
        db::update_manual_transaction(
            conn,
            id,
            account_id,
            instrument_id,
            input.date,
            input.tx_type,
            input.quantity.as_ref(),
            input.unit_price.as_ref(),
            &input.fees,
            input.amount.as_ref(),
            &description,
        )
        .map_err(|_| "ligne manuelle introuvable ou protégée".to_string())?;
        Ok(id)
    } else {
        db::insert_manual_transaction(
            conn,
            account_id,
            instrument_id,
            input.date,
            input.tx_type,
            input.quantity.as_ref(),
            input.unit_price.as_ref(),
            &input.fees,
            input.amount.as_ref(),
            &description,
        )
        .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn add_manual_transaction(
    state: State<AppState>,
    input: ManualTransactionInput,
) -> Result<i64, String> {
    let conn = state.conn.lock().unwrap();
    save_manual_transaction(&conn, None, input)
}

#[tauri::command]
pub fn update_manual_transaction(
    state: State<AppState>,
    id: i64,
    input: ManualTransactionInput,
) -> Result<i64, String> {
    let conn = state.conn.lock().unwrap();
    save_manual_transaction(&conn, Some(id), input)
}

#[tauri::command]
pub fn get_manual_transactions(
    state: State<AppState>,
) -> Result<Vec<db::ManualTransactionRow>, String> {
    let conn = state.conn.lock().unwrap();
    db::load_manual_transactions(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_manual_transaction(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::delete_manual_transaction(&conn, id)
        .map_err(|_| "ligne manuelle introuvable ou protégée".to_string())
}

#[tauri::command]
pub fn update_transaction(
    state: State<AppState>,
    id: i64,
    date: Option<String>,
    quantity: String,
    unit_price: String,
    fees: String,
) -> Result<(), String> {
    let date = match date.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(d) => Some(
            NaiveDate::parse_from_str(d, "%Y-%m-%d")
                .map_err(|_| format!("date invalide : « {d} »"))?,
        ),
    };
    let quantity = parse_dec(&quantity, "quantité")?;
    if quantity <= Decimal::ZERO {
        return Err("la quantité doit être strictement positive".to_string());
    }
    let unit_price = parse_dec(&unit_price, "prix")?;
    if unit_price < Decimal::ZERO {
        return Err("le prix ne peut pas être négatif".to_string());
    }
    let fees = parse_dec(&fees, "frais")?;
    if fees < Decimal::ZERO {
        return Err("les frais ne peuvent pas être négatifs".to_string());
    }

    let conn = state.conn.lock().unwrap();
    db::update_transaction(&conn, id, date, &quantity, &unit_price, &fees)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn revert_transaction(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::revert_transaction(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_demo_data(state: State<AppState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    db::delete_demo_data(&conn).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct QuoteRefresh {
    pub symbol: String,
    pub price: Option<Decimal>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn refresh_quotes(state: State<'_, AppState>) -> Result<Vec<QuoteRefresh>, String> {
    // Ne rafraîchit que les instruments encore en portefeuille.
    let symbols: Vec<String> = {
        let conn = state.conn.lock().unwrap();
        let txs = db::load_engine_txs(&conn).map_err(|e| e.to_string())?;
        let instruments = db::load_instruments(&conn).map_err(|e| e.to_string())?;
        let out = replay(&txs);
        let mut symbols: Vec<String> = out
            .lots
            .iter()
            .filter(|l| !l.remaining_quantity.is_zero())
            .filter_map(|l| {
                instruments
                    .get(&l.instrument_id)
                    .and_then(|i| i.symbol.clone())
            })
            .collect();
        symbols.sort();
        symbols.dedup();
        symbols
    };

    let results = market::fetch_quotes(&symbols).await;

    let conn = state.conn.lock().unwrap();
    let mut report = Vec::new();
    for r in results {
        match r.result {
            Ok(price) => {
                db::upsert_quote(&conn, &r.symbol, &price, "yahoo").map_err(|e| e.to_string())?;
                report.push(QuoteRefresh {
                    symbol: r.symbol,
                    price: Some(price),
                    error: None,
                });
            }
            Err(e) => report.push(QuoteRefresh {
                symbol: r.symbol,
                price: None,
                error: Some(e),
            }),
        }
    }
    Ok(report)
}
