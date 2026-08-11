use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::{EngineTx, TxType};

/// Lot d'acquisition : un ordre d'achat = un lot, qui conserve son identité
/// à travers ventes partielles, transferts de courtier et splits.
#[derive(Debug, Clone, Serialize)]
pub struct Lot {
    /// Identifiant stable au sein d'un replay (ordre de création).
    pub id: usize,
    pub buy_tx_id: Option<i64>,
    pub instrument_id: i64,
    /// Compte détenant actuellement le lot (change lors d'un transfert).
    pub account_id: i64,
    /// Courtier chez qui l'ordre d'origine a été passé.
    pub origin_broker: String,
    pub acquisition_date: Option<NaiveDate>,
    pub initial_quantity: Decimal,
    pub remaining_quantity: Decimal,
    /// Prix unitaire d'acquisition (hors frais).
    pub unit_cost: Decimal,
    /// Frais payés sur l'ordre entier, allouables au prorata des quantités.
    pub fees: Decimal,
    /// Lot créé pour couvrir un transfert entrant sans historique d'achat.
    /// Résorbé automatiquement au prochain replay si l'historique est importé.
    pub unreconciled: bool,
    /// Nombre de versements accumulés quand ce lot agrège des dividendes en
    /// nature (récompenses de staking…). 0 pour un lot d'achat normal.
    pub income_events: u32,
}

impl Lot {
    /// Coût de revient (frais inclus) de la quantité restante.
    pub fn invested_remaining(&self) -> Decimal {
        self.remaining_quantity * self.unit_cost + self.fees_for(self.remaining_quantity)
    }

    fn fees_for(&self, qty: Decimal) -> Decimal {
        if self.initial_quantity.is_zero() {
            Decimal::ZERO
        } else {
            self.fees * qty / self.initial_quantity
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Disposal {
    pub sell_tx_id: i64,
    pub lot_id: usize,
    pub quantity: Decimal,
    pub cost_basis: Decimal,
    pub proceeds: Decimal,
    pub realized_pnl: Decimal,
}

#[derive(Debug, Default, Serialize)]
pub struct ReplayOutput {
    pub lots: Vec<Lot>,
    pub disposals: Vec<Disposal>,
    pub warnings: Vec<String>,
}

/// Priorité d'exécution pour les opérations d'une même journée, afin que le
/// replay soit déterministe quel que soit l'ordre d'import des fichiers.
fn type_priority(t: TxType) -> u8 {
    match t {
        TxType::Buy => 0,
        // Un dividende en nature (récompense de staking) reçu le même jour
        // qu'une vente ou un transfert doit exister avant que ceux-ci ne
        // consomment la position.
        TxType::Dividend => 1,
        TxType::Split => 2,
        TxType::TransferOut => 3,
        TxType::TransferIn => 4,
        TxType::Sell => 5,
        _ => 6,
    }
}

/// Rejoue l'intégralité des transactions et reconstruit l'état des lots.
/// Les transactions sont la source de vérité ; cette fonction est pure et
/// déterministe (mêmes transactions => mêmes lots).
pub fn replay(txs: &[EngineTx]) -> ReplayOutput {
    let mut txs: Vec<&EngineTx> = txs.iter().collect();
    // Les dates inconnues (ex: achats anciens sans date) passent en premier :
    // elles sont forcément antérieures à l'historique daté.
    txs.sort_by_key(|t| {
        (
            t.date.unwrap_or(NaiveDate::MIN),
            type_priority(t.tx_type),
            t.id,
        )
    });

    let mut out = ReplayOutput::default();

    for tx in txs {
        match tx.tx_type {
            TxType::Buy => apply_buy(&mut out, tx),
            TxType::Sell => apply_sell(&mut out, tx),
            TxType::TransferIn => apply_transfer_in(&mut out, tx),
            TxType::Split => apply_split(&mut out, tx),
            TxType::Dividend => apply_dividend(&mut out, tx),
            // TRANSFER_OUT est administratif : c'est le TRANSFER_IN apparié
            // qui déplace les lots. Espèces/frais n'affectent pas les lots.
            _ => {}
        }
    }

    out
}

fn apply_buy(out: &mut ReplayOutput, tx: &EngineTx) {
    let (Some(instrument_id), Some(qty), Some(price)) =
        (tx.instrument_id, tx.quantity, tx.unit_price)
    else {
        out.warnings.push(format!(
            "BUY tx#{} incomplet (instrument/quantité/prix manquant), ignoré",
            tx.id
        ));
        return;
    };
    let id = out.lots.len();
    out.lots.push(Lot {
        id,
        buy_tx_id: Some(tx.id),
        instrument_id,
        account_id: tx.account_id,
        origin_broker: tx.broker.clone(),
        acquisition_date: tx.date,
        initial_quantity: qty,
        remaining_quantity: qty,
        unit_cost: price,
        fees: tx.fees,
        unreconciled: false,
        income_events: 0,
    });
}

/// Un dividende en numéraire n'affecte pas les lots. Un dividende **en
/// nature** (quantité + prix de réception : récompenses de staking…) crée un
/// lot daté du versement, exactement comme un achat : l'ordre FIFO, la base
/// de coût (frais inclus) et l'édition transaction par transaction restent
/// exacts. Le regroupement des versements est un choix d'affichage, fait en
/// aval — jamais ici.
fn apply_dividend(out: &mut ReplayOutput, tx: &EngineTx) {
    let (Some(qty), Some(price)) = (tx.quantity, tx.unit_price) else {
        return; // dividende en numéraire
    };
    if qty.is_zero() {
        return;
    }
    let Some(instrument_id) = tx.instrument_id else {
        out.warnings.push(format!(
            "Dividende en nature tx#{} sans instrument résolu : {qty} titres ignorés",
            tx.id
        ));
        return;
    };
    let id = out.lots.len();
    out.lots.push(Lot {
        id,
        buy_tx_id: Some(tx.id),
        instrument_id,
        account_id: tx.account_id,
        origin_broker: tx.broker.clone(),
        acquisition_date: tx.date,
        initial_quantity: qty,
        remaining_quantity: qty,
        unit_cost: price,
        fees: tx.fees,
        unreconciled: false,
        income_events: 1,
    });
}

/// Indices des lots ouverts pour un instrument, en ordre FIFO
/// (dates inconnues d'abord, puis date d'acquisition, puis ordre de création).
fn open_lots_fifo(lots: &[Lot], instrument_id: i64, account_id: Option<i64>) -> Vec<usize> {
    let mut idx: Vec<usize> = lots
        .iter()
        .enumerate()
        .filter(|(_, l)| {
            l.instrument_id == instrument_id
                && !l.remaining_quantity.is_zero()
                && account_id.is_none_or(|a| l.account_id == a)
        })
        .map(|(i, _)| i)
        .collect();
    idx.sort_by_key(|&i| {
        (
            lots[i].acquisition_date.unwrap_or(NaiveDate::MIN),
            lots[i].id,
        )
    });
    idx
}

fn apply_sell(out: &mut ReplayOutput, tx: &EngineTx) {
    let (Some(instrument_id), Some(qty)) = (tx.instrument_id, tx.quantity) else {
        out.warnings
            .push(format!("SELL tx#{} incomplet, ignoré", tx.id));
        return;
    };
    let price = tx.unit_price.unwrap_or(Decimal::ZERO);
    let mut to_sell = qty;

    for i in open_lots_fifo(&out.lots, instrument_id, Some(tx.account_id)) {
        if to_sell.is_zero() {
            break;
        }
        let lot = &mut out.lots[i];
        let take = to_sell.min(lot.remaining_quantity);
        let cost_basis = take * lot.unit_cost + lot.fees_for(take);
        let proceeds = take * price
            - if qty.is_zero() {
                Decimal::ZERO
            } else {
                tx.fees * take / qty
            };
        lot.remaining_quantity -= take;
        to_sell -= take;
        out.disposals.push(Disposal {
            sell_tx_id: tx.id,
            lot_id: lot.id,
            quantity: take,
            cost_basis,
            proceeds,
            realized_pnl: proceeds - cost_basis,
        });
    }

    if !to_sell.is_zero() {
        out.warnings.push(format!(
            "Vente tx#{} : {} titres vendus sans lot disponible (instrument #{})",
            tx.id, to_sell, instrument_id
        ));
    }
}

/// Un transfert entrant déplace les lots historiques existants (autres comptes)
/// vers le compte cible, sans toucher date/coût/performance. Si l'historique ne
/// couvre pas la quantité transférée, le reliquat devient un lot `unreconciled`
/// au PRU communiqué par le courtier — remplacé dès que l'historique manquant
/// est importé (nouveau replay).
fn apply_transfer_in(out: &mut ReplayOutput, tx: &EngineTx) {
    let (Some(instrument_id), Some(qty)) = (tx.instrument_id, tx.quantity) else {
        out.warnings
            .push(format!("TRANSFER_IN tx#{} incomplet, ignoré", tx.id));
        return;
    };
    let mut to_move = qty;

    for i in open_lots_fifo(&out.lots, instrument_id, None) {
        if to_move.is_zero() {
            break;
        }
        let lot = &mut out.lots[i];
        if lot.account_id == tx.account_id {
            continue;
        }
        if lot.remaining_quantity <= to_move {
            // Le lot entier change de compte, identité préservée.
            to_move -= lot.remaining_quantity;
            lot.account_id = tx.account_id;
        } else {
            // Transfert partiel : le lot est scindé, chaque moitié garde
            // date, coût unitaire et frais au prorata.
            let moved = to_move;
            let fees_moved = lot.fees_for(moved);
            lot.remaining_quantity -= moved;
            lot.initial_quantity -= moved;
            lot.fees -= fees_moved;
            let (buy_tx_id, date, unit_cost, broker, unreconciled, income_events) = (
                lot.buy_tx_id,
                lot.acquisition_date,
                lot.unit_cost,
                lot.origin_broker.clone(),
                lot.unreconciled,
                lot.income_events,
            );
            let id = out.lots.len();
            out.lots.push(Lot {
                id,
                buy_tx_id,
                instrument_id,
                account_id: tx.account_id,
                origin_broker: broker,
                acquisition_date: date,
                initial_quantity: moved,
                remaining_quantity: moved,
                unit_cost,
                fees: fees_moved,
                unreconciled,
                income_events,
            });
            to_move = Decimal::ZERO;
        }
    }

    if !to_move.is_zero() {
        out.warnings.push(format!(
            "Transfert tx#{} : {} titres reçus sans historique d'achat (instrument #{}) — lot au PRU courtier créé",
            tx.id, to_move, instrument_id
        ));
        let id = out.lots.len();
        out.lots.push(Lot {
            id,
            buy_tx_id: Some(tx.id),
            instrument_id,
            account_id: tx.account_id,
            origin_broker: "inconnu".to_string(),
            acquisition_date: tx.date,
            initial_quantity: to_move,
            remaining_quantity: to_move,
            unit_cost: tx.unit_price.unwrap_or(Decimal::ZERO),
            fees: Decimal::ZERO,
            unreconciled: true,
            income_events: 0,
        });
    }
}

/// Split `r:1` : la quantité du champ `quantity` porte le ratio.
/// Quantités multipliées, coût unitaire divisé, coût total du lot invariant.
fn apply_split(out: &mut ReplayOutput, tx: &EngineTx) {
    let (Some(instrument_id), Some(ratio)) = (tx.instrument_id, tx.quantity) else {
        out.warnings
            .push(format!("SPLIT tx#{} sans ratio, ignoré", tx.id));
        return;
    };
    if ratio.is_zero() {
        out.warnings
            .push(format!("SPLIT tx#{} avec ratio nul, ignoré", tx.id));
        return;
    }
    for lot in out
        .lots
        .iter_mut()
        .filter(|l| l.instrument_id == instrument_id)
    {
        lot.initial_quantity *= ratio;
        lot.remaining_quantity *= ratio;
        lot.unit_cost /= ratio;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn tx(id: i64, account: i64, date: &str, t: TxType, qty: &str, price: &str) -> EngineTx {
        EngineTx {
            id,
            account_id: account,
            broker: if account == 1 {
                "BoursoBank".into()
            } else {
                "Bourse Direct".into()
            },
            date: Some(d(date)),
            tx_type: t,
            instrument_id: Some(10),
            quantity: Some(qty.parse().unwrap()),
            unit_price: Some(price.parse().unwrap()),
            fees: Decimal::ZERO,
        }
    }

    #[test]
    fn multiple_buys_create_distinct_lots() {
        let out = replay(&[
            tx(1, 1, "2024-01-10", TxType::Buy, "10", "20"),
            tx(2, 1, "2024-02-10", TxType::Buy, "10", "30"),
        ]);
        assert_eq!(out.lots.len(), 2);
        assert_eq!(out.lots[0].unit_cost, dec!(20));
        assert_eq!(out.lots[1].unit_cost, dec!(30));
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn partial_sell_consumes_fifo() {
        let out = replay(&[
            tx(1, 1, "2024-01-10", TxType::Buy, "10", "20"),
            tx(2, 1, "2024-02-10", TxType::Buy, "10", "30"),
            tx(3, 1, "2024-03-10", TxType::Sell, "15", "40"),
        ]);
        assert_eq!(out.lots[0].remaining_quantity, dec!(0));
        assert_eq!(out.lots[1].remaining_quantity, dec!(5));
        assert_eq!(out.disposals.len(), 2);
        // Lot 1 : 10 vendues, PnL = 10*(40-20) = 200
        assert_eq!(out.disposals[0].realized_pnl, dec!(200));
        // Lot 2 : 5 vendues, PnL = 5*(40-30) = 50
        assert_eq!(out.disposals[1].realized_pnl, dec!(50));
    }

    #[test]
    fn transfer_preserves_lot_identity() {
        let out = replay(&[
            tx(1, 1, "2024-01-10", TxType::Buy, "10", "20"),
            tx(2, 2, "2025-03-04", TxType::TransferIn, "10", "22"),
        ]);
        assert_eq!(out.lots.len(), 1);
        let lot = &out.lots[0];
        assert_eq!(lot.account_id, 2); // compte actuel = Bourse Direct
        assert_eq!(lot.acquisition_date, Some(d("2024-01-10"))); // date inchangée
        assert_eq!(lot.unit_cost, dec!(20)); // coût inchangé (pas le PRU courtier)
        assert_eq!(lot.origin_broker, "BoursoBank");
        assert!(!lot.unreconciled);
    }

    #[test]
    fn transfer_without_history_creates_unreconciled_lot() {
        // 22 Nexity connues, 141 transférées => 119 au PRU courtier.
        let out = replay(&[
            tx(1, 1, "2023-06-28", TxType::Buy, "22", "18.39"),
            tx(2, 2, "2025-03-04", TxType::TransferIn, "141", "12.25773"),
        ]);
        assert_eq!(out.lots.len(), 2);
        assert_eq!(out.lots[0].remaining_quantity, dec!(22));
        assert_eq!(out.lots[0].account_id, 2);
        assert!(!out.lots[0].unreconciled);
        assert_eq!(out.lots[1].remaining_quantity, dec!(119));
        assert!(out.lots[1].unreconciled);
        assert_eq!(out.lots[1].unit_cost, dec!(12.25773));
        assert_eq!(out.warnings.len(), 1);
    }

    #[test]
    fn late_history_import_resolves_unreconciled_on_next_replay() {
        // Même scénario, mais l'historique complet arrive plus tard (id plus
        // grand) : le tri par date le replace avant le transfert et le lot
        // unreconciled disparaît.
        let out = replay(&[
            tx(5, 2, "2025-03-04", TxType::TransferIn, "141", "12.25773"),
            tx(6, 1, "2023-06-28", TxType::Buy, "22", "18.39"),
            tx(7, 1, "2022-09-05", TxType::Buy, "119", "22.22"),
        ]);
        assert_eq!(out.lots.len(), 2);
        assert!(out.lots.iter().all(|l| !l.unreconciled));
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn split_preserves_total_cost() {
        let out = replay(&[
            tx(1, 1, "2024-01-10", TxType::Buy, "10", "100"),
            tx(2, 1, "2024-06-10", TxType::Split, "2", "0"),
        ]);
        let lot = &out.lots[0];
        assert_eq!(lot.remaining_quantity, dec!(20));
        assert_eq!(lot.unit_cost, dec!(50));
        assert_eq!(lot.invested_remaining(), dec!(1000));
    }

    #[test]
    fn oversell_is_reported_not_silently_fixed() {
        let out = replay(&[
            tx(1, 1, "2024-01-10", TxType::Buy, "10", "20"),
            tx(2, 1, "2024-02-10", TxType::Sell, "12", "25"),
        ]);
        assert_eq!(out.lots[0].remaining_quantity, dec!(0));
        assert_eq!(out.warnings.len(), 1);
        assert!(out.warnings[0].contains("sans lot disponible"));
    }

    #[test]
    fn fees_allocated_pro_rata_on_partial_sell() {
        let mut buy = tx(1, 1, "2024-01-10", TxType::Buy, "10", "20");
        buy.fees = dec!(2);
        let out = replay(&[buy, tx(2, 1, "2024-02-10", TxType::Sell, "5", "30")]);
        // Coût de la portion vendue : 5*20 + 2*5/10 = 101
        assert_eq!(out.disposals[0].cost_basis, dec!(101));
        // Coût de la portion restante : 5*20 + 1 = 101
        assert_eq!(out.lots[0].invested_remaining(), dec!(101));
    }

    #[test]
    fn in_kind_dividends_create_dated_editable_lots() {
        let out = replay(&[
            tx(1, 1, "2024-01-10", TxType::Dividend, "2", "10"),
            tx(2, 1, "2024-02-10", TxType::Dividend, "3", "20"),
        ]);

        // Un lot par versement : dates exactes pour le FIFO, transaction
        // sous-jacente conservée pour l'édition.
        assert_eq!(out.lots.len(), 2);
        assert_eq!(out.lots[0].acquisition_date, Some(d("2024-01-10")));
        assert_eq!(out.lots[0].unit_cost, dec!(10));
        assert_eq!(out.lots[0].buy_tx_id, Some(1));
        assert_eq!(out.lots[1].unit_cost, dec!(20));
        assert!(out.lots.iter().all(|l| l.income_events == 1));
    }

    #[test]
    fn in_kind_dividend_between_buys_keeps_fifo_order() {
        // Récompense en janvier, achat en mars, récompense en avril : le FIFO
        // doit consommer janvier puis mars puis avril — la récompense d'avril
        // ne doit pas être antidatée en janvier.
        let out = replay(&[
            tx(1, 1, "2024-01-10", TxType::Dividend, "1", "20"),
            tx(2, 1, "2024-03-10", TxType::Buy, "1", "50"),
            tx(3, 1, "2024-04-10", TxType::Dividend, "1", "80"),
            tx(4, 1, "2024-06-10", TxType::Sell, "2", "100"),
        ]);
        assert_eq!(out.disposals.len(), 2);
        assert_eq!(out.disposals[0].cost_basis, dec!(20)); // récompense de janvier
        assert_eq!(out.disposals[1].cost_basis, dec!(50)); // achat de mars
        let open: Vec<_> = out.lots.iter().filter(|l| !l.remaining_quantity.is_zero()).collect();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].unit_cost, dec!(80)); // la récompense d'avril reste
    }

    #[test]
    fn same_day_reward_is_available_to_the_sell() {
        // La récompense du jour doit être rejouée avant la vente du même
        // jour : pas de survente fantôme, pas de lot résiduel.
        let out = replay(&[
            tx(1, 1, "2024-01-10", TxType::Buy, "1", "50"),
            tx(3, 1, "2024-02-10", TxType::Sell, "1.01", "60"),
            tx(2, 1, "2024-02-10", TxType::Dividend, "0.01", "55"),
        ]);
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
        assert!(out.lots.iter().all(|l| l.remaining_quantity.is_zero()));
    }

    #[test]
    fn in_kind_dividend_fees_count_in_cost_basis() {
        let mut reward = tx(1, 1, "2024-01-10", TxType::Dividend, "2", "10");
        reward.fees = dec!(0.50);
        let out = replay(&[reward]);
        assert_eq!(out.lots[0].invested_remaining(), dec!(20.50));
    }

    #[test]
    fn in_kind_dividend_without_instrument_warns() {
        let mut reward = tx(1, 1, "2024-01-10", TxType::Dividend, "2", "10");
        reward.instrument_id = None;
        let out = replay(&[reward]);
        assert!(out.lots.is_empty());
        assert_eq!(out.warnings.len(), 1);
        assert!(out.warnings[0].contains("sans instrument résolu"));
    }

    #[test]
    fn cash_dividend_does_not_create_a_lot() {
        let mut dividend = tx(1, 1, "2024-01-10", TxType::Dividend, "1", "10");
        dividend.quantity = None;
        dividend.unit_price = None;

        let out = replay(&[dividend]);

        assert!(out.lots.is_empty());
    }

    #[test]
    fn staking_cost_stays_correct_after_a_sale_between_rewards() {
        let out = replay(&[
            tx(1, 1, "2024-01-10", TxType::Dividend, "2", "10"),
            tx(2, 1, "2024-01-20", TxType::Sell, "1", "30"),
            tx(3, 1, "2024-02-10", TxType::Dividend, "3", "20"),
        ]);

        // La vente a consommé 1 titre du versement de janvier (coût 10) ;
        // celui de février reste intact avec son propre coût.
        assert_eq!(out.disposals[0].cost_basis, dec!(10));
        assert_eq!(out.lots[0].remaining_quantity, dec!(1));
        assert_eq!(out.lots[1].remaining_quantity, dec!(3));
        let invested: Decimal = out.lots.iter().map(|l| l.invested_remaining()).sum();
        assert_eq!(invested, dec!(70)); // 1×10 + 3×20
    }
}
