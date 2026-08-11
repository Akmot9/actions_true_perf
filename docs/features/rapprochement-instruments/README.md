# Feature — Rapprochement des instruments

Statut : **Implémentée**

Objectif : faire converger les références différentes d'un même actif vers une
position unique, quel que soit le courtier qui l'a nommé.

## US-INS-001 — Fusionner un instrument par ISIN

**En tant qu'investisseur multi-courtiers, je veux que le même titre identifié par son ISIN n'apparaisse qu'une fois.**

Critères d'acceptation :

- L'ISIN est utilisé en priorité pour retrouver un instrument existant.
- Un symbole ou un ISIN manquant est complété lors d'un import ultérieur.
- Air Liquide importé par libellé, symbole Yahoo ou ISIN converge vers la même position.

## US-INS-002 — Fusionner un instrument par symbole ou nom

**En tant qu'investisseur, je veux rapprocher les fichiers qui ne fournissent pas tous un ISIN afin d'éviter les doublons de position.**

Critères d'acceptation :

- À défaut d'ISIN, le symbole coté sert de clé de rapprochement.
- Un instrument précédemment créé par son nom peut être enrichi d'un symbole.
- Un instrument sans symbole reste accessible avec son nom et ses lots.

## US-INS-003 — Canonicaliser les symboles obsolètes

**En tant qu'investisseur, je veux que les anciens codes de marché restent rattachés au titre actuel afin de conserver l'historique et les cotations.**

Critères d'acceptation :

- `FDJ.PA`, `STLA.PA` et `STM.PA` sont convertis vers leurs symboles actuels.
- Les instruments et cotations déjà stockés sont migrés vers les symboles canoniques.
- La canonicalisation ne modifie pas l'empreinte historique d'une transaction importée.

## US-INS-004 — Utiliser des paires crypto en euros

**En tant qu'investisseur européen, je veux valoriser mes cryptomonnaies en euros afin de comparer leurs performances au reste du portefeuille.**

Critères d'acceptation :

- BTC, ETH et SOL deviennent respectivement `BTC-EUR`, `ETH-EUR` et `SOL-EUR`.
- Une crypto inconnue suit la convention `<SYMBOLE>-EUR`.
- Le nom fourni par le courtier est conservé lorsqu'aucun nom canonique n'est connu.

## US-INS-005 — Signaler un instrument non reconnu

**En tant qu'investisseur, je veux conserver un actif inconnu tout en sachant qu'il ne peut pas être coté automatiquement.**

Critères d'acceptation :

- L'opération et le lot sont conservés même sans symbole de marché.
- L'import affiche un avertissement avec le nom de l'instrument et la ligne source.
- La quantité et le PRU restent calculables, mais les champs dépendant d'une cotation affichent une valeur indisponible.

Traçabilité : `src-tauri/crates/portfolio-core/src/instruments.rs`, `src-tauri/src/db.rs`.
