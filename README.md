# FIDO-LABS v3.0 (Web Edition)

## Plateforme Forensique Intégrée — Extraction, Analyse & Gestion Judiciaire

FIDO-LABS v3 est une suite d'investigation numérique médico-légale et de veille forensique pour terminaux mobiles et supports de stockage, conçue pour opérer en environnement web local et autonome.

### Nouveautés : Sauvegarde Automatique & Gestion Enrichie des Dossiers
- **💾 Sauvegarde Automatique après chaque Extraction** :
  - Dès qu'une extraction se termine (ADB logique, médias/galerie, BROM MediaTek, EDL Qualcomm, sauvegardes iOS/Android), la session complète est automatiquement enregistrée et scellée avec son empreinte **SHA-256**.
  - Si un dossier est actif, l'extraction lui est directement rattachée. Si aucun dossier n'est ouvert, un dossier de session horodaté est automatiquement généré pour garantir qu'aucune preuve n'est perdue.
  - Option **"Téléchargement auto vers PC"** : télécharge automatiquement le fichier de session `.json` prêt à être classé dans votre dossier `Documents/FIDO-LABS/Cases/`.
  - Bouton direct **"📂 Télécharger vers Mes Documents (.json)"** disponible immédiatement sur la fiche de résultat d'extraction.
- **📁 Gestion de Dossiers & Enrichissement Judiciaire** :
  - Renseignement complet des métadonnées de procédure : Référence Parquet, Nom de l'Affaire, Juridiction/Tribunal, Personne mise en cause / Suspect, N° de Scellé, Motif/Infraction, Statut de procédure (Ouvert, En cours, Sous scellé, Transmis Parquet, Archivé).
  - Bouton **"✏️ Modifier / Compléter informations"** pour actualiser ou enrichir un dossier à tout moment.
  - Historique des **Extractions & Sessions Rattachées** avec rechargement en 1 clic dans l'analyseur.
  - **Journal des Notes d'Enquête** : ajout d'observations d'enquêteurs, déclarations d'audition et indices recueillis.
  - **Import / Export de Sessions PC** : bouton **"📥 Importer session (.json)"** pour restaurer n'importe quel dossier ou session sauvegardé sur l'ordinateur.

### Modules Forensiques Intégrés
- **Extraction Mobile** : Logique ADB, identification automatique AFU/BFU, BROM MediaTek, EDL Qualcomm 9008, profil checkm8 iOS A5–A11, ALEAPP.
- **Supports Amovibles & Disques** : Imagerie bit-à-bit SD & disque (DD brut, E01) avec intégrité SHA-256 et file carving.
- **Inspecteur Hexadécimal & Signatures Magiques** : Vue 16 octets/ligne, détection d'en-têtes (SQLite, JPEG, PNG, ZIP, PDF), calcul d'entropie de Shannon et extraction de chaînes (Strings).
- **Forensique Navigateurs & Données Web** : Historique unifié (Chrome, Safari, Firefox, Edge), recherches web, téléchargements, cookies et jetons de session.
- **Métadonnées EXIF & Analyse Photo** : Analyse des boîtiers, optiques, détection de retouche logicielle (Photoshop) et géolocalisation GPS.
- **Contrôle d'Intégrité Hash & IOC** : Calculs MD5/SHA-256, base blanche NSRL (NIST), détection de malwares mobiles et scanner de règles YARA.
- **Procès-Verbal Judiciaire ISO/IEC 27037** : Rédaction officielle de PV d'examen technique scellé pour réquisition judiciaire avec option d'impression directe.

### Démarrage
```bash
npm install
npm run dev
```
Accessible sur `http://localhost:3000`.
