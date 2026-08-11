# Feature — Saisie manuelle

Statut : **Implémentée**

Objectif : compléter un historique sans CSV tout en distinguant clairement les
opérations saisies par l'utilisateur des données importées.

## US-MAN-001 — Ajouter une opération à la main

**En tant qu'investisseur, je veux saisir une opération absente de mes exports afin d'obtenir un portefeuille complet.**

Critères d'acceptation :

- L'action `Ajouter` ouvre un formulaire adapté au mobile et au bureau.
- L'utilisateur choisit achat, vente, dividende en espèces ou staking.
- La date, l'instrument, le symbole de marché, le courtier et le type de compte sont renseignables.
- Un instrument existant peut être repris depuis les suggestions ou créé par son nom et son symbole.

## US-MAN-002 — Adapter les montants au type d'opération

**En tant qu'investisseur, je veux ne saisir que les valeurs utiles afin d'éviter les ambiguïtés de calcul.**

Critères d'acceptation :

- Un achat ou une vente demande quantité, prix unitaire et frais.
- Un dividende demande le montant net reçu.
- Le staking demande la quantité reçue et son cours à la réception.
- Les quantités et montants obligatoires doivent être strictement positifs ; prix et frais ne peuvent pas être négatifs.
- Les nombres français avec virgule sont acceptés.

## US-MAN-003 — Identifier une saisie manuelle

**En tant qu'investisseur, je veux reconnaître les lignes que j'ai ajoutées afin de garder une piste d'audit claire.**

Critères d'acceptation :

- Un achat saisi affiche un badge `manuel` dans son lot.
- Toutes les opérations manuelles sont regroupées dans une section dédiée.
- La section présente le type, l'instrument, la date, le compte et les valeurs principales.
- Une saisie manuelle n'est rattachée à aucun fichier d'import.

## US-MAN-004 — Modifier une saisie manuelle

**En tant qu'investisseur, je veux corriger toute valeur saisie afin de réparer une erreur sans recréer la ligne.**

Critères d'acceptation :

- Une ligne de la section dédiée rouvre le formulaire prérempli.
- Le type d'opération, l'instrument, le compte, la date et les montants sont modifiables.
- Le portefeuille et la liste des saisies sont recalculés immédiatement après l'enregistrement.

## US-MAN-005 — Supprimer une saisie sans toucher aux imports

**En tant qu'investisseur, je veux supprimer une ligne ajoutée à la main sans risquer d'effacer une donnée importée.**

Critères d'acceptation :

- La suppression demande une confirmation explicite.
- La base refuse la suppression si la transaction est rattachée à un fichier source.
- Le portefeuille est entièrement rejoué après suppression.

## US-MAN-006 — Traiter correctement le staking manuel

**En tant qu'investisseur crypto, je veux que mon staking saisi à la main reste un revenu en nature afin qu'il ne soit pas pris pour un achat.**

Critères d'acceptation :

- Le staking est persisté comme dividende avec quantité et prix de réception.
- Sa valeur alimente les dividendes et sa quantité rejoint le lot synthétique de staking.
- Le formulaire rappelle que la variation ultérieure du cours n'annule pas le revenu reçu.

Traçabilité : `src/App.tsx`, `src/App.css`, `src/types.ts`,
`src-tauri/src/commands.rs`, `src-tauri/src/db.rs`, `src-tauri/src/lib.rs`.
