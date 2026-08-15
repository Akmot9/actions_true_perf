# Notes pour la revue Apple

Apple demande, dans **App Store Connect → App Review Information → Notes**, une
description complète de l’app avant chaque soumission. Une fiche incomplète
entraîne un refus au titre de la guideline 2.1 — *Information Needed*, comme lors
de la soumission du 14 août 2026.

Ce document contient le texte de référence à recopier dans le champ **Notes**, à
réutiliser tel quel pour chaque nouvelle version. Seuls les modèles d’appareils
et les versions d’iOS testés changent d’une soumission à l’autre.

## Texte à coller dans le champ Notes

> Hello,
>
> **Summary: TruePerf has no user account, no login, no server backend, no
> in-app purchase or subscription, no user-generated content, no advertising, no
> analytics and no tracking. It requests no permission of any kind. All data
> stays in a local SQLite database on the device.**
>
> **1. Screen recording**
>
> A screen recording captured on a physical iPhone 17 Pro Max running iOS
> 26.6, starting from app launch and covering the full user flow, is attached.
>
> The app has none of the flows listed in the review guidelines: no account
> registration, login or deletion; no paid content, purchase or subscription; no
> user-generated content, reporting or blocking; and no prompt for location,
> contacts, camera, photos, notifications or App Tracking Transparency. The only
> system UI shown in the recording is the standard iOS document picker, opened by
> the user when they choose to import their own CSV file.
>
> **2. Devices and operating systems tested**
>
> - iPhone 17 Pro Max — iOS 26.6 (physical device, build installed via
>   TestFlight)
>
> The app is iPhone-only (`TARGETED_DEVICE_FAMILY = 1`).
>
> **3. Functions, target audience, problem solved**
>
> TruePerf is an offline personal investment tracker. Its users are individual
> retail investors in France who hold securities across several brokers (for
> example a PEA at one broker and a CTO at another).
>
> Problem: broker statements show a single blended "performance" figure that
> mixes together purchase fees, partial sales, dividends, staking rewards and
> transfers between brokers. The investor cannot tell where the gain or loss
> actually comes from.
>
> What the app does: the user imports the CSV statements exported from their own
> brokers, or types transactions in manually. A deterministic engine written in
> Rust rebuilds the portfolio lot by lot: every purchase is one lot, sales are
> matched FIFO, fees are allocated pro rata, transfers between brokers keep the
> original lot history instead of creating a fake purchase, and staking rewards
> are recorded as income in kind rather than as a purchase. The app then
> displays, separately: market value, invested capital, unrealised P/L, realised
> P/L and income received. Every total can be expanded down to the individual
> orders that explain it.
>
> The app is an informational tracking tool only. It does not execute trades,
> does not connect to any brokerage account, does not hold funds and does not
> provide financial, investment or tax advice. This is stated in the app
> description.
>
> **4. Setting up and accessing the main features — no credentials required**
>
> No login and no sample file are needed to review the app.
>
> On first launch the app automatically creates a small clearly-labelled demo
> portfolio (equities, dividends, a Solana position and several staking rewards),
> so every feature is immediately visible and testable. Steps:
>
> 1. Launch the app. The portfolio, the totals banner and the positions list are
>    displayed immediately with the demo data.
> 2. Tap a position to expand it and see its individual lots, cost basis, fees
>    and P/L breakdown.
> 3. Tap a lot to edit an order; the originally imported version is preserved and
>    can be restored.
> 4. Use the "+" action to add a transaction manually (purchase, sale, dividend,
>    staking reward, transfer).
> 5. Use the refresh action to update market quotes.
> 6. Use the "delete demo data" action (two-tap confirmation) to remove the demo
>    portfolio. It does not come back.
> 7. Optional: use the import action to select a CSV file through the iOS
>    document picker. Supported formats are detected automatically: Trade
>    Republic, Bourse Direct, and Yahoo/BoursoBank `portfolio.csv`. Imports are
>    idempotent — importing the same statement twice does not duplicate
>    transactions.
>
> **5. External services, tools and platforms used**
>
> One single external service:
>
> - Yahoo Finance public quotation endpoint
>   (`https://query1.finance.yahoo.com/v8/finance/chart/<symbol>`), over HTTPS,
>   used only to fetch the latest price of the instruments the user tracks. Only
>   the ticker symbol is sent (for example `AI.PA`). No personal information, no
>   quantity, no amount and no identifier is transmitted. Responses are cached
>   locally so the app remains usable offline; if a quote is unavailable the app
>   says so instead of inventing a value.
>
> There is no authentication service, no payment processor, no AI service, no
> analytics SDK, no advertising SDK and no backend operated by us. The app is
> built with Tauri 2, SolidJS and Rust, and stores data in an embedded SQLite
> database on the device.
>
> **6. Regional differences**
>
> There are none. The app behaves identically in every region: same features,
> same content, same data sources. The interface is in French and amounts are
> displayed in euros, which reflects the target audience (French retail
> investors), but no feature is enabled, disabled or altered based on the user's
> country or storefront.
>
> **7. Regulated industry / third-party protected material**
>
> The app does not operate in a regulated industry and requires no licence or
> authorisation. It is a personal bookkeeping tool: it only reads statement files
> that the user already owns and exported from their own broker, and it performs
> arithmetic on them locally. It is not a broker, not a bank, not a payment
> service, not a portfolio management service, and it does not provide investment
> or tax advice — it is explicitly presented as an informational tool.
>
> Regarding third-party material: the app displays no third-party protected
> content. Market quotes are retrieved from Yahoo Finance's publicly accessible
> endpoint and are shown only to the user who requested them, for their own
> holdings; they are not redistributed, republished or resold.

## Vidéo de démonstration

Apple exige un enregistrement réalisé sur un **iPhone physique** exécutant la
dernière version d’iOS, commençant par le lancement de l’app. Enregistrement
d’écran iOS, environ 60 à 90 secondes, sur le build TestFlight soumis.

1. Écran d’accueil iOS, appui sur l’icône TruePerf — le lancement doit être
   visible, l’enregistrement ne doit pas commencer app déjà ouverte.
2. Portefeuille de démonstration : bandeau des totaux et liste des positions.
3. Défilement, pour montrer le bandeau collant compact.
4. Ouvrir une position : lots, coût de revient, frais, P/L.
5. Ouvrir un lot : édition d’un ordre, puis annulation ou restauration.
6. Ajouter une transaction manuelle, par exemple un dividende, et la voir
   apparaître dans les totaux.
7. Rafraîchir les cotations et montrer la mise à jour des montants.
8. Import CSV : sélecteur de fichiers iOS, choix d’un fichier, rapport d’import
   avec nouvelles lignes et doublons.
9. Suppression des données de démonstration, avec le double appui de
   confirmation.
10. Fin sur le portefeuille réel ou vide.

## Vérifications avant chaque soumission

- Renseigner la version d’iOS réellement installée sur l’iPhone de test, relevée
  dans **Réglages → Général → Informations → Version du logiciel**. Une réponse
  vague sur ce point relance un refus.
- Ne lister que les appareils sur lesquels le build a effectivement été lancé.
  Le build est produit en CI sans Mac : il n’y a pas de test sur simulateur à
  déclarer tant qu’aucun n’a été fait.
- Joindre la vidéo à la soumission ou à la réponse à l’équipe de revue.
- Recopier le texte ci-dessus dans **App Review Information → Notes**.
- Vérifier que la **Privacy Policy URL** de la fiche pointe vers une version
  publiée de [`PRIVACY.md`](../PRIVACY.md).
- Vérifier que les captures App Store montrent l’app en usage réel, et non
  l’écran de démarrage ou le titre seul — guideline 2.3.3.

## Points de vigilance connus

- **Cotations Yahoo.** L’endpoint utilisé n’est pas documenté publiquement par
  Yahoo. La formulation retenue — accès public, affichage au seul utilisateur qui
  en fait la demande, aucune redistribution — est exacte. Si la revue insiste au
  titre du point 7, le repli propre est un fournisseur de cotations dont les
  conditions d’utilisation sont explicites.
- **Politique de confidentialité.** L’app n’expose aucun lien vers la politique
  depuis son interface. Ce n’est pas bloquant pour la guideline 2.1, mais l’URL
  doit impérativement figurer dans la fiche App Store Connect.
