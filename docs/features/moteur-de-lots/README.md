# Feature — Moteur de lots

Statut : **Implémentée**

Objectif : reconstruire de manière déterministe les quantités, coûts et
performances à partir de l'historique complet des transactions.

## US-LOT-001 — Conserver un lot par ordre d'achat

**En tant qu'investisseur, je veux voir séparément chaque ordre d'achat afin de mesurer sa performance propre.**

Critères d'acceptation :

- Chaque achat complet crée un lot avec son compte, son courtier, sa date, sa quantité, son prix et ses frais.
- Deux achats du même instrument restent deux lots distincts.
- Un lot conserve un identifiant stable pendant un même rejeu de l'historique.

## US-LOT-002 — Rejouer l'historique de façon déterministe

**En tant qu'investisseur, je veux obtenir le même portefeuille indépendamment de l'ordre dans lequel j'importe mes fichiers.**

Critères d'acceptation :

- Les transactions sont triées par date, priorité métier et identifiant avant calcul.
- Les dates inconnues sont considérées comme les plus anciennes.
- À transactions identiques, le moteur produit les mêmes lots, cessions et avertissements.

## US-LOT-003 — Consommer les lots en FIFO lors d'une vente

**En tant qu'investisseur, je veux que les ventes consomment d'abord mes lots les plus anciens afin d'obtenir un coût réalisé cohérent.**

Critères d'acceptation :

- Une vente ne consomme que les lots ouverts du même instrument et du même compte.
- Une vente partielle réduit la quantité restante sans supprimer l'identité du lot.
- Une vente couvrant plusieurs lots produit une cession calculée pour chaque lot consommé.

## US-LOT-004 — Calculer les frais au prorata

**En tant qu'investisseur, je veux répartir correctement les frais lors d'une vente partielle afin de ne pas fausser le coût restant.**

Critères d'acceptation :

- Les frais d'achat sont alloués à la quantité vendue au prorata de la quantité initiale.
- Les frais de vente sont répartis au prorata lorsque plusieurs lots sont consommés.
- Le coût investi du reliquat conserve la part de frais qui lui revient.

## US-LOT-005 — Calculer le résultat réalisé

**En tant qu'investisseur, je veux connaître mon gain ou ma perte sur les quantités vendues.**

Critères d'acceptation :

- Le produit net d'une cession retranche les frais de vente alloués.
- Le coût de revient inclut le prix d'acquisition et les frais d'achat alloués.
- Le P/L réalisé est la différence entre produit net et coût de revient.

## US-LOT-006 — Appliquer un split sans modifier le coût total

**En tant qu'investisseur, je veux que les divisions d'actions ajustent mes quantités sans créer de gain ou de perte artificiel.**

Critères d'acceptation :

- La quantité initiale et la quantité restante sont multipliées par le ratio.
- Le coût unitaire est divisé par le même ratio.
- Un ratio nul est ignoré avec un avertissement.

## US-LOT-007 — Signaler un historique incohérent

**En tant qu'investisseur, je veux être averti lorsqu'une opération ne peut pas être entièrement expliquée par les lots disponibles.**

Critères d'acceptation :

- Un achat, une vente, un transfert ou un split incomplet est signalé et n'est pas inventé silencieusement.
- Une vente supérieure à la quantité disponible laisse les lots à zéro et indique la quantité non couverte.
- Les avertissements du moteur sont renvoyés avec le portefeuille.

Traçabilité : `src-tauri/crates/portfolio-core/src/engine.rs`.
