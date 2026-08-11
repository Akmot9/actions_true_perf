# Feature — Performance du portefeuille

Statut : **Implémentée**

Objectif : présenter la valeur et la performance du portefeuille consolidé,
avec la possibilité de remonter jusqu'à chaque lot d'acquisition.

## US-PER-001 — Consolider les positions multi-comptes

**En tant qu'investisseur multi-courtiers, je veux une position unique par instrument afin d'obtenir une vue globale de mon exposition.**

Critères d'acceptation :

- Tous les lots ouverts d'un même instrument sont regroupés dans une position.
- Chaque lot conserve son compte actuel et son courtier d'origine.
- Les lots totalement consommés ne contribuent plus à la quantité ouverte.

## US-PER-002 — Calculer quantité, investi et PRU

**En tant qu'investisseur, je veux connaître ma quantité détenue, mon capital de référence et mon prix moyen.**

Critères d'acceptation :

- La quantité est la somme des quantités restantes des lots ouverts.
- L'investi additionne les coûts restants et les frais alloués.
- Le PRU est l'investi divisé par la quantité restante.
- Pour le staking, le capital de référence correspond à la valeur de réception, pas à un décaissement.

## US-PER-003 — Calculer la valeur et le P/L latent

**En tant qu'investisseur, je veux comparer la valeur actuelle au coût de référence afin de suivre la performance non réalisée.**

Critères d'acceptation :

- La valeur de marché est `quantité restante × dernier cours`.
- Le P/L latent est `valeur de marché − investi`.
- Le pourcentage est calculé uniquement lorsque l'investi est non nul.
- Sans cotation, les champs de valeur et de P/L restent indisponibles plutôt que d'être inventés.

## US-PER-004 — Voir le détail par lot

**En tant qu'investisseur, je veux déplier une position afin d'identifier les ordres qui gagnent ou perdent.**

Critères d'acceptation :

- Chaque ligne affiche date, courtier, quantité restante et initiale, prix, frais, investi, valeur et P/L.
- La durée de détention est calculée à partir de la date d'acquisition.
- Les lots sont triés chronologiquement.
- Un indicateur visuel colore chaque lot selon l'amplitude de sa performance.

## US-PER-005 — Suivre le P/L réalisé

**En tant qu'investisseur, je veux séparer les gains déjà matérialisés des variations encore latentes.**

Critères d'acceptation :

- Les cessions FIFO alimentent le P/L réalisé de l'instrument.
- Le total réalisé additionne toutes les cessions, y compris celles d'une position désormais fermée.
- Le P/L réalisé est présenté séparément du P/L latent.

## US-PER-006 — Suivre les dividendes

**En tant qu'investisseur, je veux voir les revenus de mes positions séparément de leur variation de cours.**

Critères d'acceptation :

- Les dividendes sont additionnés par instrument ouvert.
- La fiche de position affiche les dividendes perçus lorsqu'ils sont positifs.
- Le bandeau global conserve tous les dividendes, même après la fermeture complète d'une position.

## US-PER-007 — Lire les totaux du portefeuille

**En tant qu'investisseur, je veux une synthèse immédiate afin d'évaluer mon portefeuille sans ouvrir chaque position.**

Critères d'acceptation :

- Le bandeau affiche valeur, investi, P/L latent, P/L réalisé et dividendes.
- Les montants financiers sont calculés avec des nombres décimaux et arrondis à deux décimales pour la présentation.
- Les positions sont triées par valeur de marché, ou par investi lorsqu'aucun cours n'existe.

Traçabilité : `src-tauri/src/commands.rs`, `src-tauri/crates/portfolio-core/src/engine.rs`, `src/App.tsx`, `src/format.ts`.
