# Déployer l’application iOS sans Mac

Le build, la signature et l’envoi vers TestFlight sont exécutés par le workflow
GitHub Actions [`ios-testflight.yml`](../.github/workflows/ios-testflight.yml)
sur un runner `macos-26`. Aucun Mac personnel n’est nécessaire.

Les opérations de compte restent à effectuer une fois dans les portails Apple et
GitHub : accepter les contrats, choisir l’identifiant définitif, créer les
identifiants de signature et renseigner la fiche App Store.

## 1. Choisir l’identifiant définitif

Le `identifier` actuel du projet, `com.actions_true_perf.app`, contient des
underscores. Apple n’accepte que les lettres, chiffres, tirets et points dans un
Bundle ID.

Choisir un identifiant unique au format DNS inversé, par exemple :

```text
com.akmot9.actionstrueperf
```

Ce choix devient difficile à changer après le premier build envoyé. Le workflow
l’obtient depuis la variable GitHub `IOS_BUNDLE_ID` et remplace uniquement la
configuration iOS pendant le build.

## 2. Créer l’application chez Apple

1. Dans Apple Developer, ouvrir **Certificates, Identifiers & Profiles**.
2. Dans **Identifiers**, créer un **App ID explicite** avec le Bundle ID choisi.
3. Dans App Store Connect, vérifier que le titulaire du compte a accepté les
   derniers contrats dans **Business**.
4. Dans **Apps**, choisir **+ → New App**, plateforme **iOS**.
5. Sélectionner exactement le même Bundle ID et définir un SKU interne, par
   exemple `actions-true-perf-ios`.

Le nom public de l’app et le Bundle ID sont deux valeurs différentes. Le nom
peut être `Suivi des ordres`, `True Performance` ou un autre nom disponible.

## 3. Créer la clé App Store Connect

Dans **App Store Connect → Users and Access → Integrations → App Store Connect
API**, créer une clé avec le rôle **App Manager** puis télécharger le fichier
`AuthKey_<KEY_ID>.p8`. Apple ne permet de le télécharger qu’une fois.

Noter également :

- l’**Issuer ID** ;
- le **Key ID** ;
- le **Team ID**, visible dans les informations d’adhésion Apple Developer.

## 4. Créer le certificat de distribution sans Mac

La clé privée et la demande de certificat peuvent être produites sous Linux :

```sh
mkdir apple-signing
cd apple-signing

openssl genrsa -out ios-distribution.key 2048
openssl req -new \
  -key ios-distribution.key \
  -out CertificateSigningRequest.certSigningRequest
```

Dans Apple Developer :

1. Ouvrir **Certificates** puis **+**.
2. Choisir **Apple Distribution**.
3. Envoyer `CertificateSigningRequest.certSigningRequest`.
4. Télécharger le certificat et le placer sous le nom `ios-distribution.cer`
   dans le dossier créé ci-dessus.

Créer ensuite le fichier P12 protégé par un mot de passe :

```sh
openssl x509 \
  -inform DER \
  -in ios-distribution.cer \
  -out ios-distribution.pem

openssl pkcs12 -export \
  -inkey ios-distribution.key \
  -in ios-distribution.pem \
  -out ios-distribution.p12 \
  -name "Apple Distribution"

base64 -w 0 ios-distribution.p12 > ios-distribution.p12.base64
```

Ne jamais ajouter le dossier `apple-signing`, le P12, la clé privée ou la clé
API Apple au dépôt Git.

## 5. Créer le profil App Store

Dans **Apple Developer → Profiles → +** :

1. choisir le profil de distribution **App Store Connect** ;
2. sélectionner l’App ID créé à l’étape 2 ;
3. sélectionner le certificat **Apple Distribution** ;
4. générer le profil.

Le workflow le téléchargera automatiquement grâce à la clé API. Il n’est pas
nécessaire de stocker le fichier `.mobileprovision` dans GitHub.

## 6. Configurer le dépôt GitHub

Dans **Settings → Secrets and variables → Actions**, créer les variables :

| Variable              | Valeur                                       |
| --------------------- | -------------------------------------------- |
| `IOS_BUNDLE_ID`       | Le Bundle ID définitif, sans underscore      |
| `IOS_PRODUCT_NAME`    | Nom installé, par exemple `Suivi des ordres` |
| `APPLE_TEAM_ID`       | Team ID Apple de 10 caractères               |
| `APPSTORE_ISSUER_ID`  | Issuer ID de la clé App Store Connect        |
| `APPSTORE_API_KEY_ID` | Key ID de la clé App Store Connect           |

Créer ensuite les secrets :

| Secret                              | Valeur                                   |
| ----------------------------------- | ---------------------------------------- |
| `APPSTORE_API_PRIVATE_KEY`          | Contenu complet de `AuthKey_<KEY_ID>.p8` |
| `APPSTORE_CERTIFICATES_FILE_BASE64` | Contenu de `ios-distribution.p12.base64` |
| `APPSTORE_CERTIFICATES_PASSWORD`    | Mot de passe choisi lors de l’export P12 |

Avec GitHub CLI connecté au bon compte :

```sh
gh variable set IOS_BUNDLE_ID --body "com.akmot9.actionstrueperf"
gh variable set IOS_PRODUCT_NAME --body "Suivi des ordres"
gh variable set APPLE_TEAM_ID --body "VOTRE_TEAM_ID"
gh variable set APPSTORE_ISSUER_ID --body "VOTRE_ISSUER_ID"
gh variable set APPSTORE_API_KEY_ID --body "VOTRE_KEY_ID"

gh secret set APPSTORE_API_PRIVATE_KEY < AuthKey_VOTRE_KEY_ID.p8
gh secret set APPSTORE_CERTIFICATES_FILE_BASE64 < ios-distribution.p12.base64
gh secret set APPSTORE_CERTIFICATES_PASSWORD
```

La dernière commande demande le mot de passe sans l’afficher.

## 7. Lancer le build

1. Pousser le workflow sur GitHub.
2. Ouvrir l’onglet **Actions** du dépôt.
3. Sélectionner **iOS · TestFlight**.
4. Choisir **Run workflow**.
5. Laisser **Envoyer le build vers TestFlight** activé.

Le workflow :

1. utilise Xcode 26 et le SDK iOS 26 ;
2. importe temporairement le certificat dans le trousseau du runner ;
3. télécharge le profil de distribution ;
4. génère le projet iOS Tauri ;
5. construit et signe l’IPA ;
6. conserve l’IPA 14 jours dans les artefacts GitHub ;
7. envoie le build dans TestFlight.

Un tag Git au format `ios-v*`, par exemple `ios-v0.1.0`, déclenche également un
build avec envoi automatique vers TestFlight.

## 8. Passer de TestFlight à l’App Store

Une fois le build traité par Apple :

1. le tester avec **TestFlight** ;
2. compléter la description, les mots-clés, la catégorie et l’âge conseillé ;
3. fournir entre une et dix captures iPhone, idéalement au format 6,9 pouces ;
4. publier une politique de confidentialité accessible par URL et depuis l’app ;
5. déclarer précisément les pratiques de collecte de données ;
6. répondre aux questions sur le chiffrement et les droits sur les contenus ;
7. remplir **App Review Information → Notes** avec le texte des
   [notes de revue Apple](./notes-de-revue-apple.md) et joindre la vidéo de
   démonstration qui y est décrite ;
8. choisir le build dans la version iOS puis **Add for Review** et **Submit for
   Review**.

L’étape 7 n’est pas facultative : la soumission du 14 août 2026 a été refusée au
titre de la guideline 2.1 — *Information Needed* parce que ce champ était vide.

Pour un iPhone 17 Pro Max, Apple accepte notamment les captures portrait
`1320 × 2868`, `1290 × 2796` ou `1260 × 2736` pixels.

## Points restant à préparer avant la revue Apple

- une URL publique de politique de confidentialité ;
- un lien vers cette politique directement dans l’application ;
- les captures App Store finales ;
- une description et les informations de contact demandées par la revue ;
- la vérification de l’import CSV et du stockage local sur un véritable iPhone
  via TestFlight ;
- l’enregistrement vidéo sur iPhone physique, et la liste des modèles et
  versions d’iOS testés, exigés par la revue — voir les
  [notes de revue Apple](./notes-de-revue-apple.md).
