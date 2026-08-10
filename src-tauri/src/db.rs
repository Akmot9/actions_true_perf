use chrono::NaiveDate;
use portfolio_core::domain::{EngineTx, ImportedTransaction, TxType};
use rusqlite::{params, Connection};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

const SCHEMA: &str = "
CREATE TABLE accounts (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    broker TEXT NOT NULL UNIQUE,
    account_type TEXT NOT NULL DEFAULT 'PEA',
    currency TEXT NOT NULL DEFAULT 'EUR'
);
CREATE TABLE instruments (
    id INTEGER PRIMARY KEY,
    symbol TEXT UNIQUE,
    name TEXT NOT NULL,
    currency TEXT NOT NULL DEFAULT 'EUR'
);
CREATE TABLE import_files (
    id INTEGER PRIMARY KEY,
    file_name TEXT NOT NULL,
    file_hash TEXT NOT NULL UNIQUE,
    broker TEXT NOT NULL,
    imported_at TEXT NOT NULL DEFAULT (datetime('now')),
    raw_content BLOB NOT NULL
);
CREATE TABLE transactions (
    id INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL REFERENCES accounts(id),
    date TEXT,
    type TEXT NOT NULL,
    instrument_id INTEGER REFERENCES instruments(id),
    quantity TEXT,
    unit_price TEXT,
    fees TEXT NOT NULL DEFAULT '0',
    amount TEXT,
    currency TEXT NOT NULL DEFAULT 'EUR',
    raw_description TEXT NOT NULL,
    source_file_id INTEGER REFERENCES import_files(id),
    fingerprint TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE quotes (
    symbol TEXT PRIMARY KEY,
    price TEXT NOT NULL,
    provider TEXT NOT NULL,
    fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
);
";

/// Migrations appliquées séquentiellement ; `user_version` mémorise la
/// dernière appliquée. Ne jamais modifier une migration publiée : en ajouter.
const MIGRATIONS: &[&str] = &[
    SCHEMA,
    // v2 : édition manuelle d'une transaction. Les valeurs d'origine sont
    // archivées dans original_json à la première édition (restaurables) ;
    // l'empreinte de déduplication reste inchangée.
    "ALTER TABLE transactions ADD COLUMN edited_at TEXT;
     ALTER TABLE transactions ADD COLUMN original_json TEXT;",
    // v3 : canonicalisation des symboles obsolètes (FDJ renommée FDJ United,
    // codes Paris Yahoo de Stellantis/STMicro différents des codes historiques).
    "UPDATE OR IGNORE instruments SET symbol = 'FDJU.PA'  WHERE symbol = 'FDJ.PA';
     UPDATE OR IGNORE instruments SET symbol = 'STLAP.PA' WHERE symbol = 'STLA.PA';
     UPDATE OR IGNORE instruments SET symbol = 'STMPA.PA' WHERE symbol = 'STM.PA';
     UPDATE OR IGNORE quotes SET symbol = 'FDJU.PA'  WHERE symbol = 'FDJ.PA';
     UPDATE OR IGNORE quotes SET symbol = 'STLAP.PA' WHERE symbol = 'STLA.PA';
     UPDATE OR IGNORE quotes SET symbol = 'STMPA.PA' WHERE symbol = 'STM.PA';
     DELETE FROM quotes WHERE symbol IN ('FDJ.PA', 'STLA.PA', 'STM.PA');",
    // v4 : ISIN sur les instruments — clé de rapprochement la plus fiable
    // entre courtiers (Trade Republic identifie les titres par ISIN).
    "ALTER TABLE instruments ADD COLUMN isin TEXT;",
];

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let mut version: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let target = (i + 1) as i32;
        if version < target {
            conn.execute_batch(sql)?;
            conn.pragma_update(None, "user_version", target)?;
            version = target;
        }
    }
    Ok(conn)
}

pub fn get_or_create_account(conn: &Connection, broker: &str, account_type: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO accounts (name, broker, account_type) VALUES (?1, ?2, ?3)
         ON CONFLICT(broker) DO NOTHING",
        params![format!("{account_type} {broker}"), broker, account_type],
    )?;
    conn.query_row("SELECT id FROM accounts WHERE broker = ?1", params![broker], |r| r.get(0))
}

/// Retrouve un instrument par ISIN, sinon par symbole, sinon par nom,
/// sinon le crée. Complète ISIN/symbole manquants au passage, si bien que
/// les imports successifs de courtiers différents convergent vers le même
/// instrument (ex: « AIR LIQUIDE » Bourse Direct + FR0000120073 Trade
/// Republic + AI.PA portfolio.csv = une seule position).
pub fn get_or_create_instrument(
    conn: &Connection,
    symbol: Option<&str>,
    isin: Option<&str>,
    name: &str,
) -> rusqlite::Result<i64> {
    if let Some(isin) = isin {
        if let Ok(id) = conn.query_row("SELECT id FROM instruments WHERE isin = ?1", params![isin], |r| r.get(0)) {
            if let Some(sym) = symbol {
                conn.execute("UPDATE instruments SET symbol = ?1 WHERE id = ?2 AND symbol IS NULL", params![sym, id])?;
            }
            return Ok(id);
        }
    }
    if let Some(sym) = symbol {
        if let Ok(id) = conn.query_row("SELECT id FROM instruments WHERE symbol = ?1", params![sym], |r| r.get(0)) {
            if let Some(isin) = isin {
                conn.execute("UPDATE instruments SET isin = ?1 WHERE id = ?2 AND isin IS NULL", params![isin, id])?;
            }
            return Ok(id);
        }
        // Un import précédent a pu créer l'instrument par son nom seul.
        if let Ok(id) = conn.query_row("SELECT id FROM instruments WHERE symbol IS NULL AND name = ?1", params![name], |r| r.get(0))
        {
            conn.execute("UPDATE instruments SET symbol = ?1, isin = COALESCE(isin, ?2) WHERE id = ?3", params![sym, isin, id])?;
            return Ok(id);
        }
    }
    if symbol.is_none() {
        if let Ok(id) = conn.query_row("SELECT id FROM instruments WHERE name = ?1", params![name], |r| r.get(0)) {
            if let Some(isin) = isin {
                conn.execute("UPDATE instruments SET isin = ?1 WHERE id = ?2 AND isin IS NULL", params![isin, id])?;
            }
            return Ok(id);
        }
    }
    conn.execute(
        "INSERT INTO instruments (symbol, isin, name) VALUES (?1, ?2, ?3)",
        params![symbol, isin, name],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn record_import_file(
    conn: &Connection,
    file_name: &str,
    file_hash: &str,
    broker: &str,
    content: &str,
) -> rusqlite::Result<(i64, bool)> {
    if let Ok(id) =
        conn.query_row("SELECT id FROM import_files WHERE file_hash = ?1", params![file_hash], |r| r.get(0))
    {
        return Ok((id, true));
    }
    conn.execute(
        "INSERT INTO import_files (file_name, file_hash, broker, raw_content) VALUES (?1, ?2, ?3, ?4)",
        params![file_name, file_hash, broker, content.as_bytes()],
    )?;
    Ok((conn.last_insert_rowid(), false))
}

/// Insère une transaction importée ; renvoie false si son empreinte existe
/// déjà (doublon d'un import précédent).
pub fn insert_transaction(
    conn: &Connection,
    account_id: i64,
    instrument_id: Option<i64>,
    source_file_id: i64,
    t: &ImportedTransaction,
) -> rusqlite::Result<bool> {
    let n = conn.execute(
        "INSERT OR IGNORE INTO transactions
         (account_id, date, type, instrument_id, quantity, unit_price, fees, amount, raw_description, source_file_id, fingerprint)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            account_id,
            t.date.map(|d| d.to_string()),
            t.tx_type.as_str(),
            instrument_id,
            t.quantity.map(|q| q.to_string()),
            t.unit_price.map(|p| p.to_string()),
            t.fees.to_string(),
            t.amount.map(|a| a.to_string()),
            t.raw_description,
            source_file_id,
            t.fingerprint,
        ],
    )?;
    Ok(n > 0)
}

pub fn upsert_quote(conn: &Connection, symbol: &str, price: &Decimal, provider: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO quotes (symbol, price, provider, fetched_at) VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(symbol) DO UPDATE SET price = ?2, provider = ?3, fetched_at = datetime('now')",
        params![symbol, price.to_string(), provider],
    )?;
    Ok(())
}

fn dec(s: Option<String>) -> Option<Decimal> {
    s.and_then(|s| Decimal::from_str(&s).ok())
}

pub fn load_engine_txs(conn: &Connection) -> rusqlite::Result<Vec<EngineTx>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.account_id, a.broker, t.date, t.type, t.instrument_id, t.quantity, t.unit_price, t.fees
         FROM transactions t JOIN accounts a ON a.id = t.account_id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(EngineTx {
            id: r.get(0)?,
            account_id: r.get(1)?,
            broker: r.get(2)?,
            date: r
                .get::<_, Option<String>>(3)?
                .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()),
            tx_type: TxType::parse(&r.get::<_, String>(4)?).unwrap_or(TxType::Other),
            instrument_id: r.get(5)?,
            quantity: dec(r.get(6)?),
            unit_price: dec(r.get(7)?),
            fees: dec(r.get::<_, Option<String>>(8)?).unwrap_or(Decimal::ZERO),
        })
    })?;
    rows.collect()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InstrumentRow {
    pub id: i64,
    pub symbol: Option<String>,
    pub name: String,
}

pub fn load_instruments(conn: &Connection) -> rusqlite::Result<HashMap<i64, InstrumentRow>> {
    let mut stmt = conn.prepare("SELECT id, symbol, name FROM instruments")?;
    let rows = stmt.query_map([], |r| {
        Ok(InstrumentRow { id: r.get(0)?, symbol: r.get(1)?, name: r.get(2)? })
    })?;
    Ok(rows.filter_map(Result::ok).map(|i| (i.id, i)).collect())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QuoteRow {
    pub price: Decimal,
    pub provider: String,
    pub fetched_at: String,
}

pub fn load_quotes(conn: &Connection) -> rusqlite::Result<HashMap<String, QuoteRow>> {
    let mut stmt = conn.prepare("SELECT symbol, price, provider, fetched_at FROM quotes")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            QuoteRow {
                price: Decimal::from_str(&r.get::<_, String>(1)?).unwrap_or(Decimal::ZERO),
                provider: r.get(2)?,
                fetched_at: r.get(3)?,
            },
        ))
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

/// Modifie une transaction sur demande explicite de l'utilisateur.
/// Les valeurs d'origine sont archivées une seule fois (première édition) ;
/// l'empreinte de déduplication n'est pas recalculée, si bien qu'un réimport
/// du fichier source ne recrée pas l'ordre dans sa version d'origine.
pub fn update_transaction(
    conn: &Connection,
    id: i64,
    date: Option<NaiveDate>,
    quantity: &Decimal,
    unit_price: &Decimal,
    fees: &Decimal,
) -> rusqlite::Result<()> {
    let (o_date, o_qty, o_price, o_fees, original_json) = conn.query_row(
        "SELECT date, quantity, unit_price, fees, original_json FROM transactions WHERE id = ?1",
        params![id],
        |r| {
            Ok((
                r.get::<_, Option<String>>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        },
    )?;
    let original_json = original_json.unwrap_or_else(|| {
        serde_json::json!({
            "date": o_date, "quantity": o_qty, "unit_price": o_price, "fees": o_fees,
        })
        .to_string()
    });
    conn.execute(
        "UPDATE transactions
         SET date = ?2, quantity = ?3, unit_price = ?4, fees = ?5,
             edited_at = datetime('now'), original_json = ?6
         WHERE id = ?1",
        params![
            id,
            date.map(|d| d.to_string()),
            quantity.to_string(),
            unit_price.to_string(),
            fees.to_string(),
            original_json,
        ],
    )?;
    Ok(())
}

/// Restaure les valeurs d'origine d'une transaction éditée.
pub fn revert_transaction(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    let original_json: Option<String> = conn.query_row(
        "SELECT original_json FROM transactions WHERE id = ?1",
        params![id],
        |r| r.get(0),
    )?;
    let Some(json) = original_json else {
        return Ok(()); // jamais éditée : rien à restaurer
    };
    let v: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
    let s = |k: &str| v[k].as_str().map(str::to_string);
    conn.execute(
        "UPDATE transactions
         SET date = ?2, quantity = ?3, unit_price = ?4, fees = COALESCE(?5, '0'),
             edited_at = NULL, original_json = NULL
         WHERE id = ?1",
        params![id, s("date"), s("quantity"), s("unit_price"), s("fees")],
    )?;
    Ok(())
}

/// Identifiants des transactions modifiées manuellement.
pub fn load_edited_ids(conn: &Connection) -> rusqlite::Result<std::collections::HashSet<i64>> {
    let mut stmt = conn.prepare("SELECT id FROM transactions WHERE edited_at IS NOT NULL")?;
    let rows = stmt.query_map([], |r| r.get::<_, i64>(0))?;
    Ok(rows.filter_map(Result::ok).collect())
}

/// Somme des dividendes (nets, tels que crédités) par instrument.
pub fn load_dividends(conn: &Connection) -> rusqlite::Result<HashMap<i64, Decimal>> {
    let mut stmt = conn.prepare(
        "SELECT instrument_id, amount FROM transactions WHERE type = 'DIVIDEND' AND instrument_id IS NOT NULL",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)))?;
    let mut sums: HashMap<i64, Decimal> = HashMap::new();
    for row in rows.filter_map(Result::ok) {
        if let Some(amount) = dec(row.1) {
            *sums.entry(row.0).or_default() += amount;
        }
    }
    Ok(sums)
}
