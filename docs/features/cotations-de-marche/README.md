# Feature — Cotations de marché

Statut : **Implémentée**

Objectif : valoriser les positions ouvertes avec un cours récent tout en
restant utilisable lorsque le fournisseur est indisponible.

## US-COT-001 — Actualiser les cours au lancement

**En tant qu'investisseur, je veux que les cours soient rafraîchis à l'ouverture afin de voir une valeur de portefeuille récente.**

Critères d'acceptation :

- L'interface déclenche automatiquement une actualisation au montage de l'application.
- Seuls les symboles des lots encore ouverts sont demandés.
- Un même symbole n'est interrogé qu'une fois par actualisation.

## US-COT-002 — Actualiser les cours manuellement

**En tant qu'investisseur, je veux pouvoir demander un nouveau cours à tout moment.**

Critères d'acceptation :

- Le bouton `Actualiser les cours` lance une nouvelle requête.
- Le bouton indique qu'une actualisation est en cours et empêche un second clic simultané.
- Le portefeuille est recalculé après réception des résultats.

## US-COT-003 — Valoriser les actifs en euros

**En tant qu'investisseur, je veux des cotations compatibles avec la devise du portefeuille afin de comparer mes positions.**

Critères d'acceptation :

- Les symboles Yahoo des actions et ETF utilisent, lorsque prévu, une place cotée en euros.
- Les cryptomonnaies utilisent une paire `-EUR`.
- Le cours Yahoo reçu est converti en décimal et arrondi à quatre décimales avant stockage.

## US-COT-004 — Conserver un cache de cours

**En tant qu'investisseur, je veux continuer à voir la dernière valorisation connue lorsque je suis hors ligne.**

Critères d'acceptation :

- Chaque cours réussi est stocké avec son fournisseur et sa date de récupération.
- Un cours trouvé dans `portfolio.csv` peut initialiser le cache.
- Une erreur réseau ne supprime ni ne remplace le dernier cours valide.

## US-COT-005 — Comprendre une erreur de cotation

**En tant qu'investisseur, je veux savoir quels instruments n'ont pas pu être actualisés sans bloquer le reste du portefeuille.**

Critères d'acceptation :

- Une erreur est rapportée séparément pour chaque symbole concerné.
- Les symboles réussis sont mis à jour même si d'autres échouent.
- Les requêtes ont un délai maximal et les erreurs HTTP ou réponses sans cours sont explicites.

Traçabilité : `src-tauri/src/market.rs`, `src-tauri/src/commands.rs`, `src/App.tsx`.
