# Feature — Déduplication et persistance

Statut : **Implémentée**

Objectif : rendre les imports répétables et conserver localement les données
sans détériorer les corrections ou l'historique.

## US-DAT-001 — Réimporter un fichier sans doublons

**En tant qu'investisseur, je veux pouvoir réimporter le même relevé sans dupliquer mes opérations.**

Critères d'acceptation :

- Chaque opération reçoit une empreinte stable.
- Une empreinte déjà présente empêche l'insertion d'une seconde transaction.
- Le rapport distingue les nouvelles opérations des opérations déjà importées.

## US-DAT-002 — Préserver deux opérations réellement identiques

**En tant qu'investisseur, je veux conserver deux ordres identiques exécutés le même jour afin de ne pas perdre d'historique.**

Critères d'acceptation :

- Un index d'occurrence différencie deux lignes identiques dans un même fichier.
- Le même fichier recalculé produit les mêmes empreintes dans le même ordre.
- L'identifiant de transaction Trade Republic est utilisé lorsqu'il est fourni.

## US-DAT-003 — Conserver les données localement

**En tant qu'utilisateur d'une application de bureau, je veux retrouver mon portefeuille après un redémarrage sans dépendre d'un service cloud.**

Critères d'acceptation :

- Les comptes, instruments, transactions, imports et cotations sont stockés dans une base SQLite locale.
- Le fichier source brut et sa date d'import sont archivés dans la base.
- L'application rouvre automatiquement la même base dans son dossier de données Tauri.

## US-DAT-004 — Faire évoluer la base sans perdre l'historique

**En tant qu'investisseur, je veux que les mises à jour de l'application préservent mes données existantes.**

Critères d'acceptation :

- Les migrations sont appliquées séquentiellement et mémorisées par version.
- Les anciennes références de symboles sont canonicalisées automatiquement.
- Les anciennes récompenses Trade Republic `FREE_RECEIPT` sont reclassées en dividendes de staking.

## US-DAT-005 — Protéger une correction manuelle lors d'un réimport

**En tant qu'investisseur, je veux qu'un réimport ne remplace pas une correction que j'ai volontairement apportée.**

Critères d'acceptation :

- La correction conserve l'empreinte originale de la transaction.
- La transaction corrigée est reconnue comme doublon lors d'un réimport.
- Les valeurs originales restent archivées pour une restauration ultérieure.

Traçabilité : `src-tauri/src/db.rs`, `src-tauri/crates/portfolio-core/src/import/mod.rs`.
