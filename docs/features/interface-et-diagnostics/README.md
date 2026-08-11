# Feature — Interface et diagnostics

Statut : **Implémentée**

Objectif : rendre les calculs vérifiables et les anomalies visibles dans une
interface de bureau compacte.

## US-UI-001 — Voir immédiatement les positions

**En tant qu'investisseur, je veux une liste synthétique de mes positions afin de repérer rapidement leur poids et leur performance.**

Critères d'acceptation :

- Chaque position affiche instrument, symbole, nombre de lots, quantité, cours, valeur, PRU et P/L.
- Un chevron permet d'ouvrir ou de fermer le détail.
- Une position sans historique complet affiche la quantité concernée dans un badge.

## US-UI-002 — Parcourir visuellement les lots

**En tant qu'investisseur, je veux un aperçu des performances par lot avant d'ouvrir le détail.**

Critères d'acceptation :

- Une bande de cellules représente les lots de la position.
- La couleur indique gain, perte ou performance indisponible, avec plusieurs intensités.
- Le survol donne la date ou le type staking, la quantité, le prix de référence et le pourcentage.

## US-UI-003 — Comprendre un portefeuille vide ou en erreur

**En tant que nouvel utilisateur, je veux savoir comment commencer ou pourquoi le portefeuille ne charge pas.**

Critères d'acceptation :

- Sans position, l'interface invite à importer un relevé compatible.
- Le texte précise que les transferts reconnus ne deviennent pas de faux achats.
- Une erreur de chargement du portefeuille est affichée dans un état dédié.

## US-UI-004 — Consulter les avertissements métier

**En tant qu'investisseur, je veux inspecter les incohérences détectées sans encombrer la vue principale.**

Critères d'acceptation :

- Le nombre d'avertissements du moteur est visible.
- L'utilisateur peut afficher ou masquer leur détail.
- Les lots non rapprochés expliquent leur origine et la manière de les résoudre.

## US-UI-005 — Utiliser l'application au clavier

**En tant qu'utilisateur, je veux fermer ou valider une édition de façon prévisible afin de travailler efficacement.**

Critères d'acceptation :

- La touche Échap ferme le dialogue sans enregistrer.
- La soumission du formulaire déclenche l'enregistrement lorsque les valeurs sont valides.
- Les boutons exposent des libellés accessibles et les états développés sont signalés.

## US-UI-006 — Lire les montants de manière homogène

**En tant qu'investisseur francophone, je veux des dates, nombres et montants cohérents afin d'éviter les ambiguïtés.**

Critères d'acceptation :

- Les montants sont présentés en euros et les pourcentages avec leur signe.
- Les valeurs absentes sont affichées par un tiret plutôt que par un faux zéro.
- Les gains et pertes utilisent des styles visuels distincts.

Traçabilité : `src/App.tsx`, `src/App.css`, `src/format.ts`.
