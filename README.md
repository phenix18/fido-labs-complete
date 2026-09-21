# FIDO-LABS v3.0
## Forensique Intégrée — Extracteur + Analyseur

### Modules
- **Extraction**: Mobile (ADB/BROM/EDL), SD Card, Disque (DD/E01)
- **Décodeurs**: WhatsApp, Orange Money/Moov/Wave/MTN, File Carving
- **Analyse**: Timeline, Money Flow, Graphe de Liens (i2-style), Cartographie
- **Gestion**: CSM (Chain of Custody), Calendrier, Filtres avancés, Rapports généraux/spécifiques
- **Licence**: Standard / Pro / Site avec contrôle d'accès par module

### Build
- UI: Vanilla HTML/CSS/JS embarqué (local offline)
- Backend: Rust + Tauri 1.6
- Base de données locale SQLite (rusqlite)

### Installation
- Installeur Inno Setup pour Windows
- Build Linux: `bash build.sh`
- Dev: `cd src-tauri && cargo tauri dev`

### Éthique
Usage légal uniquement avec autorisation judiciaire. Chaque extraction est loggée.
