# Feature — Transferts de titres

Statut : **Implémentée**

Objectif : déplacer les lots entre comptes sans transformer un transfert en
achat et sans perdre la date ou le coût d'origine.

## US-TRF-001 — Reconnaître un transfert entrant

**En tant qu'investisseur changeant de courtier, je veux que les titres reçus soient reconnus comme transférés et non achetés.**

Critères d'acceptation :

- Une ligne Bourse Direct `VIRT TITRES` devient `TRANSFER_IN`.
- Le transfert n'ajoute pas une seconde fois la quantité lorsque les lots d'origine sont connus.
- Le PRU indiqué par le courtier destinataire n'écrase pas le coût historique connu.

## US-TRF-002 — Préserver l'identité du lot transféré

**En tant qu'investisseur, je veux conserver la date et le prix d'achat d'origine après un changement de courtier.**

Critères d'acceptation :

- Le compte courant du lot devient le compte destinataire.
- Le courtier d'origine, la date d'acquisition, le prix et les frais restent inchangés.
- Un lot entièrement transféré conserve son identité.

## US-TRF-003 — Gérer un transfert partiel

**En tant qu'investisseur, je veux transférer une partie d'un lot sans perdre la traçabilité des deux portions.**

Critères d'acceptation :

- La portion déplacée devient un lot du compte destinataire.
- La portion d'origine reste sur son compte avec sa quantité réduite.
- Les frais sont répartis au prorata et les attributs d'origine sont conservés.

## US-TRF-004 — Représenter un historique d'achat manquant

**En tant qu'investisseur, je veux voir les titres transférés même si je n'ai pas encore importé leur historique d'achat.**

Critères d'acceptation :

- La quantité non expliquée crée un lot marqué `non rapproché`.
- Le coût de repli utilise le PRU communiqué par le courtier destinataire.
- La position affiche la quantité totale sans historique et un avertissement explicite.

## US-TRF-005 — Rapprocher automatiquement un historique importé plus tard

**En tant qu'investisseur, je veux que l'import tardif de mon ancien historique remplace automatiquement les estimations de transfert.**

Critères d'acceptation :

- Le rejeu chronologique replace les achats anciens avant le transfert même s'ils ont été importés après.
- Les vrais lots sont déplacés vers le compte destinataire.
- Le lot non rapproché disparaît totalement ou ne conserve que le reliquat encore inexpliqué.

Traçabilité : `src-tauri/crates/portfolio-core/src/engine.rs`, `src-tauri/crates/portfolio-core/src/import/bourse_direct.rs`.
