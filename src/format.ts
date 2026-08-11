const eurFmt = new Intl.NumberFormat("fr-FR", {
  style: "currency",
  currency: "EUR",
});

const numFmt = new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 4 });

export function eur(value: string | number | null | undefined): string {
  if (value === null || value === undefined || value === "") return "—";
  return eurFmt.format(Number(value));
}

export function num(value: string | number | null | undefined): string {
  if (value === null || value === undefined || value === "") return "—";
  const n = Number(value);
  // Les quantités crypto minuscules (récompenses de staking) seraient
  // arrondies à « 0 » avec 4 décimales : basculer en chiffres significatifs.
  if (n !== 0 && Math.abs(n) < 0.001) {
    return n.toLocaleString("fr-FR", { maximumSignificantDigits: 3 });
  }
  return numFmt.format(n);
}

export function pct(value: string | number | null | undefined): string {
  if (value === null || value === undefined || value === "") return "—";
  const n = Number(value);
  const sign = n > 0 ? "+" : "";
  return `${sign}${n.toLocaleString("fr-FR", { maximumFractionDigits: 2 })} %`;
}

export function date(iso: string | null): string {
  if (!iso) return "date inconnue";
  const [y, m, d] = iso.split("-");
  return `${d}/${m}/${y}`;
}

/** Durée de détention depuis une date ISO, en années/mois. */
export function holdingSince(iso: string | null): string {
  if (!iso) return "—";
  const from = new Date(iso);
  const now = new Date();
  let months =
    (now.getFullYear() - from.getFullYear()) * 12 + (now.getMonth() - from.getMonth());
  if (now.getDate() < from.getDate()) months -= 1;
  if (months < 1) return "< 1 mois";
  const years = Math.floor(months / 12);
  const rest = months % 12;
  if (years === 0) return `${rest} mois`;
  if (rest === 0) return `${years} an${years > 1 ? "s" : ""}`;
  return `${years} an${years > 1 ? "s" : ""} ${rest} m`;
}

/** Classe CSS de performance (gain/perte/neutre). */
export function perfClass(value: string | null | undefined): string {
  if (value === null || value === undefined || value === "") return "";
  const n = Number(value);
  if (n > 0.001) return "gain";
  if (n < -0.001) return "loss";
  return "";
}
