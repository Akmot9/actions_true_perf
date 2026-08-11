# Catalogue des features et user stories

Ce catalogue décrit le comportement actuellement implémenté dans l'application
« Suivi des ordres ». Une user story n'est marquée comme implémentée que si le
parcours correspondant existe dans le code actuel.

## Features

| Feature | Périmètre |
| --- | --- |
| [Données de démonstration](./donnees-de-demonstration/) | Portefeuille d'exemple au premier lancement et suppression sans impact sur les imports réels |
| [Import multi-courtiers](./import-multi-courtiers/) | Détection et import des CSV Bourse Direct, Trade Republic et Yahoo/BoursoBank |
| [Rapprochement des instruments](./rapprochement-instruments/) | Fusion des titres entre courtiers, ISIN, symboles et cryptomonnaies |
| [Déduplication et persistance](./deduplication-persistance/) | Imports idempotents, stockage local et migrations |
| [Moteur de lots](./moteur-de-lots/) | Lots d'acquisition, FIFO, frais, ventes, splits et avertissements |
| [Transferts de titres](./transferts-de-titres/) | Déplacement des lots, conservation de l'historique et rapprochement tardif |
| [Staking et dividendes](./staking-et-dividendes/) | Dividendes espèces, récompenses en nature et regroupement du staking |
| [Performance du portefeuille](./performance-portefeuille/) | Positions, PRU, valeur, P/L latent et réalisé, totaux |
| [Cotations de marché](./cotations-de-marche/) | Actualisation Yahoo, cache et fonctionnement hors ligne |
| [Édition des ordres](./edition-des-ordres/) | Correction manuelle, validation et restauration des valeurs importées |
| [Saisie manuelle](./saisie-manuelle/) | Ajout, modification et suppression d'achats, ventes, dividendes et staking |
| [Interface et diagnostics](./interface-et-diagnostics/) | Navigation, détails par lot, rapports, erreurs et avertissements |

## Conventions

- `US-XXX-NNN` est l'identifiant stable de la user story.
- Toutes les stories de ce catalogue ont le statut **Implémentée**.
- Les critères d'acceptation décrivent le comportement observable attendu.
- Les règles techniques qui protègent directement les données utilisateur sont
  documentées comme stories, même lorsqu'elles sont automatiques.

## Hors périmètre actuel

Les fonctions suivantes ne sont pas encore implémentées et ne constituent donc
pas des stories livrées : suppression individuelle d'une transaction importée,
gestion de plusieurs comptes chez un même courtier, choix
manuel du fournisseur de cours, synchronisation cloud et export du portefeuille.
