//! Fournisseur de cotations. L'implémentation actuelle interroge l'API
//! non officielle de Yahoo Finance ; le reste de l'application ne dépend que
//! de `fetch_quotes`, ce qui permet de changer de fournisseur sans toucher
//! au domaine.

use rust_decimal::Decimal;
use std::time::Duration;

pub struct QuoteResult {
    pub symbol: String,
    pub result: Result<Decimal, String>,
}

pub async fn fetch_quotes(symbols: &[String]) -> Vec<QuoteResult> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return symbols
                .iter()
                .map(|s| QuoteResult {
                    symbol: s.clone(),
                    result: Err(e.to_string()),
                })
                .collect()
        }
    };

    let mut results = Vec::new();
    for symbol in symbols {
        results.push(QuoteResult {
            symbol: symbol.clone(),
            result: fetch_one(&client, symbol).await,
        });
    }
    results
}

async fn fetch_one(client: &reqwest::Client, symbol: &str) -> Result<Decimal, String> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?range=1d&interval=1d&includePrePost=false"
    );
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let price = body["chart"]["result"][0]["meta"]["regularMarketPrice"]
        .as_f64()
        .ok_or_else(|| "cours absent de la réponse".to_string())?;
    // Conversion immédiate en décimal, arrondie à 4 chiffres : les montants
    // ne transitent jamais en flottant au-delà de ce point d'entrée.
    Decimal::from_f64_retain(price)
        .map(|d| d.round_dp(4))
        .ok_or_else(|| "cours invalide".to_string())
}
