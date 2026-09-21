# FIDO-LABS - Architecture Investigateur

## Etats Téléphone Gérés
1. ON - Logical ADB + Agent APK
2. OFF - BROM (MTK) VID 0e8d / EDL 9008 (Qualcomm) VID 05c6 - Nécessite test point + cable EDL
3. LOCKED - NE PAS BYPASS sans autorisation judiciaire. Méthode légale: demande code, mandat, ou utilisation outil certifié avec log.
4. BROKEN_SCREEN - OTG souris + activation ADB si déjà autorisé, sinon ISP/JTAG (lecture directe puce)

## Modules Auto Plug
- Service Rust qui écoute USB hotplug (rusb)
- Dès VID/PID détecté -> propose profil
- Ex: Branchement Tecno éteint -> Détecte 0e8d:0003 -> Propose "MTK BROM Dump"

## Sécurité Juridique
- Chaque extraction loggue SHA256 + horodatage + opérateur
- Rapport chain of custody obligatoire
