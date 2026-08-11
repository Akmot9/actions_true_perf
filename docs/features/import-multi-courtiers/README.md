# Feature — Import multi-courtiers

Statut : **Implémentée**

Objectif : transformer plusieurs formats CSV en transactions homogènes sans
demander à l'utilisateur de choisir manuellement le format.

## US-IMP-001 — Importer un ou plusieurs relevés CSV

**En tant qu'investisseur, je veux sélectionner un ou plusieurs fichiers CSV afin de consolider mes opérations dans un seul portefeuille.**

Critères d'acceptation :

- Le sélecteur accepte plusieurs fichiers `.csv` au cours d'une même action.
- Chaque fichier est traité indépendamment et le portefeuille est recalculé à la fin.
- Une erreur sur un fichier est affichée avec son nom sans masquer les rapports déjà produits.

## US-IMP-002 — Détecter automatiquement le format

**En tant qu'investisseur, je veux que le format du relevé soit reconnu automatiquement afin de ne pas configurer chaque courtier.**

Critères d'acceptation :

- Les en-têtes permettent de reconnaître Bourse Direct, Trade Republic et l'export `portfolio.csv` Yahoo/BoursoBank.
- Un fichier dont les en-têtes ne correspondent à aucun format produit une erreur explicite.
- Les lignes CSV flexibles et les décimales françaises avec virgule ou espaces sont acceptées.

## US-IMP-003 — Importer un relevé Bourse Direct

**En tant que client Bourse Direct, je veux retrouver la nature réelle de chaque opération afin d'éviter de faux achats ou de fausses performances.**

Critères d'acceptation :

- `ACH CPT`, `VTE CPT`, `COUPONS` et `VIRT TITRES` deviennent respectivement achat, vente, dividende et transfert entrant.
- Les frais d'achat et de vente sont déduits des montants débités ou crédités lorsqu'ils ne sont pas fournis séparément.
- Une indemnisation avec quantité négative est traitée comme une cession forcée de quantité positive.
- Les frais, mouvements d'espèces, rompus et changements internes de valeur sont conservés sans créer de faux lots.
- Une désignation inconnue devient `OTHER` avec un avertissement ; elle n'est pas ignorée silencieusement.

## US-IMP-004 — Importer l'export officiel Trade Republic

**En tant que client Trade Republic, je veux importer mon historique officiel afin d'intégrer actions, ETF et cryptomonnaies.**

Critères d'acceptation :

- Les achats et ventes `TRADING` conservent quantité, prix, frais et taxes.
- Les quantités négatives des ventes sont normalisées en quantités positives.
- Les dividendes espèces sont classés comme dividendes.
- Les récompenses `DELIVERY/FREE_RECEIPT` sont classées comme dividendes en nature de staking.
- Les attributions gratuites `BONUS_ISSUE` créent un lot à coût nul.
- Les lignes espèces associées à une opération déjà représentée ne créent pas un deuxième lot.

## US-IMP-005 — Importer l'historique Yahoo/BoursoBank

**En tant qu'ancien client BoursoBank, je veux importer chaque ligne de mon export portfolio comme un ordre distinct afin de préserver mon historique d'acquisition.**

Critères d'acceptation :

- Chaque ligne `BUY` valide crée un lot distinct avec sa date, sa quantité, son prix et sa commission.
- Une ligne sans date reste importée comme lot de date inconnue et génère un avertissement.
- Une ligne sans quantité ou prix est ignorée avec un avertissement.
- Un type différent de `BUY` est signalé et ignoré.
- Les cours présents dans `Current Price` alimentent le cache local de cotations.

## US-IMP-006 — Comprendre le résultat d'un import

**En tant qu'investisseur, je veux un bilan après chaque import afin de vérifier ce qui a été reconnu.**

Critères d'acceptation :

- Le rapport affiche le courtier, le fichier, le nombre total d'opérations, les nouvelles lignes et les doublons.
- Le rapport ventile les opérations par type normalisé.
- Les dates invalides, instruments inconnus et opérations non reconnues sont listés comme avertissements.
- Un fichier déjà connu est clairement identifié.

## US-IMP-007 — Créer automatiquement le compte du courtier

**En tant qu'investisseur, je veux que mes opérations soient rattachées au bon compte sans paramétrage préalable.**

Critères d'acceptation :

- Bourse Direct et Yahoo/BoursoBank créent ou réutilisent un compte de type PEA.
- Trade Republic crée ou réutilise un compte de type CTO.
- Les opérations importées conservent le courtier d'origine.

Traçabilité : `src-tauri/crates/portfolio-core/src/import/`, `src/App.tsx`, `src-tauri/src/commands.rs`.
