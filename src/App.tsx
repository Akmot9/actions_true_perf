import { createSignal, createResource, For, Show, onCleanup, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import type {
  ImportReport,
  LotView,
  ManualOperation,
  ManualTransaction,
  ManualTransactionInput,
  PortfolioView,
  PositionView,
  QuoteRefresh,
} from "./types";
import { date, eur, holdingSince, num, pct, perfClass } from "./format";
import "./App.css";

const operationLabels: Record<ManualOperation, string> = {
  BUY: "Achat",
  SELL: "Vente",
  DIVIDEND: "Dividende",
  STAKING: "Staking",
};

function localToday(): string {
  const now = new Date();
  const offset = now.getTimezoneOffset() * 60_000;
  return new Date(now.getTime() - offset).toISOString().slice(0, 10);
}

/** Intensité de la case du strip d'ordres selon la performance (%). */
function heatClass(lot: LotView): string {
  if (lot.pnl_pct === null) return "na";
  const n = Number(lot.pnl_pct);
  const side = n >= 0 ? "gain" : "loss";
  const mag = Math.abs(n) >= 25 ? "3" : Math.abs(n) >= 10 ? "2" : "1";
  return `${side}-${mag}`;
}

function DivergingBar(props: { pct: string | null }) {
  const value = () => (props.pct === null ? 0 : Number(props.pct));
  const width = () => Math.min(Math.abs(value()), 60) / 0.6; // 60 % de perf = barre pleine
  return (
    <div class="divbar" aria-hidden="true">
      <div class="divbar-half">
        <Show when={value() < 0}>
          <div class="divbar-fill loss" style={{ width: `${width()}%` }} />
        </Show>
      </div>
      <div class="divbar-half">
        <Show when={value() > 0}>
          <div class="divbar-fill gain" style={{ width: `${width()}%` }} />
        </Show>
      </div>
    </div>
  );
}

function ManualTransactionDialog(props: {
  transaction: ManualTransaction | null;
  instruments: PositionView[];
  onClose: (saved: boolean) => void;
}) {
  const existing = props.transaction;
  const [operation, setOperation] = createSignal<ManualOperation>(existing?.operation ?? "BUY");
  const [date_, setDate] = createSignal(existing?.date ?? localToday());
  const [instrumentName, setInstrumentName] = createSignal(existing?.instrument_name ?? "");
  const [symbol, setSymbol] = createSignal(existing?.symbol ?? "");
  const [broker, setBroker] = createSignal(existing?.broker ?? "Trade Republic");
  const [accountType, setAccountType] = createSignal(existing?.account_type ?? "CTO");
  const [quantity, setQuantity] = createSignal(existing?.quantity ?? "");
  const [unitPrice, setUnitPrice] = createSignal(existing?.unit_price ?? "");
  const [fees, setFees] = createSignal(existing?.fees ?? "0");
  const [amount, setAmount] = createSignal(existing?.amount ?? "");
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);

  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") props.onClose(false);
  };
  onMount(() => document.addEventListener("keydown", onKey));
  onCleanup(() => document.removeEventListener("keydown", onKey));

  // Symbole issu de l'auto-complétion, à distinguer d'une saisie de
  // l'utilisateur : si le nom ne correspond plus, il doit être effacé pour ne
  // pas rattacher silencieusement l'opération au mauvais instrument.
  const [autoSymbol, setAutoSymbol] = createSignal<string | null>(null);

  function onInstrumentInput(value: string) {
    setInstrumentName(value);
    const match = props.instruments.find(
      (instrument) => instrument.name.toLocaleLowerCase() === value.trim().toLocaleLowerCase(),
    );
    if (match?.symbol) {
      setSymbol(match.symbol);
      setAutoSymbol(match.symbol);
    } else if (autoSymbol() !== null && symbol() === autoSymbol()) {
      setSymbol("");
      setAutoSymbol(null);
    }
  }

  async function save(e: Event) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    const input: ManualTransactionInput = {
      operation: operation(),
      date: date_(),
      instrument_name: instrumentName(),
      symbol: symbol().trim() || null,
      broker: broker(),
      account_type: accountType(),
      quantity: operation() === "DIVIDEND" ? null : quantity(),
      unit_price: operation() === "DIVIDEND" ? null : unitPrice(),
      fees: operation() === "BUY" || operation() === "SELL" ? fees() : "0",
      amount: operation() === "DIVIDEND" ? amount() : null,
    };
    try {
      if (existing) {
        await invoke("update_manual_transaction", { id: existing.id, input });
      } else {
        await invoke("add_manual_transaction", { input });
      }
      props.onClose(true);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  // Confirmation en deux clics dans la page : window.confirm n'est pas
  // implémenté par le WebView (WKWebView sur iOS/macOS) et rendrait la
  // suppression impossible.
  const [confirmDelete, setConfirmDelete] = createSignal(false);

  async function remove() {
    if (!existing) return;
    if (!confirmDelete()) {
      setConfirmDelete(true);
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await invoke("delete_manual_transaction", { id: existing.id });
      props.onClose(true);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div class="overlay" onClick={() => props.onClose(false)}>
      <form
        class="dialog manual-dialog"
        role="dialog"
        aria-label={existing ? "Modifier une opération manuelle" : "Ajouter une opération"}
        onClick={(e) => e.stopPropagation()}
        onSubmit={save}
      >
        <header>
          <h2>{existing ? "Modifier la ligne" : "Ajouter une opération"}</h2>
          <p class="muted">Saisie manuelle, modifiable ou supprimable à tout moment.</p>
        </header>

        <div class="field-row two">
          <div class="field">
            <label for="manual-operation">Opération</label>
            <select
              id="manual-operation"
              value={operation()}
              onChange={(e) => setOperation(e.currentTarget.value as ManualOperation)}
            >
              <For each={Object.entries(operationLabels)}>
                {([value, label]) => <option value={value}>{label}</option>}
              </For>
            </select>
          </div>
          <div class="field">
            <label for="manual-date">Date</label>
            <input
              id="manual-date"
              type="date"
              required
              value={date_()}
              onInput={(e) => setDate(e.currentTarget.value)}
            />
          </div>
        </div>

        <div class="field-row two">
          <div class="field">
            <label for="manual-instrument">Instrument</label>
            <input
              id="manual-instrument"
              list="known-instruments"
              autocomplete="off"
              required
              placeholder="Ex. Air Liquide"
              value={instrumentName()}
              onInput={(e) => onInstrumentInput(e.currentTarget.value)}
            />
            <datalist id="known-instruments">
              <For each={props.instruments}>
                {(instrument) => <option value={instrument.name}>{instrument.symbol ?? ""}</option>}
              </For>
            </datalist>
          </div>
          <div class="field">
            <label for="manual-symbol">Symbole de marché</label>
            <input
              id="manual-symbol"
              autocomplete="off"
              placeholder="Ex. AI.PA ou SOL-EUR"
              value={symbol()}
              onInput={(e) => setSymbol(e.currentTarget.value)}
            />
          </div>
        </div>

        <div class="field-row two">
          <div class="field">
            <label for="manual-broker">Courtier / compte</label>
            <input
              id="manual-broker"
              required
              placeholder="Ex. Trade Republic"
              value={broker()}
              onInput={(e) => setBroker(e.currentTarget.value)}
            />
          </div>
          <div class="field">
            <label for="manual-account">Type de compte</label>
            <select
              id="manual-account"
              value={accountType()}
              onChange={(e) => setAccountType(e.currentTarget.value)}
            >
              <option value="CTO">CTO</option>
              <option value="PEA">PEA</option>
              <option value="CRYPTO">Crypto</option>
              <option value="ASSURANCE_VIE">Assurance-vie</option>
            </select>
          </div>
        </div>

        <Show
          when={operation() === "DIVIDEND"}
          fallback={
            <div class="field-row" classList={{ two: operation() === "STAKING" }}>
              <div class="field">
                <label for="manual-quantity">
                  {operation() === "STAKING" ? "Quantité reçue" : "Quantité"}
                </label>
                <input
                  id="manual-quantity"
                  class="num"
                  inputmode="decimal"
                  required
                  value={quantity()}
                  onInput={(e) => setQuantity(e.currentTarget.value)}
                />
              </div>
              <div class="field">
                <label for="manual-price">
                  {operation() === "STAKING" ? "Cours à la réception (€)" : "Prix unitaire (€)"}
                </label>
                <input
                  id="manual-price"
                  class="num"
                  inputmode="decimal"
                  required
                  value={unitPrice()}
                  onInput={(e) => setUnitPrice(e.currentTarget.value)}
                />
              </div>
              <Show when={operation() !== "STAKING"}>
                <div class="field">
                  <label for="manual-fees">Frais (€)</label>
                  <input
                    id="manual-fees"
                    class="num"
                    inputmode="decimal"
                    required
                    value={fees()}
                    onInput={(e) => setFees(e.currentTarget.value)}
                  />
                </div>
              </Show>
            </div>
          }
        >
          <div class="field">
            <label for="manual-amount">Montant net reçu (€)</label>
            <input
              id="manual-amount"
              class="num"
              inputmode="decimal"
              required
              value={amount()}
              onInput={(e) => setAmount(e.currentTarget.value)}
            />
          </div>
        </Show>

        <Show when={operation() === "STAKING"}>
          <p class="muted note">
            La récompense sera comptée comme un dividende en nature, pas comme une moins-value.
          </p>
        </Show>
        <Show when={error()}>
          <p class="dialog-error">{error()}</p>
        </Show>
        <footer class="dialog-actions">
          <Show when={existing} fallback={<span />}>
            <button
              type="button"
              class="btn danger ghost"
              classList={{ arming: confirmDelete() }}
              disabled={busy()}
              onClick={remove}
            >
              {confirmDelete() ? "Confirmer la suppression ?" : "Supprimer"}
            </button>
          </Show>
          <span class="dialog-buttons">
            <button type="button" class="btn" disabled={busy()} onClick={() => props.onClose(false)}>
              Annuler
            </button>
            <button type="submit" class="btn primary" disabled={busy()}>
              {busy() ? "Enregistrement…" : "Enregistrer"}
            </button>
          </span>
        </footer>
      </form>
    </div>
  );
}

/** Formulaire d'édition de l'ordre sous-jacent à un lot. Les valeurs
 *  d'origine sont archivées côté Rust et restaurables à tout moment. */
function EditDialog(props: {
  lot: LotView;
  positionName: string;
  onClose: (saved: boolean) => void;
}) {
  const [date_, setDate] = createSignal(props.lot.acquisition_date ?? "");
  const [qty, setQty] = createSignal(props.lot.initial_quantity);
  const [price, setPrice] = createSignal(props.lot.unit_cost);
  const [fees, setFees] = createSignal(props.lot.fees);
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);

  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") props.onClose(false);
  };
  onMount(() => document.addEventListener("keydown", onKey));
  onCleanup(() => document.removeEventListener("keydown", onKey));

  async function save(e: Event) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await invoke("update_transaction", {
        id: props.lot.tx_id,
        date: date_() || null,
        quantity: qty(),
        unitPrice: price(),
        fees: fees(),
      });
      props.onClose(true);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function revert() {
    setBusy(true);
    setError(null);
    try {
      await invoke("revert_transaction", { id: props.lot.tx_id });
      props.onClose(true);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div class="overlay" onClick={() => props.onClose(false)}>
      <form
        class="dialog"
        role="dialog"
        aria-label={`Modifier l'ordre ${props.positionName}`}
        onClick={(e) => e.stopPropagation()}
        onSubmit={save}
      >
        <header>
          <h2>Modifier l'ordre</h2>
          <p class="muted">
            {props.positionName} · {props.lot.origin_broker}
            <Show when={props.lot.unreconciled}>
              {" "}
              · lot non rapproché (coût estimé au PRU courtier)
            </Show>
          </p>
        </header>
        <div class="field">
          <label for="edit-date">Date d'achat</label>
          <input
            id="edit-date"
            type="date"
            value={date_()}
            onInput={(e) => setDate(e.currentTarget.value)}
          />
        </div>
        <div class="field-row">
          <div class="field">
            <label for="edit-qty">Quantité</label>
            <input
              id="edit-qty"
              class="num"
              inputmode="decimal"
              required
              value={qty()}
              onInput={(e) => setQty(e.currentTarget.value)}
            />
          </div>
          <div class="field">
            <label for="edit-price">Prix unitaire (€)</label>
            <input
              id="edit-price"
              class="num"
              inputmode="decimal"
              required
              value={price()}
              onInput={(e) => setPrice(e.currentTarget.value)}
            />
          </div>
          <div class="field">
            <label for="edit-fees">Frais (€)</label>
            <input
              id="edit-fees"
              class="num"
              inputmode="decimal"
              required
              value={fees()}
              onInput={(e) => setFees(e.currentTarget.value)}
            />
          </div>
        </div>
        <Show when={props.lot.remaining_quantity !== props.lot.initial_quantity}>
          <p class="muted note">
            La quantité modifiée est celle de l'ordre initial (
            {num(props.lot.initial_quantity)}) ; les ventes déjà passées seront
            recalculées.
          </p>
        </Show>
        <Show when={error()}>
          <p class="dialog-error">{error()}</p>
        </Show>
        <footer class="dialog-actions">
          <Show when={props.lot.edited} fallback={<span />}>
            <button type="button" class="btn ghost" disabled={busy()} onClick={revert}>
              Restaurer l'original
            </button>
          </Show>
          <span class="dialog-buttons">
            <button type="button" class="btn" disabled={busy()} onClick={() => props.onClose(false)}>
              Annuler
            </button>
            <button type="submit" class="btn primary" disabled={busy()}>
              Enregistrer
            </button>
          </span>
        </footer>
      </form>
    </div>
  );
}

function ManualTransactions(props: {
  transactions: ManualTransaction[];
  onEdit: (transaction: ManualTransaction) => void;
}) {
  const [open, setOpen] = createSignal(true);

  function detail(transaction: ManualTransaction): string {
    if (transaction.operation === "DIVIDEND") {
      return transaction.amount ? eur(transaction.amount) : "—";
    }
    const base = `${num(transaction.quantity)} × ${eur(transaction.unit_price)}`;
    return Number(transaction.fees) > 0 ? `${base} · ${eur(transaction.fees)} de frais` : base;
  }

  return (
    <section class="manual-entries" aria-label="Saisies manuelles">
      <button
        type="button"
        class="manual-entries-head"
        aria-expanded={open()}
        onClick={() => setOpen(!open())}
      >
        <span>
          <strong>Saisies manuelles</strong>
          <span class="badge manual">{props.transactions.length}</span>
        </span>
        <span class="muted">{open() ? "Masquer" : "Afficher"}</span>
      </button>
      <Show when={open()}>
        <div class="manual-list">
          <For each={props.transactions}>
            {(transaction) => (
              <button
                type="button"
                class="manual-entry"
                onClick={() => props.onEdit(transaction)}
                aria-label={`Modifier ${operationLabels[transaction.operation]} ${transaction.instrument_name}`}
              >
                <span class={`operation operation-${transaction.operation.toLowerCase()}`}>
                  {operationLabels[transaction.operation]}
                </span>
                <span class="manual-entry-main">
                  <strong>{transaction.instrument_name}</strong>
                  <span class="muted">
                    {date(transaction.date)} · {transaction.broker}
                  </span>
                </span>
                <span class="num manual-entry-detail">{detail(transaction)}</span>
                <span class="manual-entry-edit" aria-hidden="true">✎</span>
              </button>
            )}
          </For>
        </div>
      </Show>
    </section>
  );
}

function LotRow(props: {
  lot: LotView;
  child?: boolean;
  expanded?: boolean;
  onToggle?: () => void;
  onEdit: (lot: LotView) => void;
}) {
  const lot = () => props.lot;
  return (
    <div class="lot-row" classList={{ "lot-child": props.child }} role="row">
      <span class="lot-date">
        <Show
          when={lot().income_events > 0 && !props.child}
          fallback={date(lot().acquisition_date)}
        >
          <Show
            when={lot().children.length > 0}
            fallback={
              <>
                <span>Staking</span>
                <span class="muted">{date(lot().acquisition_date)}</span>
              </>
            }
          >
            <button
              type="button"
              class="btn-toggle"
              aria-expanded={props.expanded}
              title="Afficher chaque versement"
              onClick={props.onToggle}
            >
              <span aria-hidden="true">{props.expanded ? "▾" : "▸"}</span> Staking
            </button>
            <span class="badge income">
              {lot().income_events} versement{lot().income_events > 1 ? "s" : ""}
            </span>
            <span class="muted">depuis {date(lot().acquisition_date)}</span>
          </Show>
        </Show>
        <Show when={lot().unreconciled}>
          <span class="badge warn" title="Titres reçus par transfert sans historique d'achat : coût estimé au PRU courtier. Importez l'historique d'origine pour rapprocher.">
            non rapproché
          </span>
        </Show>
        <Show when={lot().edited}>
          <span class="badge edited" title="Ordre modifié manuellement — les valeurs importées restent restaurables.">
            modifié
          </span>
        </Show>
        <Show when={lot().manual}>
          <span class="badge manual" title="Opération ajoutée manuellement.">
            manuel
          </span>
        </Show>
      </span>
      <span class="lot-broker">{lot().origin_broker}</span>
      <span class="num lot-stat lot-quantity" data-label="Quantité">
        {num(lot().remaining_quantity)}
        <Show when={lot().remaining_quantity !== lot().initial_quantity}>
          <span class="muted"> / {num(lot().initial_quantity)}</span>
        </Show>
      </span>
      <span class="num lot-stat lot-price" data-label="Prix de référence">
        {eur(lot().unit_cost)}
      </span>
      <span class="num muted lot-stat lot-fees" data-label="Frais">
        {Number(lot().fees) ? eur(lot().fees) : "—"}
      </span>
      <span class="num lot-stat lot-invested" data-label="Investi">
        {eur(lot().invested)}
      </span>
      <span class="num lot-stat lot-value" data-label="Valeur">
        {eur(lot().market_value)}
      </span>
      <span
        class={`num lot-stat lot-pnl ${perfClass(lot().pnl)}`}
        data-label="P/L latent"
      >
        {lot().pnl ? eur(lot().pnl) : "—"}
      </span>
      <span class="perf-col lot-stat lot-performance" data-label="Performance">
        <DivergingBar pct={lot().pnl_pct} />
        <span class={`num pct ${perfClass(lot().pnl_pct)}`}>{pct(lot().pnl_pct)}</span>
      </span>
      <span class="num muted lot-stat lot-holding" data-label="Détention">
        {holdingSince(lot().acquisition_date)}
      </span>
      <span class="lot-actions">
        <Show when={lot().tx_id !== null}>
          <button
            type="button"
            class="btn-icon"
            title="Modifier cet ordre"
            aria-label="Modifier cet ordre"
            onClick={() => props.onEdit(lot())}
          >
            ✎
          </button>
        </Show>
      </span>
    </div>
  );
}

function LotRows(props: {
  position: PositionView;
  onEdit: (lot: LotView) => void;
}) {
  const [expanded, setExpanded] = createSignal<Set<number>>(new Set());
  const toggle = (id: number) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  return (
    <div class="lots">
      <div class="lot-row lot-head" role="row">
        <span>Ordre</span>
        <span>Courtier</span>
        <span class="num">Quantité</span>
        <span class="num">Prix</span>
        <span class="num">Frais</span>
        <span class="num">Investi</span>
        <span class="num">Valeur</span>
        <span class="num">P/L €</span>
        <span class="perf-col">P/L %</span>
        <span class="num">Détention</span>
        <span />
      </div>
      <For each={props.position.lots}>
        {(lot) => (
          <>
            <LotRow
              lot={lot}
              expanded={expanded().has(lot.id)}
              onToggle={() => toggle(lot.id)}
              onEdit={props.onEdit}
            />
            <Show when={lot.children.length > 0 && expanded().has(lot.id)}>
              <For each={lot.children}>
                {(child) => <LotRow lot={child} child onEdit={props.onEdit} />}
              </For>
            </Show>
          </>
        )}
      </For>
    </div>
  );
}

function PositionRow(props: {
  position: PositionView;
  onEdit: (lot: LotView, positionName: string) => void;
}) {
  const p = props.position;
  const [open, setOpen] = createSignal(false);
  return (
    <article class="position" classList={{ open: open() }}>
      <button
        class="position-head"
        onClick={() => setOpen(!open())}
        aria-expanded={open()}
      >
        <span class="chevron" aria-hidden="true">
          ▸
        </span>
        <span class="position-name">
          <strong>{p.name}</strong>
          <span class="symbol">{p.symbol ?? "?"}</span>
          <Show when={Number(p.unreconciled_quantity) > 0}>
            <span class="badge warn">{num(p.unreconciled_quantity)} sans historique</span>
          </Show>
        </span>
        <span class="orders-strip position-orders" title={`${p.lots.length} lot${p.lots.length > 1 ? "s" : ""} — achats et staking regroupé`}>
          <For each={p.lots}>
            {(lot) => (
              <span
                class={`cell ${heatClass(lot)}`}
                title={`${lot.income_events > 0 ? `Staking · ${lot.income_events} versements` : date(lot.acquisition_date)} · ${num(lot.remaining_quantity)} × ${eur(lot.unit_cost)} · ${pct(lot.pnl_pct)}`}
              />
            )}
          </For>
        </span>
        <span class="num qty position-metric position-quantity" data-label="Quantité">
          {num(p.quantity)}
        </span>
        <span class="num position-metric position-quote" data-label="Cours">
          {p.quote ? eur(p.quote.price) : "—"}
        </span>
        <span class="num position-metric position-value" data-label="Valeur">
          {eur(p.market_value)}
        </span>
        <span class="num muted position-metric position-pru" data-label="PRU">
          {p.avg_cost ? eur(p.avg_cost) : "—"}
        </span>
        <span
          class={`num position-metric position-pnl ${perfClass(p.pnl)}`}
          data-label="P/L latent"
        >
          {p.pnl ? eur(p.pnl) : "—"}
        </span>
        <span
          class={`num pct position-metric position-pct ${perfClass(p.pnl_pct)}`}
          data-label="Performance"
        >
          {pct(p.pnl_pct)}
        </span>
      </button>
      <Show when={open()}>
        <LotRows position={p} onEdit={(lot) => props.onEdit(lot, p.name)} />
        <Show when={Number(p.dividends) > 0 || Number(p.realized_pnl) !== 0}>
          <div class="position-foot">
            <Show when={Number(p.dividends) > 0}>
              <span>
                Dividendes perçus : <b class="num">{eur(p.dividends)}</b>
              </span>
            </Show>
            <Show when={Number(p.realized_pnl) !== 0}>
              <span>
                P/L réalisé : <b class={`num ${perfClass(p.realized_pnl)}`}>{eur(p.realized_pnl)}</b>
              </span>
            </Show>
          </div>
        </Show>
      </Show>
    </article>
  );
}

export default function App() {
  const [portfolio, { refetch }] = createResource<PortfolioView>(() =>
    invoke<PortfolioView>("get_portfolio"),
  );
  const [manualTransactions, { refetch: refetchManualTransactions }] =
    createResource<ManualTransaction[]>(() =>
      invoke<ManualTransaction[]>("get_manual_transactions"),
    );
  const [reports, setReports] = createSignal<ImportReport[]>([]);
  const [importError, setImportError] = createSignal<string | null>(null);
  const [refreshing, setRefreshing] = createSignal(false);
  const [quoteErrors, setQuoteErrors] = createSignal<string[]>([]);
  const [showWarnings, setShowWarnings] = createSignal(false);
  const [deletingDemo, setDeletingDemo] = createSignal(false);
  const [demoError, setDemoError] = createSignal<string | null>(null);
  const [editing, setEditing] = createSignal<{ lot: LotView; name: string } | null>(null);
  const [manualDialog, setManualDialog] = createSignal<{
    transaction: ManualTransaction | null;
  } | null>(null);
  let fileInput!: HTMLInputElement;

  async function onEditClosed(saved: boolean) {
    setEditing(null);
    if (saved) await refetch();
  }

  async function onManualDialogClosed(saved: boolean) {
    setManualDialog(null);
    if (saved) {
      await Promise.all([refetch(), refetchManualTransactions()]);
    }
  }

  function editLot(lot: LotView, name: string) {
    if (lot.manual && lot.tx_id !== null) {
      const transaction = manualTransactions()?.find((item) => item.id === lot.tx_id);
      if (transaction) {
        setManualDialog({ transaction });
        return;
      }
    }
    setEditing({ lot, name });
  }

  // Rafraîchit les cours au lancement ; en cas d'échec (hors ligne), les
  // derniers cours en cache restent affichés et l'erreur apparaît par symbole.
  onMount(() => {
    void onRefreshQuotes();
  });

  async function onFiles(files: FileList | null) {
    if (!files || files.length === 0) return;
    setImportError(null);
    const next: ImportReport[] = [];
    for (const file of Array.from(files)) {
      try {
        const content = await file.text();
        next.push(
          await invoke<ImportReport>("import_csv", { fileName: file.name, content }),
        );
      } catch (e) {
        setImportError(`${file.name} : ${String(e)}`);
      }
    }
    setReports(next);
    fileInput.value = "";
    await refetch();
  }

  async function onRefreshQuotes() {
    setRefreshing(true);
    setQuoteErrors([]);
    try {
      const results = await invoke<QuoteRefresh[]>("refresh_quotes");
      setQuoteErrors(
        results.filter((r) => r.error).map((r) => `${r.symbol} : ${r.error}`),
      );
      await refetch();
    } catch (e) {
      setQuoteErrors([String(e)]);
    } finally {
      setRefreshing(false);
    }
  }

  // Deux clics plutôt que window.confirm (absent du WebView iOS/macOS).
  const [confirmDemoDelete, setConfirmDemoDelete] = createSignal(false);

  // Bandeau des totaux : compact quand on défile (hystérésis anti-va-et-vient).
  const [scrolled, setScrolled] = createSignal(false);
  onMount(() => {
    const onScroll = () => {
      const y = window.scrollY;
      setScrolled((prev) => (prev ? y > 90 : y > 170));
    };
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    onCleanup(() => window.removeEventListener("scroll", onScroll));
  });

  async function onDeleteDemo() {
    if (!confirmDemoDelete()) {
      setConfirmDemoDelete(true);
      return;
    }
    setConfirmDemoDelete(false);
    setDeletingDemo(true);
    setDemoError(null);
    try {
      await invoke("delete_demo_data");
      await refetch();
    } catch (e) {
      setDemoError(String(e));
    } finally {
      setDeletingDemo(false);
    }
  }

  return (
    <main class="app">
      <header class="topbar">
        <h1>
          <span>TruePerf</span>
          <span class="tagline">performance par lot d'acquisition</span>
        </h1>
        <div class="actions">
          <button class="btn" onClick={onRefreshQuotes} disabled={refreshing()}>
            <span class="btn-symbol" aria-hidden="true">↻</span>
            <span>{refreshing() ? "Actualisation…" : "Actualiser"}</span>
          </button>
          <button class="btn primary" onClick={() => setManualDialog({ transaction: null })}>
            <span class="btn-symbol" aria-hidden="true">＋</span>
            <span>Ajouter</span>
          </button>
          <button class="btn" onClick={() => fileInput.click()}>
            <span class="btn-symbol" aria-hidden="true">＋</span>
            <span>Importer</span>
          </button>
          <input
            ref={fileInput}
            type="file"
            accept=".csv,text/csv"
            multiple
            hidden
            onChange={(e) => onFiles(e.currentTarget.files)}
          />
        </div>
      </header>

      <Show when={portfolio()}>
        {(p) => (
          <>
            <Show when={p().has_demo_data}>
              <section class="demo-banner" aria-label="Données de démonstration">
                <div class="demo-copy">
                  <span class="demo-eyebrow">Mode découverte</span>
                  <p>
                    Ces lignes sont des exemples : achats Air Liquide et
                    TotalEnergies, dividendes, achat Solana et vos sept versements
                    de staking Trade Republic.
                  </p>
                  <Show when={demoError()}>
                    <span class="demo-error">{demoError()}</span>
                  </Show>
                </div>
                <button
                  type="button"
                  class="btn demo-delete"
                  disabled={deletingDemo()}
                  onClick={onDeleteDemo}
                >
                  {deletingDemo()
                    ? "Suppression…"
                    : confirmDemoDelete()
                      ? "Confirmer la suppression ?"
                      : "Supprimer les exemples"}
                </button>
              </section>
            </Show>
            <section
              class={`totals${scrolled() ? " scrolled" : ""}`}
              aria-label="Totaux du portefeuille"
            >
              <div class="total total-primary">
                <span class="label">Valeur</span>
                <span class="value num">{eur(p().total_market_value)}</span>
              </div>
              <div class="total total-invested">
                <span class="label">Investi</span>
                <span class="value num">{eur(p().total_invested)}</span>
              </div>
              <div class="total total-unrealized">
                <span class="label">P/L latent</span>
                <span class={`value num ${perfClass(p().total_pnl)}`}>
                  {eur(p().total_pnl)}
                </span>
              </div>
              <div class="total total-realized">
                <span class="label">P/L réalisé</span>
                <span class={`value num ${perfClass(p().total_realized_pnl)}`}>
                  {eur(p().total_realized_pnl)}
                </span>
              </div>
              <div class="total total-income">
                <span class="label">Dividendes</span>
                <span class="value num">{eur(p().total_dividends)}</span>
              </div>
            </section>

            <Show when={reports().length > 0 || importError()}>
              <section class="reports">
                <Show when={importError()}>
                  <p class="report error">{importError()}</p>
                </Show>
                <For each={reports()}>
                  {(r) => (
                    <div class="report">
                      <p>
                        <b>{r.broker}</b> — {r.file_name} : {r.total} opérations
                        détectées, <b>{r.inserted} nouvelles</b>, {r.duplicates} déjà
                        importées.
                        <Show when={r.file_already_imported}>
                          {" "}
                          <span class="muted">(fichier déjà connu)</span>
                        </Show>
                      </p>
                      <p class="muted">
                        {r.by_type.map(([t, n]) => `${n} ${t}`).join(" · ")}
                      </p>
                      <Show when={r.warnings.length > 0}>
                        <ul class="warnings">
                          <For each={r.warnings}>{(w) => <li>⚠ {w}</li>}</For>
                        </ul>
                      </Show>
                    </div>
                  )}
                </For>
              </section>
            </Show>

            <Show when={quoteErrors().length > 0}>
              <section class="reports">
                <div class="report">
                  <p>Cours indisponibles (dernier cours en cache conservé) :</p>
                  <ul class="warnings">
                    <For each={quoteErrors()}>{(e) => <li>⚠ {e}</li>}</For>
                  </ul>
                </div>
              </section>
            </Show>

            <Show when={(manualTransactions()?.length ?? 0) > 0}>
              <ManualTransactions
                transactions={manualTransactions() ?? []}
                onEdit={(transaction) => setManualDialog({ transaction })}
              />
            </Show>

            <Show
              when={p().positions.length > 0}
              fallback={
                <section class="empty">
                  <h2>Aucune position</h2>
                  <p>
                    Ajoutez une opération à la main ou importez vos relevés CSV
                    (Bourse Direct, Trade Republic, Yahoo/BoursoBank). Les transferts
                    de titres (<code>VIRT TITRES</code>) sont reconnus et ne créent
                    jamais de faux achats.
                  </p>
                </section>
              }
            >
              <section class="positions" role="table" aria-label="Positions">
                <div class="position-head columns" role="row">
                  <span />
                  <span>Instrument</span>
                  <span>Ordres</span>
                  <span class="num">Quantité</span>
                  <span class="num">Cours</span>
                  <span class="num">Valeur</span>
                  <span class="num">PRU</span>
                  <span class="num">P/L €</span>
                  <span class="num">P/L %</span>
                </div>
                <For each={p().positions}>
                  {(position) => (
                    <PositionRow
                      position={position}
                      onEdit={editLot}
                    />
                  )}
                </For>
              </section>
            </Show>

            <Show when={p().warnings.length > 0}>
              <section class="engine-warnings">
                <button class="btn ghost" onClick={() => setShowWarnings(!showWarnings())}>
                  {showWarnings() ? "Masquer" : "Afficher"} les avertissements du moteur (
                  {p().warnings.length})
                </button>
                <Show when={showWarnings()}>
                  <ul class="warnings">
                    <For each={p().warnings}>{(w) => <li>⚠ {w}</li>}</For>
                  </ul>
                </Show>
              </section>
            </Show>
          </>
        )}
      </Show>
      <Show when={portfolio.error}>
        <section class="empty">
          <h2>Erreur</h2>
          <p>{String(portfolio.error)}</p>
        </section>
      </Show>
      <Show when={editing()}>
        {(e) => (
          <EditDialog lot={e().lot} positionName={e().name} onClose={onEditClosed} />
        )}
      </Show>
      <Show when={manualDialog()}>
        {(dialog) => (
          <ManualTransactionDialog
            transaction={dialog().transaction}
            instruments={portfolio()?.positions ?? []}
            onClose={onManualDialogClosed}
          />
        )}
      </Show>
    </main>
  );
}
