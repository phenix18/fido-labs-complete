# FIDO-LABS v3.0 (Web Edition)

## Plateforme Forensique Intégrée — Extraction & Analyse

FIDO-LABS v3 est une suite d'investigation numérique médico-légale et de veille forensique, conçue pour opérer en environnement web local et autonome.

### Modules & Capacités
- **Tableau de Bord & Dossiers** : Gestion des sessions et dossiers d'enquête scellés, suivi d'activité et statuts des appareils.
- **Extraction Mobile** :
  - Identification automatique de l'état (allumé, verrouillé, BFU, AFU, écran cassé)
  - Extraction logique ADB (contacts, SMS, appels, comptes, médias, inventaire complet)
  - Planification d'acquisition sur appareils verrouillés (BROM MediaTek, EDL Qualcomm 9008, profil checkm8 iOS A5–A11)
  - Déballage et parsing de sauvegardes Android (`.ab`) et sauvegardes iOS (iTunes / Finder)
  - Analyse d'artefacts ALEAPP
- **Supports Amovibles & Disques** :
  - Imagerie bit-à-bit carte SD et disque physique (DD brut, E01) avec intégrité SHA256
  - File Carving forensique (reconstitution de photos, vidéos, documents et bases SQLite)
- **Décodeur WhatsApp & SQLite** :
  - Déchiffrement de bases `msgstore.db.crypt14` / `.crypt15` avec clé légale
  - Restauration et décodage d'enregistrements effacés (freeblocks / unallocated space)
- **Money Flow & Analyse Financière** :
  - Détection automatique et extraction des flux Mobile Money (Orange Money, Wave, Moov, MTN MoMo)
  - Matrice des transactions et graphe relationnel des flux financiers
- **Timeline & Cartographie** :
  - Chronologie unifiée multi-sources (appels, SMS, WhatsApp, transactions, logs système)
  - Visualisation cartographique des coordonnées géographiques extraites
- **Graphes Relationnels & Data Analyse** :
  - Graphes de liens de communications et détection d'anomalies
  - Tableaux de bord analytiques avec diagrammes en camembert / anneau et répartition des preuves
- **Chaîne de Garde (CSM), Filtres & Rapports** :
  - Gestion rigoureuse de la chaîne de garde (Chain of Custody) horodatée
  - Filtres multi-critères et journal d'audit scellé
  - Modèles et génération de rapports d'expertise (PDF scellé SHA256)

### Démarrage
```bash
npm install
npm run dev
```
L'application démarre sur le port 3000 (`http://localhost:3000`).

### Éthique & Conformité
Usage strictement légal dans le cadre de réquisitions judiciaires, audits de sécurité et autorisations dûment accordées.
