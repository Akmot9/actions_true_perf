# Feature — Staking et dividendes

Statut : **Implémentée**

Objectif : distinguer les revenus reçus des ordres d'achat tout en suivant la
valeur des actifs versés en nature.

## US-REV-001 — Comptabiliser un dividende en espèces

**En tant qu'investisseur, je veux voir les dividendes crédités afin de connaître les revenus générés par mes positions.**

Critères d'acceptation :

- Les coupons Bourse Direct et dividendes espèces Trade Republic sont classés `DIVIDEND`.
- Leur montant crédité est additionné par instrument.
- Un dividende en espèces ne modifie ni la quantité ni les lots d'acquisition.

## US-REV-002 — Reconnaître le staking comme dividende en nature

**En tant qu'investisseur crypto, je veux que les récompenses de staking soient considérées comme des revenus et non comme des achats.**

Critères d'acceptation :

- Une ligne Trade Republic `DELIVERY/FREE_RECEIPT` devient un dividende.
- La quantité reçue et le prix de marché à la réception sont conservés.
- La valeur du revenu est calculée par `quantité × prix de réception` lorsque le CSV ne contient pas de montant espèces.

## US-REV-003 — Regrouper les micro-versements de staking

**En tant qu'investisseur, je veux une ligne synthétique par crypto afin que les versements hebdomadaires ne masquent pas mes vrais ordres.**

Critères d'acceptation :

- Les versements sont regroupés par instrument et par compte dans un seul lot de revenus.
- La quantité cumulée et le nombre de versements restent disponibles.
- Le prix de référence du lot est une moyenne pondérée des prix de réception.
- Les ordres d'achat réels restent des lots individuels.

## US-REV-004 — Identifier clairement le lot de staking

**En tant qu'investisseur, je veux distinguer le staking d'un achat dans le détail de ma position.**

Critères d'acceptation :

- Le lot porte le libellé `Staking` et un badge indiquant le nombre de versements.
- La date affichée indique le début de la série de versements.
- Le lot synthétique n'affiche pas de bouton d'édition d'un ordre unique.

## US-REV-005 — Comprendre la variation après réception

**En tant qu'investisseur, je veux distinguer le revenu reçu de la variation du cours qui suit afin de ne pas interpréter le staking comme une dépense.**

Critères d'acceptation :

- La valeur à réception est comptée dans les dividendes et sert de prix de référence du lot.
- Le P/L du lot mesure uniquement la variation entre cette valeur de réception et la valeur actuelle.
- Une variation négative du lot ne transforme pas le revenu en perte nette : la contribution économique est le dividende reçu plus la variation ultérieure.

## US-REV-006 — Conserver un coût correct après une vente

**En tant qu'investisseur, je veux que les récompenses reçues après une vente conservent un PRU cohérent.**

Critères d'acceptation :

- Le nouveau prix moyen est calculé sur la quantité de staking encore détenue.
- Une quantité déjà vendue ne participe plus au nouveau PRU.
- Le coût réalisé de la vente passée ne change pas lorsqu'un versement ultérieur arrive.

## US-REV-007 — Migrer les anciennes récompenses

**En tant qu'utilisateur existant, je veux que mes anciens `FREE_RECEIPT` soient corrigés sans réimporter mon historique.**

Critères d'acceptation :

- Les anciennes transactions Trade Republic concernées passent de `BUY` à `DIVIDEND` lors de la migration de la base.
- Les empreintes et les données sources ne sont pas modifiées.
- Le portefeuille est recalculé automatiquement au prochain démarrage.

Traçabilité : `src-tauri/crates/portfolio-core/src/import/trade_republic.rs`, `src-tauri/crates/portfolio-core/src/engine.rs`, `src-tauri/src/db.rs`, `src/App.tsx`.
