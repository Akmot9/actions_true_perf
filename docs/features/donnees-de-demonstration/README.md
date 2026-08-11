# Feature — Données de démonstration

Statut : **Implémentée**

Objectif : rendre l'application compréhensible dès son premier écran, sans
confondre les exemples intégrés avec les futures données personnelles.

## US-DEM-001 — Découvrir un portefeuille rempli au premier lancement

**En tant que nouvel utilisateur, je veux voir immédiatement des positions et des lots afin de comprendre ce que l'application peut m'apporter avant d'importer mes relevés.**

Critères d'acceptation :

- Une base réellement vierge reçoit automatiquement le jeu de découverte.
- Le jeu reprend les exemples fournis : achats Air Liquide et TotalEnergies,
  dividendes, achat Solana et sept récompenses de staking Trade Republic.
- Les cotations embarquées rendent immédiatement visibles valeur, PRU, gains
  et pertes, même hors ligne.
- Une base contenant déjà des transactions réelles ne reçoit aucun exemple.

## US-DEM-002 — Identifier clairement les exemples

**En tant qu'utilisateur, je veux savoir que les premières lignes ne sont pas mes données afin de ne pas les prendre pour un portefeuille réel.**

Critères d'acceptation :

- Un bandeau `Mode découverte` reste visible tant qu'au moins une transaction
  du jeu intégré existe.
- Le bandeau résume les catégories d'exemples affichées.
- Les exemples restent des transactions normales pour exercer le détail des
  positions, l'édition d'un achat et le regroupement du staking.

## US-DEM-003 — Supprimer les exemples quand je le souhaite

**En tant qu'utilisateur prêt à importer mes relevés, je veux effacer tous les exemples en une action afin de repartir avec un portefeuille vide.**

Critères d'acceptation :

- Le bandeau propose l'action `Supprimer les exemples`.
- Une confirmation précise que les imports personnels ne seront pas touchés.
- Seules les transactions, le fichier source et les cotations propres au jeu
  intégré sont supprimés.
- Les comptes et instruments devenus orphelins sont nettoyés.

## US-DEM-004 — Ne jamais recréer les exemples après suppression

**En tant qu'utilisateur, je veux que mon choix de supprimer la démonstration soit mémorisé afin qu'elle ne réapparaisse pas au prochain démarrage.**

Critères d'acceptation :

- L'initialisation du jeu de découverte est mémorisée dans la base locale.
- Une suppression volontaire conserve ce marqueur.
- Un redémarrage ou une mise à jour ne réinsère pas les exemples.
- Si des imports personnels partagent le même compte ou instrument, ils sont
  conservés lors de la suppression du jeu intégré.

Traçabilité : `src-tauri/src/db.rs`, `src-tauri/src/lib.rs`,
`src-tauri/src/commands.rs`, `src/App.tsx`, `src/App.css`.
