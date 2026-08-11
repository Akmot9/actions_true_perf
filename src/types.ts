// Miroir des structures sérialisées par les commandes Rust.
// Les montants sont des chaînes (rust_decimal) à convertir via Number().

export interface QuoteRow {
  price: string;
  provider: string;
  fetched_at: string;
}

export interface LotView {
  id: number;
  tx_id: number | null;
  edited: boolean;
  manual: boolean;
  acquisition_date: string | null;
  origin_broker: string;
  account: string;
  initial_quantity: string;
  remaining_quantity: string;
  unit_cost: string;
  fees: string;
  invested: string;
  market_value: string | null;
  pnl: string | null;
  pnl_pct: string | null;
  unreconciled: boolean;
  income_events: number;
  /** Versements individuels quand la ligne regroupe du staking ; vide sinon. */
  children: LotView[];
}

export type ManualOperation = "BUY" | "SELL" | "DIVIDEND" | "STAKING";

export interface ManualTransaction {
  id: number;
  date: string;
  operation: ManualOperation;
  instrument_id: number;
  instrument_name: string;
  symbol: string | null;
  broker: string;
  account_type: string;
  quantity: string | null;
  unit_price: string | null;
  fees: string;
  amount: string | null;
}

export interface ManualTransactionInput {
  operation: ManualOperation;
  date: string;
  instrument_name: string;
  symbol: string | null;
  broker: string;
  account_type: string;
  quantity: string | null;
  unit_price: string | null;
  fees: string;
  amount: string | null;
}

export interface PositionView {
  instrument_id: number;
  symbol: string | null;
  name: string;
  quantity: string;
  invested: string;
  avg_cost: string | null;
  quote: QuoteRow | null;
  market_value: string | null;
  pnl: string | null;
  pnl_pct: string | null;
  dividends: string;
  realized_pnl: string;
  unreconciled_quantity: string;
  lots: LotView[];
}

export interface PortfolioView {
  has_demo_data: boolean;
  positions: PositionView[];
  total_invested: string;
  total_market_value: string;
  total_pnl: string;
  total_dividends: string;
  total_realized_pnl: string;
  warnings: string[];
}

export interface ImportReport {
  file_name: string;
  broker: string;
  file_already_imported: boolean;
  total: number;
  inserted: number;
  duplicates: number;
  by_type: [string, number][];
  warnings: string[];
}

export interface QuoteRefresh {
  symbol: string;
  price: string | null;
  error: string | null;
}
