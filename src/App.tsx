import { createSignal, createResource, For, Show, onCleanup, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import type {
  ImportReport,
  LotView,
  PortfolioView,
  PositionView,
  QuoteRefresh,
} from "./types";
import { date, eur, holdingSince, num, pct, perfClass } from "./format";
import "./App.css";

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

function LotRows(props: {
  position: PositionView;
  onEdit: (lot: LotView) => void;
}) {
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
          <div class="lot-row" role="row">
            <span class="lot-date">
              {date(lot.acquisition_date)}
              <Show when={lot.unreconciled}>
                <span class="badge warn" title="Titres reçus par transfert sans historique d'achat : coût estimé au PRU courtier. Importez l'historique d'origine pour rapprocher.">
                  non rapproché
                </span>
              </Show>
              <Show when={lot.edited}>
                <span class="badge edited" title="Ordre modifié manuellement — les valeurs importées restent restaurables.">
                  modifié
                </span>
              </Show>
            </span>
            <span class="lot-broker">{lot.origin_broker}</span>
            <span class="num">
              {num(lot.remaining_quantity)}
              <Show when={lot.remaining_quantity !== lot.initial_quantity}>
                <span class="muted"> / {num(lot.initial_quantity)}</span>
              </Show>
            </span>
            <span class="num">{eur(lot.unit_cost)}</span>
            <span class="num muted">{Number(lot.fees) ? eur(lot.fees) : "—"}</span>
            <span class="num">{eur(lot.invested)}</span>
            <span class="num">{eur(lot.market_value)}</span>
            <span class={`num ${perfClass(lot.pnl)}`}>{lot.pnl ? eur(lot.pnl) : "—"}</span>
            <span class="perf-col">
              <DivergingBar pct={lot.pnl_pct} />
              <span class={`num pct ${perfClass(lot.pnl_pct)}`}>{pct(lot.pnl_pct)}</span>
            </span>
            <span class="num muted">{holdingSince(lot.acquisition_date)}</span>
            <span>
              <Show when={lot.tx_id !== null}>
                <button
                  type="button"
                  class="btn-icon"
                  title="Modifier cet ordre"
                  aria-label="Modifier cet ordre"
                  onClick={() => props.onEdit(lot)}
                >
                  ✎
                </button>
              </Show>
            </span>
          </div>
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
        <span class="orders-strip" title={`${p.lots.length} ordre${p.lots.length > 1 ? "s" : ""} — un carré par ordre`}>
          <For each={p.lots}>
            {(lot) => (
              <span
                class={`cell ${heatClass(lot)}`}
                title={`${date(lot.acquisition_date)} · ${num(lot.remaining_quantity)} × ${eur(lot.unit_cost)} · ${pct(lot.pnl_pct)}`}
              />
            )}
          </For>
        </span>
        <span class="num qty">{num(p.quantity)}</span>
        <span class="num">{p.quote ? eur(p.quote.price) : "—"}</span>
        <span class="num">{eur(p.market_value)}</span>
        <span class="num muted">{p.avg_cost ? eur(p.avg_cost) : "—"}</span>
        <span class={`num ${perfClass(p.pnl)}`}>{p.pnl ? eur(p.pnl) : "—"}</span>
        <span class={`num pct ${perfClass(p.pnl_pct)}`}>{pct(p.pnl_pct)}</span>
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
  const [reports, setReports] = createSignal<ImportReport[]>([]);
  const [importError, setImportError] = createSignal<string | null>(null);
  const [refreshing, setRefreshing] = createSignal(false);
  const [quoteErrors, setQuoteErrors] = createSignal<string[]>([]);
  const [showWarnings, setShowWarnings] = createSignal(false);
  const [editing, setEditing] = createSignal<{ lot: LotView; name: string } | null>(null);
  let fileInput!: HTMLInputElement;

  async function onEditClosed(saved: boolean) {
    setEditing(null);
    if (saved) await refetch();
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

  return (
    <main class="app">
      <header class="topbar">
        <h1>
          Suivi des ordres
          <span class="tagline">performance par lot d'acquisition</span>
        </h1>
        <div class="actions">
          <button class="btn" onClick={onRefreshQuotes} disabled={refreshing()}>
            {refreshing() ? "Cours en cours…" : "Actualiser les cours"}
          </button>
          <button class="btn primary" onClick={() => fileInput.click()}>
            Importer un relevé
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
            <section class="totals" aria-label="Totaux du portefeuille">
              <div class="total">
                <span class="label">Valeur</span>
                <span class="value num">{eur(p().total_market_value)}</span>
              </div>
              <div class="total">
                <span class="label">Investi</span>
                <span class="value num">{eur(p().total_invested)}</span>
              </div>
              <div class="total">
                <span class="label">P/L latent</span>
                <span class={`value num ${perfClass(p().total_pnl)}`}>
                  {eur(p().total_pnl)}
                </span>
              </div>
              <div class="total">
                <span class="label">P/L réalisé</span>
                <span class={`value num ${perfClass(p().total_realized_pnl)}`}>
                  {eur(p().total_realized_pnl)}
                </span>
              </div>
              <div class="total">
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

            <Show
              when={p().positions.length > 0}
              fallback={
                <section class="empty">
                  <h2>Aucune position</h2>
                  <p>
                    Importez vos relevés CSV (Bourse Direct, export portfolio
                    Yahoo/BoursoBank) avec le bouton « Importer un relevé ». Les
                    transferts de titres (<code>VIRT TITRES</code>) sont reconnus et ne
                    créent jamais de faux achats.
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
                      onEdit={(lot, name) => setEditing({ lot, name })}
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
    </main>
  );
}
