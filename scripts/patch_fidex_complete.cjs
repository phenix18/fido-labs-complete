const fs = require('fs');

console.log("Reading index.html for complete FidEx implementation...");
let html = fs.readFileSync('index.html', 'utf8');

// 1. UPDATE PAGE-CARTO MARKUP WITH COMPLETE BURKINA FASO GEOLOCATION ENGINE
const oldCartoMarkupRegex = /<!-- CARTOGRAPHIE -->[\s\S]*?<div id="page-carto" class="page">[\s\S]*?<\/div>\s*<!-- GRAPHES -->/;

const newCartoMarkup = `<!-- CARTOGRAPHIE BURKINA FASO & GÉOLOCALISATION FORENSIQUE -->
    <div id="page-carto" class="page">
      <div class="module-header-bar">
        <div class="module-header-title">
          <span style="font-size:24px">🗺️</span>
          <div>
            <h2 style="margin:0">Cartographie & Géolocalisation Forensique (Burkina Faso)</h2>
            <div style="font-size:11.5px;color:var(--muted)">Corrélation spatio-temporelle : bornages BTS, transactions Mobile Money et coordonnées GPS par ville et quartier</div>
          </div>
        </div>
        <div class="module-actions">
          <select id="carto-view-mode" onchange="changeBurkinaMapMode(this.value)" style="background:var(--surface2);color:var(--text);border:1px solid var(--border);border-radius:6px;padding:4px 8px;font-size:11.5px">
            <option value="national">🇧🇫 Burkina Faso (Vue Nationale)</option>
            <option value="ouaga" selected>🏙️ Ouagadougou (Quartiers & Cellules)</option>
            <option value="bobo">🏛️ Bobo-Dioulasso (Secteurs)</option>
          </select>
          <button class="btn btn-purple btn-sm" onclick="togglePlaybackTrack()" id="btn-carto-playback">▶ Rejouer trajet suspect</button>
          <button class="btn btn-ghost btn-sm" onclick="captureActiveModuleScreen('png')">📸 Capture PNG</button>
          <button class="btn btn-ghost btn-sm" onclick="captureActiveModuleScreen('svg')">📐 Export SVG</button>
          <button class="btn btn-blue btn-sm" onclick="exportModuleReportHTMLSVG('carto')">📄 Exporter HTML-SVG</button>
        </div>
      </div>

      <div class="carto-burkina-container">
        <!-- Sidebar avec filtres et recherche par quartier/ville -->
        <div class="carto-sidebar">
          <div class="carto-search-box">
            <div style="font-weight:600;font-size:12px;margin-bottom:6px">Recherche géographique</div>
            <input type="text" id="carto-search-input" placeholder="🔍 Ex: Ouaga 2000, Patte d'Oie, Gounghin, Bobo..." oninput="filterBurkinaPoints()" style="width:100%;background:var(--surface);border:1px solid var(--border);border-radius:6px;padding:6px 10px;font-size:12px;color:var(--text)">
            
            <div style="display:flex;gap:4px;flex-wrap:wrap;margin-top:8px">
              <button class="btn btn-ghost btn-sm carto-chip active" id="chip-all" onclick="filterCartoType('ALL')">Tous (14)</button>
              <button class="btn btn-ghost btn-sm carto-chip" id="chip-bts" onclick="filterCartoType('BTS')">📡 BTS (5)</button>
              <button class="btn btn-ghost btn-sm carto-chip" id="chip-money" onclick="filterCartoType('MONEY')">💸 Money (4)</button>
              <button class="btn btn-ghost btn-sm carto-chip" id="chip-exif" onclick="filterCartoType('EXIF')">📸 EXIF (3)</button>
              <button class="btn btn-ghost btn-sm carto-chip" id="chip-call" onclick="filterCartoType('CALL')">📞 Appels (2)</button>
            </div>
          </div>

          <div style="padding:10px 14px;background:var(--surface);border-bottom:1px solid var(--border);display:flex;align-items:center;justify-content:space-between">
            <span style="font-size:11px;font-weight:600;color:var(--muted)">ÉVÉNEMENTS IDENTIFIÉS</span>
            <span class="pill pill-green" id="carto-count-badge">14 points</span>
          </div>

          <div id="carto-events-list" style="flex:1;overflow-y:auto;padding:8px">
            <!-- Rendered by JS -->
          </div>

          <div style="padding:10px 14px;border-top:1px solid var(--border);background:var(--surface2)">
            <button class="btn btn-accent btn-sm" style="width:100%" onclick="openAddPointModal()">+ Ajouter repère d'investigation</button>
          </div>
        </div>

        <!-- Viewport Carte Vectorielle Burkina Faso / Ouagadougou -->
        <div class="carto-map-viewport" id="carto-viewport">
          <div class="carto-map-header">
            <div class="carto-hud-card">
              <div style="font-weight:700;font-size:12px;color:var(--accent)" id="carto-hud-title">Ouagadougou & Quartiers — Secteurs d'Enquête</div>
              <div style="font-size:10.5px;color:var(--muted)" id="carto-hud-coords">Centre : 12.3714° N, 1.5197° W · Précision Cellulaire BTS < 350m</div>
            </div>

            <div style="display:flex;gap:6px">
              <button class="btn btn-ghost btn-sm" onclick="zoomBurkinaMap(1.25)" title="Zoom avant">🔍 +</button>
              <button class="btn btn-ghost btn-sm" onclick="zoomBurkinaMap(0.8)" title="Zoom arrière">🔍 −</button>
              <button class="btn btn-ghost btn-sm" onclick="resetBurkinaMapZoom()" title="Réinitialiser vue">⟲ Centrer</button>
            </div>
          </div>

          <div id="burkina-svg-wrapper" style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;cursor:grab;overflow:hidden">
            <!-- Interactive Burkina / Ouaga SVG injected by JS -->
          </div>

          <div style="position:absolute;bottom:12px;left:12px;right:12px;z-index:10;pointer-events:none">
            <div class="carto-hud-card" style="pointer-events:auto;display:flex;align-items:center;justify-content:space-between;flex-wrap:wrap;gap:8px">
              <div style="display:flex;align-items:center;gap:6px;font-size:11px">
                <span class="carto-legend-badge"><span style="color:#22c55e">●</span> Ouaga 2000</span>
                <span class="carto-legend-badge"><span style="color:#00e5c3">●</span> Patte d'Oie</span>
                <span class="carto-legend-badge"><span style="color:#4d9fff">●</span> Gounghin</span>
                <span class="carto-legend-badge"><span style="color:#fb923c">●</span> Kalgondin</span>
                <span class="carto-legend-badge"><span style="color:#a78bfa">●</span> Tampouy</span>
              </div>
              <div style="font-size:11px;color:var(--muted)">
                Cartographie FidEx Géospatiale · Système WGS84
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- GRAPHES -->`;

html = html.replace(oldCartoMarkupRegex, newCartoMarkup);

// 2. UPDATE PAGE-GRAPHES MARKUP WITH FULL IBM ANALYST'S NOTEBOOK (i2) SUITE
const oldGraphesMarkupRegex = /<!-- GRAPHES -->[\s\S]*?<div id="page-graphes" class="page">[\s\S]*?<\/div>\s*<!-- DATA ANALYSIS/;

const newGraphesMarkup = `<!-- ANALYST'S NOTEBOOK (i2) - MODULE D'ANALYSE RELATIONNELLE CRIMINELLE -->
    <div id="page-graphes" class="page">
      <div class="module-header-bar">
        <div class="module-header-title">
          <span style="font-size:24px">🕸️</span>
          <div>
            <h2 style="margin:0">Analyst's Notebook (i2) — Renseignement Criminel & Liens</h2>
            <div style="font-size:11.5px;color:var(--muted)">Analyse relationnelle i2 : identification de structures, chef de réseau, flux financiers et matrice d'associations</div>
          </div>
        </div>
        <div class="module-actions">
          <select id="i2-layout-select" onchange="setI2Layout(this.value)" style="background:var(--surface2);color:var(--text);border:1px solid var(--border);border-radius:6px;padding:4px 8px;font-size:11.5px">
            <option value="organic">🌀 Disposition Organique (i2 Force-Directed)</option>
            <option value="hierarchic">🌲 Disposition Hiérarchique (Arbre d'Autorité)</option>
            <option value="circular">⭕ Disposition Circulaire (Groupes & Cellules)</option>
            <option value="timeline">📅 Matrice Chronologique</option>
          </select>
          <button class="btn btn-purple btn-sm" onclick="runI2CentralityAnalysis()" title="Calcule la centralité d'intermédiarité et identifie le chef de réseau / pivot">👑 Détecter Chef de Réseau</button>
          <button class="btn btn-orange btn-sm" onclick="openI2PathfinderModal()" title="Recherche le chemin de liaison le plus court entre deux suspects ou comptes">🔍 Plus Court Chemin</button>
          <button class="btn btn-ghost btn-sm" onclick="toggleI2MatrixView()" id="btn-toggle-matrix">📊 Matrice d'Association</button>
          <button class="btn btn-ghost btn-sm" onclick="captureActiveModuleScreen('png')">📸 Capture PNG</button>
          <button class="btn btn-ghost btn-sm" onclick="captureActiveModuleScreen('svg')">📐 Export SVG</button>
          <button class="btn btn-blue btn-sm" onclick="exportModuleReportHTMLSVG('i2')">📄 Exporter HTML-SVG</button>
        </div>
      </div>

      <div class="i2-workspace">
        <!-- Toolbar i2 -->
        <div class="i2-toolbar">
          <div class="i2-toolgroup">
            <button class="btn btn-ghost btn-sm" onclick="openAddI2EntityModal()">+ 👤 Entité</button>
            <button class="btn btn-ghost btn-sm" onclick="openAddI2LinkModal()">+ 🔗 Liaison</button>
            <div style="width:1px;height:18px;background:var(--border);margin:0 4px"></div>
            <button class="btn btn-ghost btn-sm" onclick="zoomI2Canvas(1.2)">🔍 +</button>
            <button class="btn btn-ghost btn-sm" onclick="zoomI2Canvas(0.8)">🔍 −</button>
            <button class="btn btn-ghost btn-sm" onclick="resetI2Zoom()">⟲ Centrer</button>
          </div>

          <div class="i2-toolgroup">
            <span style="font-size:11px;color:var(--muted)">Filtre entités :</span>
            <select id="i2-filter-type" onchange="filterI2Graph(this.value)" style="background:var(--surface);color:var(--text);border:1px solid var(--border);border-radius:4px;padding:2px 6px;font-size:11px">
              <option value="ALL">Toutes les entités</option>
              <option value="PERSON">👤 Suspects / Personnes</option>
              <option value="PHONE">📱 Téléphones / IMEI</option>
              <option value="MONEY">💳 Comptes Mobile Money</option>
              <option value="LOCATION">📍 Lieux & Quartiers</option>
              <option value="TOWER">📡 Cellules BTS</option>
            </select>
            <span style="font-size:11px;color:var(--muted);margin-left:8px">Seuil min :</span>
            <input type="range" id="i2-threshold" min="1" max="10" value="1" onchange="updateI2Threshold(this.value)" style="width:70px">
            <span class="mono" style="font-size:11px;color:var(--accent)" id="i2-threshold-val">1 lien</span>
          </div>

          <div class="i2-toolgroup">
            <span class="pill pill-green" id="i2-stats-nodes">18 nœuds</span>
            <span class="pill pill-blue" id="i2-stats-edges">27 liens</span>
          </div>
        </div>

        <div class="i2-body">
          <!-- Canvas Graph SVG -->
          <div class="i2-canvas-area" id="i2-canvas-container">
            <!-- Interactive i2 Network Graph rendered by JS -->
          </div>

          <!-- Matrice d'association (affichée en toggle) -->
          <div id="i2-matrix-container" style="display:none;position:absolute;inset:0;background:#0d121c;z-index:20;overflow:auto;padding:16px">
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
              <h3 style="margin:0">📊 Matrice d'Association Croisée (IBM i2)</h3>
              <button class="btn btn-ghost btn-sm" onclick="toggleI2MatrixView()">✕ Fermer Matrice</button>
            </div>
            <div id="i2-matrix-content"></div>
          </div>

          <!-- Inspector Panel latéral i2 -->
          <div class="i2-side-panel" id="i2-inspector-panel">
            <div style="font-size:11px;text-transform:uppercase;font-weight:700;color:var(--muted);letter-spacing:0.08em;margin-bottom:8px">FICHE D'ANALYSE (i2)</div>
            <div id="i2-inspector-content">
              <div style="text-align:center;padding:24px 0;color:var(--muted);font-size:11.5px">
                Sélectionnez une entité ou une liaison sur le schéma pour examiner son profil, son degré de centralité et ses interactions.
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- DATA ANALYSIS`;

html = html.replace(oldGraphesMarkupRegex, newGraphesMarkup);

// 3. ADD CAPTURE / EXPORT BUTTONS TO EACH OTHER MAJOR MODULE
const moduleHeadersToEnhance = [
  { id: 'page-dashboard', title: 'Dashboard', sub: 'Vue d\'ensemble opérationnelle et statut de la plateforme' },
  { id: 'page-cases', title: 'Dossiers & Sessions Judiciaires', sub: 'Registre légal des réquisitions et scellés numériques' },
  { id: 'page-mobile', title: 'Acquisition Mobile & Dumps', sub: 'Extraction logique ADB, physique MediaTek BROM et Qualcomm EDL 9008' },
  { id: 'page-sdcard', title: 'Cartes SD & Carving', sub: 'Création d\'images bit-à-bit et reconstruction de fichiers bruts' },
  { id: 'page-disk', title: 'Disques & Images E01 / DD', sub: 'Acquisition légale de supports de stockage et calcul d\'empreintes' },
  { id: 'page-explorer', title: 'Explorateur de Preuves & Fichiers', sub: 'Arborescence des pièces et vérification d\'intégrité cryptographique' },
  { id: 'page-whatsapp', title: 'Forensique WhatsApp', sub: 'Déchiffrement msgstore.db.crypt14/15 et analyse des échanges' },
  { id: 'page-money', title: 'Flux Financiers Mobile Money', sub: 'Dépistage des transferts Orange Money, Wave et Moov Money' },
  { id: 'page-timeline', title: 'Chronologie des Événements', sub: 'Alignement chronologique multi-sources des preuves de l\'enquête' },
  { id: 'page-calendar', title: 'Calendrier Judiciaire', sub: 'Planification des réquisitions, auditions et échéances d\'expertise' },
  { id: 'page-browser', title: 'Navigateurs & Traces Web', sub: 'Historique, requêtes de recherche et cache des moteurs' },
  { id: 'page-exif', title: 'Métadonnées EXIF & GPS', sub: 'Coordonnées géographiques, appareil photo et horodatages des médias' },
  { id: 'page-hexviewer', title: 'Inspecteur Hexadécimal', sub: 'Analyse binaire, signatures magiques de fichiers et calcul d\'entropie' },
  { id: 'page-iocheck', title: 'Contrôle Hashs & Signatures IOC', sub: 'Vérification cryptographique ISO 27037 et détection de menaces' }
];

moduleHeadersToEnhance.forEach(m => {
  const pageTag = `<div id="${m.id}" class="page`;
  if (html.includes(pageTag) && !html.includes(`data-has-action-bar="${m.id}"`)) {
    const actionBarHtml = `
      <div class="module-header-bar" data-has-action-bar="${m.id}">
        <div class="module-header-title">
          <div>
            <h2 style="margin:0">${m.title}</h2>
            <div style="font-size:11.5px;color:var(--muted)">${m.sub}</div>
          </div>
        </div>
        <div class="module-actions">
          <button class="btn btn-ghost btn-sm" onclick="captureActiveModuleScreen('png')">📸 Capture PNG</button>
          <button class="btn btn-ghost btn-sm" onclick="captureActiveModuleScreen('svg')">📐 Export SVG</button>
          <button class="btn btn-accent btn-sm" onclick="exportModuleReportHTMLSVG('${m.id.replace('page-','')}')">📄 Exporter HTML-SVG</button>
        </div>
      </div>`;
    
    // Replace opening of page to include this standardized action bar
    const regex = new RegExp(`(<div id="${m.id}" class="page[^"]*">)`);
    html = html.replace(regex, `$1\n${actionBarHtml}\n`);
  }
});

// 4. WRITE UPDATED HTML
fs.writeFileSync('index.html', html, 'utf8');
console.log("Markup for Burkina Faso Cartography and Analyst's Notebook i2 successfully injected!");
