use chrono::NaiveDate;
use portfolio_core::domain::{EngineTx, ImportedTransaction, TxType};
use rusqlite::{params, Connection, OptionalExtension};
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

const DEMO_SOURCE_HASH: &str = "builtin-demo-v1";
const DEMO_INITIALIZED_SETTING: &str = "onboarding_demo_v1_initialized";

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
    // v5 : les FREE_RECEIPT crypto de Trade Republic sont des récompenses de
    // staking, donc des dividendes en nature et non des ordres d'achat.
    "UPDATE transactions
     SET type = 'DIVIDEND'
     WHERE type = 'BUY'
       AND raw_description LIKE 'FREE_RECEIPT%'
       AND account_id IN (SELECT id FROM accounts WHERE broker = 'Trade Republic');",
    // v6 : mémorise l'initialisation des données de découverte. Le marqueur
    // empêche leur réapparition après une suppression volontaire.
    "CREATE TABLE app_settings (
         key TEXT PRIMARY KEY,
         value TEXT NOT NULL
     );",
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

fn demo_decimal(value: &str) -> Decimal {
    Decimal::from_str(value).expect("valid built-in demo decimal")
}

fn demo_date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid built-in demo date")
}

#[allow(clippy::too_many_arguments)]
fn demo_transaction(
    broker: &str,
    date: NaiveDate,
    tx_type: TxType,
    quantity: Option<&str>,
    unit_price: Option<&str>,
    fees: &str,
    amount: Option<&str>,
    description: &str,
    fingerprint: &str,
) -> ImportedTransaction {
    ImportedTransaction {
        broker: broker.to_string(),
        date: Some(date),
        tx_type,
        // L'instrument est déjà résolu avant l'insertion du seed.
        instrument: None,
        quantity: quantity.map(demo_decimal),
        unit_price: unit_price.map(demo_decimal),
        fees: demo_decimal(fees),
        amount: amount.map(demo_decimal),
        raw_description: format!("[EXEMPLE] {description}"),
        row: 0,
        fingerprint: fingerprint.to_string(),
    }
}

/// Ajoute un petit portefeuille de découverte uniquement sur une base encore
/// vierge. Le marqueur est écrit même pour une base déjà remplie afin qu'une
/// suppression ultérieure des vraies données ne déclenche jamais le seed.
pub fn seed_demo_if_first_run(conn: &Connection) -> rusqlite::Result<bool> {
    let initialized = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![DEMO_INITIALIZED_SETTING],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .is_some();
    if initialized {
        return Ok(false);
    }

    let transaction_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM transactions", [], |row| row.get(0))?;
    if transaction_count > 0 {
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, 'existing-data')",
            params![DEMO_INITIALIZED_SETTING],
        )?;
        return Ok(false);
    }

    let (source_file_id, _) = record_import_file(
        conn,
        "exemples-premier-lancement.csv",
        DEMO_SOURCE_HASH,
        "Données d'exemple",
        "Jeu de découverte intégré — exemples fournis par l'utilisateur.",
    )?;
    let bourso_account = get_or_create_account(conn, "BoursoBank", "PEA")?;
    let trade_republic_account = get_or_create_account(conn, "Trade Republic", "CTO")?;
    let air_liquide =
        get_or_create_instrument(conn, Some("AI.PA"), Some("FR0000120073"), "Air Liquide")?;
    let totalenergies =
        get_or_create_instrument(conn, Some("TTE.PA"), Some("FR0000120271"), "TotalEnergies")?;
    let solana = get_or_create_instrument(conn, Some("SOL-EUR"), None, "Solana")?;

    let rows = [
        (
            bourso_account,
            air_liquide,
            demo_transaction(
                "BoursoBank",
                demo_date(2022, 3, 3),
                TxType::Buy,
                Some("1"),
                Some("148.28"),
                "0",
                None,
                "Achat Air Liquide",
                "demo-v1-ai-2022-03-03",
            ),
        ),
        (
            bourso_account,
            air_liquide,
            demo_transaction(
                "BoursoBank",
                demo_date(2022, 10, 1),
                TxType::Buy,
                Some("1"),
                Some("130.22"),
                "0",
                None,
                "Achat Air Liquide",
                "demo-v1-ai-2022-10-01",
            ),
        ),
        (
            bourso_account,
            air_liquide,
            demo_transaction(
                "BoursoBank",
                demo_date(2023, 3, 30),
                TxType::Buy,
                Some("1"),
                Some("151.92"),
                "0",
                None,
                "Achat Air Liquide",
                "demo-v1-ai-2023-03-30",
            ),
        ),
        (
            trade_republic_account,
            air_liquide,
            demo_transaction(
                "Trade Republic",
                demo_date(2026, 5, 20),
                TxType::Dividend,
                None,
                None,
                "0",
                Some("4.41"),
                "Dividende Air Liquide",
                "demo-v1-ai-dividend-2026-05-20",
            ),
        ),
        (
            bourso_account,
            totalenergies,
            demo_transaction(
                "BoursoBank",
                demo_date(2021, 12, 31),
                TxType::Buy,
                Some("5"),
                Some("44.72"),
                "0",
                None,
                "Achat TotalEnergies",
                "demo-v1-tte-2021-12-31",
            ),
        ),
        (
            bourso_account,
            totalenergies,
            demo_transaction(
                "BoursoBank",
                demo_date(2025, 4, 1),
                TxType::Dividend,
                None,
                None,
                "0",
                Some("5.53"),
                "Coupon TotalEnergies",
                "demo-v1-tte-dividend-2025-04-01",
            ),
        ),
        (
            trade_republic_account,
            solana,
            demo_transaction(
                "Trade Republic",
                demo_date(2025, 12, 18),
                TxType::Buy,
                Some("0.159405"),
                Some("106.645"),
                "1",
                None,
                "Achat Solana",
                "demo-v1-sol-buy-2025-12-18",
            ),
        ),
        (
            trade_republic_account,
            solana,
            demo_transaction(
                "Trade Republic",
                demo_date(2026, 3, 30),
                TxType::Dividend,
                Some("0.000196"),
                Some("72.83"),
                "0",
                None,
                "FREE_RECEIPT SOL — staking",
                "demo-v1-sol-staking-2026-03-30",
            ),
        ),
        (
            trade_republic_account,
            solana,
            demo_transaction(
                "Trade Republic",
                demo_date(2026, 4, 6),
                TxType::Dividend,
                Some("0.000198"),
                Some("71.28"),
                "0",
                None,
                "FREE_RECEIPT SOL — staking",
                "demo-v1-sol-staking-2026-04-06",
            ),
        ),
        (
            trade_republic_account,
            solana,
            demo_transaction(
                "Trade Republic",
                demo_date(2026, 5, 4),
                TxType::Dividend,
                Some("0.000190"),
                Some("71.55"),
                "0",
                None,
                "FREE_RECEIPT SOL — staking",
                "demo-v1-sol-staking-2026-05-04",
            ),
        ),
        (
            trade_republic_account,
            solana,
            demo_transaction(
                "Trade Republic",
                demo_date(2026, 5, 11),
                TxType::Dividend,
                Some("0.000142"),
                Some("80.88"),
                "0",
                None,
                "FREE_RECEIPT SOL — staking",
                "demo-v1-sol-staking-2026-05-11",
            ),
        ),
        (
            trade_republic_account,
            solana,
            demo_transaction(
                "Trade Republic",
                demo_date(2026, 5, 18),
                TxType::Dividend,
                Some("0.000193"),
                Some("72.65"),
                "0",
                None,
                "FREE_RECEIPT SOL — staking",
                "demo-v1-sol-staking-2026-05-18",
            ),
        ),
        (
            trade_republic_account,
            solana,
            demo_transaction(
                "Trade Republic",
                demo_date(2026, 5, 25),
                TxType::Dividend,
                Some("0.000143"),
                Some("73.57"),
                "0",
                None,
                "FREE_RECEIPT SOL — staking",
                "demo-v1-sol-staking-2026-05-25",
            ),
        ),
        (
            trade_republic_account,
            solana,
            demo_transaction(
                "Trade Republic",
                demo_date(2026, 6, 1),
                TxType::Dividend,
                Some("0.000190"),
                Some("69.53"),
                "0",
                None,
                "FREE_RECEIPT SOL — staking",
                "demo-v1-sol-staking-2026-06-01",
            ),
        ),
    ];

    for (account_id, instrument_id, transaction) in rows {
        insert_transaction(
            conn,
            account_id,
            Some(instrument_id),
            source_file_id,
            &transaction,
        )?;
    }
    upsert_quote(conn, "AI.PA", &demo_decimal("171.28"), "demo")?;
    upsert_quote(conn, "TTE.PA", &demo_decimal("74.44"), "demo")?;
    upsert_quote(conn, "SOL-EUR", &demo_decimal("64.012"), "demo")?;
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, 'seeded')",
        params![DEMO_INITIALIZED_SETTING],
    )?;
    Ok(true)
}

pub fn has_demo_data(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS(
             SELECT 1
             FROM transactions t
             JOIN import_files f ON f.id = t.source_file_id
             WHERE f.file_hash = ?1
         )",
        params![DEMO_SOURCE_HASH],
        |row| row.get(0),
    )
}

/// Supprime exclusivement le seed intégré. Les transactions réelles qui ont
/// pu être importées entre-temps, même sur les mêmes instruments, restent.
pub fn delete_demo_data(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM transactions
         WHERE source_file_id IN (SELECT id FROM import_files WHERE file_hash = ?1)",
        params![DEMO_SOURCE_HASH],
    )?;
    conn.execute(
        "DELETE FROM import_files WHERE file_hash = ?1",
        params![DEMO_SOURCE_HASH],
    )?;
    conn.execute("DELETE FROM quotes WHERE provider = 'demo'", [])?;
    conn.execute(
        "DELETE FROM accounts
         WHERE NOT EXISTS (SELECT 1 FROM transactions WHERE account_id = accounts.id)",
        [],
    )?;
    conn.execute(
        "DELETE FROM instruments
         WHERE NOT EXISTS (SELECT 1 FROM transactions WHERE instrument_id = instruments.id)",
        [],
    )?;
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, 'deleted')
         ON CONFLICT(key) DO UPDATE SET value = 'deleted'",
        params![DEMO_INITIALIZED_SETTING],
    )?;
    Ok(())
}

pub fn get_or_create_account(
    conn: &Connection,
    broker: &str,
    account_type: &str,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO accounts (name, broker, account_type) VALUES (?1, ?2, ?3)
         ON CONFLICT(broker) DO NOTHING",
        params![format!("{account_type} {broker}"), broker, account_type],
    )?;
    conn.query_row(
        "SELECT id FROM accounts WHERE broker = ?1",
        params![broker],
        |r| r.get(0),
    )
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
        if let Ok(id) = conn.query_row(
            "SELECT id FROM instruments WHERE isin = ?1",
            params![isin],
            |r| r.get(0),
        ) {
            if let Some(sym) = symbol {
                conn.execute(
                    "UPDATE instruments SET symbol = ?1 WHERE id = ?2 AND symbol IS NULL",
                    params![sym, id],
                )?;
            }
            return Ok(id);
        }
    }
    if let Some(sym) = symbol {
        if let Ok(id) = conn.query_row(
            "SELECT id FROM instruments WHERE symbol = ?1",
            params![sym],
            |r| r.get(0),
        ) {
            if let Some(isin) = isin {
                conn.execute(
                    "UPDATE instruments SET isin = ?1 WHERE id = ?2 AND isin IS NULL",
                    params![isin, id],
                )?;
            }
            return Ok(id);
        }
        // Un import précédent a pu créer l'instrument par son nom seul.
        if let Ok(id) = conn.query_row(
            "SELECT id FROM instruments WHERE symbol IS NULL AND name = ?1",
            params![name],
            |r| r.get(0),
        ) {
            conn.execute(
                "UPDATE instruments SET symbol = ?1, isin = COALESCE(isin, ?2) WHERE id = ?3",
                params![sym, isin, id],
            )?;
            return Ok(id);
        }
    }
    if symbol.is_none() {
        if let Ok(id) = conn.query_row(
            "SELECT id FROM instruments WHERE name = ?1",
            params![name],
            |r| r.get(0),
        ) {
            if let Some(isin) = isin {
                conn.execute(
                    "UPDATE instruments SET isin = ?1 WHERE id = ?2 AND isin IS NULL",
                    params![isin, id],
                )?;
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
    if let Ok(id) = conn.query_row(
        "SELECT id FROM import_files WHERE file_hash = ?1",
        params![file_hash],
        |r| r.get(0),
    ) {
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

/// Ligne saisie directement dans l'application. Contrairement aux imports,
/// elle n'a pas de fichier source ; cette propriété sert aussi à limiter la
/// modification et la suppression aux seules données appartenant à
/// l'utilisateur.
#[allow(clippy::too_many_arguments)]
pub fn insert_manual_transaction(
    conn: &Connection,
    account_id: i64,
    instrument_id: i64,
    date: NaiveDate,
    tx_type: &str,
    quantity: Option<&Decimal>,
    unit_price: Option<&Decimal>,
    fees: &Decimal,
    amount: Option<&Decimal>,
    description: &str,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO transactions
         (account_id, date, type, instrument_id, quantity, unit_price, fees,
          amount, raw_description, source_file_id, fingerprint)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL,
                 'manual-' || lower(hex(randomblob(16))))",
        params![
            account_id,
            date.to_string(),
            tx_type,
            instrument_id,
            quantity.map(ToString::to_string),
            unit_price.map(ToString::to_string),
            fees.to_string(),
            amount.map(ToString::to_string),
            description,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

#[allow(clippy::too_many_arguments)]
pub fn update_manual_transaction(
    conn: &Connection,
    id: i64,
    account_id: i64,
    instrument_id: i64,
    date: NaiveDate,
    tx_type: &str,
    quantity: Option<&Decimal>,
    unit_price: Option<&Decimal>,
    fees: &Decimal,
    amount: Option<&Decimal>,
    description: &str,
) -> rusqlite::Result<()> {
    let changed = conn.execute(
        "UPDATE transactions
         SET account_id = ?2, instrument_id = ?3, date = ?4, type = ?5,
             quantity = ?6, unit_price = ?7, fees = ?8, amount = ?9,
             raw_description = ?10, edited_at = datetime('now')
         WHERE id = ?1 AND source_file_id IS NULL",
        params![
            id,
            account_id,
            instrument_id,
            date.to_string(),
            tx_type,
            quantity.map(ToString::to_string),
            unit_price.map(ToString::to_string),
            fees.to_string(),
            amount.map(ToString::to_string),
            description,
        ],
    )?;
    if changed == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

/// Refuse explicitement de supprimer une donnée issue d'un import.
pub fn delete_manual_transaction(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    let deleted = conn.execute(
        "DELETE FROM transactions WHERE id = ?1 AND source_file_id IS NULL",
        params![id],
    )?;
    if deleted == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManualTransactionRow {
    pub id: i64,
    pub date: String,
    pub operation: String,
    pub instrument_id: i64,
    pub instrument_name: String,
    pub symbol: Option<String>,
    pub broker: String,
    pub account_type: String,
    pub quantity: Option<String>,
    pub unit_price: Option<String>,
    pub fees: String,
    pub amount: Option<String>,
}

pub fn load_manual_transactions(conn: &Connection) -> rusqlite::Result<Vec<ManualTransactionRow>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.date,
                CASE WHEN t.type = 'DIVIDEND' AND t.quantity IS NOT NULL
                     THEN 'STAKING' ELSE t.type END,
                i.id, i.name, i.symbol, a.broker, a.account_type,
                t.quantity, t.unit_price, t.fees, t.amount
         FROM transactions t
         JOIN instruments i ON i.id = t.instrument_id
         JOIN accounts a ON a.id = t.account_id
         WHERE t.source_file_id IS NULL
         ORDER BY t.date DESC, t.id DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(ManualTransactionRow {
            id: r.get(0)?,
            date: r.get(1)?,
            operation: r.get(2)?,
            instrument_id: r.get(3)?,
            instrument_name: r.get(4)?,
            symbol: r.get(5)?,
            broker: r.get(6)?,
            account_type: r.get(7)?,
            quantity: r.get(8)?,
            unit_price: r.get(9)?,
            fees: r.get(10)?,
            amount: r.get(11)?,
        })
    })?;
    rows.collect()
}

pub fn load_manual_ids(conn: &Connection) -> rusqlite::Result<std::collections::HashSet<i64>> {
    let mut stmt = conn.prepare("SELECT id FROM transactions WHERE source_file_id IS NULL")?;
    let rows = stmt.query_map([], |r| r.get::<_, i64>(0))?;
    Ok(rows.filter_map(Result::ok).collect())
}

pub fn upsert_quote(
    conn: &Connection,
    symbol: &str,
    price: &Decimal,
    provider: &str,
) -> rusqlite::Result<()> {
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
        Ok(InstrumentRow {
            id: r.get(0)?,
            symbol: r.get(1)?,
            name: r.get(2)?,
        })
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

/// Somme des dividendes par instrument. Pour un dividende en nature sans
/// montant espèces (staking), utilise sa valeur de marché à la réception.
pub fn load_dividends(conn: &Connection) -> rusqlite::Result<HashMap<i64, Decimal>> {
    let mut stmt = conn.prepare(
        "SELECT instrument_id, amount, quantity, unit_price
         FROM transactions
         WHERE type = 'DIVIDEND' AND instrument_id IS NOT NULL",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
        ))
    })?;
    let mut sums: HashMap<i64, Decimal> = HashMap::new();
    for row in rows.filter_map(Result::ok) {
        let value =
            dec(row.1).or_else(|| dec(row.2).zip(dec(row.3)).map(|(qty, price)| qty * price));
        if let Some(amount) = value {
            *sums.entry(row.0).or_default() += amount;
        }
    }
    Ok(sums)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account_and_instrument(conn: &Connection) -> (i64, i64) {
        let account = get_or_create_account(conn, "Trade Republic", "CTO").unwrap();
        let instrument = get_or_create_instrument(conn, Some("SOL-EUR"), None, "Solana").unwrap();
        (account, instrument)
    }

    #[test]
    fn staking_migration_reclassifies_existing_free_receipts() {
        let conn = open(Path::new(":memory:")).unwrap();
        let (account, instrument) = account_and_instrument(&conn);
        conn.execute(
            "INSERT INTO transactions
             (account_id, type, instrument_id, quantity, unit_price, raw_description, fingerprint)
             VALUES (?1, 'BUY', ?2, '0.0002', '72.83', 'FREE_RECEIPT SOL', 'staking-old')",
            params![account, instrument],
        )
        .unwrap();

        conn.execute_batch(MIGRATIONS[4]).unwrap();

        let tx_type: String = conn
            .query_row(
                "SELECT type FROM transactions WHERE fingerprint = 'staking-old'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tx_type, "DIVIDEND");
    }

    #[test]
    fn in_kind_dividend_value_uses_quantity_times_reception_price() {
        let conn = open(Path::new(":memory:")).unwrap();
        let (account, instrument) = account_and_instrument(&conn);
        conn.execute(
            "INSERT INTO transactions
             (account_id, type, instrument_id, quantity, unit_price, raw_description, fingerprint)
             VALUES (?1, 'DIVIDEND', ?2, '0.0002', '72.83', 'FREE_RECEIPT SOL', 'staking-new')",
            params![account, instrument],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO transactions
             (account_id, type, instrument_id, amount, raw_description, fingerprint)
             VALUES (?1, 'DIVIDEND', ?2, '1.23', 'Dividend', 'cash-dividend')",
            params![account, instrument],
        )
        .unwrap();

        let dividends = load_dividends(&conn).unwrap();
        assert_eq!(
            dividends[&instrument],
            Decimal::from_str("1.244566").unwrap()
        );
    }

    #[test]
    fn demo_is_seeded_once_then_never_returns_after_deletion() {
        let conn = open(Path::new(":memory:")).unwrap();

        assert!(seed_demo_if_first_run(&conn).unwrap());
        assert!(has_demo_data(&conn).unwrap());
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 14);
        assert!(!seed_demo_if_first_run(&conn).unwrap());

        delete_demo_data(&conn).unwrap();
        assert!(!has_demo_data(&conn).unwrap());
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
        assert!(!seed_demo_if_first_run(&conn).unwrap());
    }

    #[test]
    fn demo_is_not_added_to_an_existing_portfolio() {
        let conn = open(Path::new(":memory:")).unwrap();
        let (account, instrument) = account_and_instrument(&conn);
        conn.execute(
            "INSERT INTO transactions
             (account_id, date, type, instrument_id, quantity, unit_price, raw_description, fingerprint)
             VALUES (?1, '2026-07-07', 'BUY', ?2, '0.25', '70', 'Real buy', 'real-buy')",
            params![account, instrument],
        )
        .unwrap();

        assert!(!seed_demo_if_first_run(&conn).unwrap());
        assert!(!has_demo_data(&conn).unwrap());
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn deleting_demo_preserves_real_rows_on_the_same_instrument() {
        let conn = open(Path::new(":memory:")).unwrap();
        seed_demo_if_first_run(&conn).unwrap();
        let (account, instrument) = account_and_instrument(&conn);
        conn.execute(
            "INSERT INTO transactions
             (account_id, date, type, instrument_id, quantity, unit_price, raw_description, fingerprint)
             VALUES (?1, '2026-07-07', 'BUY', ?2, '0.25', '70', 'Real buy', 'real-buy')",
            params![account, instrument],
        )
        .unwrap();

        delete_demo_data(&conn).unwrap();

        let remaining: Vec<String> = conn
            .prepare("SELECT fingerprint FROM transactions ORDER BY fingerprint")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(remaining, vec!["real-buy"]);
        assert!(!has_demo_data(&conn).unwrap());
    }

    #[test]
    fn manual_transactions_are_editable_and_deletable_but_imports_are_protected() {
        let conn = open(Path::new(":memory:")).unwrap();
        let (account, instrument) = account_and_instrument(&conn);
        let date = demo_date(2026, 8, 11);
        let quantity = demo_decimal("0.25");
        let price = demo_decimal("70");
        let fees = demo_decimal("1");

        let id = insert_manual_transaction(
            &conn,
            account,
            instrument,
            date,
            "BUY",
            Some(&quantity),
            Some(&price),
            &fees,
            None,
            "[MANUEL] Achat manuel",
        )
        .unwrap();
        let rows = load_manual_transactions(&conn).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].operation, "BUY");
        assert!(load_manual_ids(&conn).unwrap().contains(&id));

        let staking_quantity = demo_decimal("0.0002");
        let staking_price = demo_decimal("72.83");
        update_manual_transaction(
            &conn,
            id,
            account,
            instrument,
            date,
            "DIVIDEND",
            Some(&staking_quantity),
            Some(&staking_price),
            &Decimal::ZERO,
            None,
            "[MANUEL] Staking manuel",
        )
        .unwrap();
        let rows = load_manual_transactions(&conn).unwrap();
        assert_eq!(rows[0].operation, "STAKING");
        assert_eq!(rows[0].quantity.as_deref(), Some("0.0002"));

        delete_manual_transaction(&conn, id).unwrap();
        assert!(load_manual_transactions(&conn).unwrap().is_empty());

        let (source_id, _) = record_import_file(
            &conn,
            "import.csv",
            "protected-import",
            "Trade Republic",
            "source",
        )
        .unwrap();
        let imported = demo_transaction(
            "Trade Republic",
            date,
            TxType::Buy,
            Some("1"),
            Some("50"),
            "0",
            None,
            "Achat importé",
            "protected-row",
        );
        insert_transaction(&conn, account, Some(instrument), source_id, &imported).unwrap();
        let imported_id: i64 = conn
            .query_row(
                "SELECT id FROM transactions WHERE fingerprint = 'protected-row'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(delete_manual_transaction(&conn, imported_id).is_err());
    }
}
