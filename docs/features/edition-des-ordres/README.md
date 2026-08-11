# Feature — Édition des ordres

Statut : **Implémentée**

Objectif : permettre une correction ponctuelle d'une opération importée sans
perdre la valeur source ni casser la déduplication.

## US-EDI-001 — Ouvrir l'édition d'un ordre

**En tant qu'investisseur, je veux corriger un ordre depuis son lot afin de réparer une donnée source inexacte.**

Critères d'acceptation :

- Un lot adossé à une transaction affiche une action d'édition.
- Le formulaire est prérempli avec la date, la quantité initiale, le prix et les frais.
- Un lot synthétique sans transaction unique, comme le staking regroupé, n'est pas éditable par ce parcours.

## US-EDI-002 — Valider les valeurs saisies

**En tant qu'investisseur, je veux être empêché d'enregistrer des valeurs impossibles afin de protéger les calculs du portefeuille.**

Critères d'acceptation :

- La quantité doit être strictement positive.
- Le prix et les frais ne peuvent pas être négatifs.
- Une date renseignée doit respecter le format attendu.
- Les décimales avec virgule sont acceptées.

## US-EDI-003 — Recalculer le portefeuille après correction

**En tant qu'investisseur, je veux voir immédiatement l'effet de ma correction sur les lots et performances.**

Critères d'acceptation :

- La transaction persistée reçoit les nouvelles valeurs.
- Le portefeuille est rechargé après l'enregistrement.
- Les ventes, transferts, quantités restantes, PRU et P/L sont entièrement rejoués.

## US-EDI-004 — Identifier une donnée modifiée

**En tant qu'investisseur, je veux distinguer une valeur corrigée d'une valeur importée afin de garder une piste d'audit.**

Critères d'acceptation :

- La première correction archive les valeurs originales.
- Le lot corrigé affiche un badge `modifié`.
- Les corrections ultérieures ne remplacent pas l'archive d'origine.

## US-EDI-005 — Restaurer l'original

**En tant qu'investisseur, je veux annuler ma correction afin de revenir exactement aux données importées.**

Critères d'acceptation :

- L'action `Restaurer l'original` n'apparaît que pour une transaction modifiée.
- La date, la quantité, le prix et les frais originaux sont restaurés.
- Le marquage de modification disparaît et le portefeuille est recalculé.

Traçabilité : `src/App.tsx`, `src-tauri/src/commands.rs`, `src-tauri/src/db.rs`.
