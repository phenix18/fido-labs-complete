const fs = require('fs');

console.log("Preparing FidEx (fido-labs-extraction) enhancements...");

let html = fs.readFileSync('index.html', 'utf8');

// 1. UPDATE TITLE AND META TAGS
html = html.replace(/<title>.*?<\/title>/i, '<title>FidEx ( fido-labs-extraction )</title>');
html = html.replace(/<meta property="og:title" content=".*?">/i, '<meta property="og:title" content="FidEx ( fido-labs-extraction )">');
html = html.replace(/<meta name="description" content=".*?">/i, '<meta name="description" content="FidEx ( fido-labs-extraction ) — Plateforme forensique complète : extraction mobile (ADB/BROM/EDL), cartes SD, disques, WhatsApp, Mobile Money, inspecteur hexadécimal, forensique web, EXIF & GPS, contrôle d\'intégrité IOC/NSRL, règles YARA, Analyst Notebook i2, Cartographie Burkina Faso et procès-verbal judiciaire ISO 27037.">');
html = html.replace(/<meta property="og:description" content=".*?">/i, '<meta property="og:description" content="FidEx ( fido-labs-extraction ) — Plateforme forensique d\'extraction et d\'analyse numérique avancée (Fido-Labs 2026).">');

// 2. UPDATE SIDEBAR LOGO AND MENUBAR BRAND
html = html.replace(
  /<div class="logo" onclick="toggleSidebar\(\)"><div class="li">FL<\/div><span class="lt">FIDO-LABS v3<\/span><\/div>/,
  `<div class="logo" onclick="toggleSidebar()" title="FidEx (fido-labs-extraction)">
    <div class="li" style="background:linear-gradient(135deg,#00e5c3,#4d9fff);box-shadow:0 0 10px rgba(0,229,195,0.4)">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#0a0c12" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
        <path d="M9 12l2 2 4-4"/>
      </svg>
    </div>
    <div style="display:flex;flex-direction:column;min-width:0;line-height:1.15">
      <span class="lt" style="font-size:13.5px;font-weight:800;letter-spacing:.06em;color:#00e5c3">FidEx</span>
      <span class="lt" style="font-size:9.5px;font-weight:500;color:var(--muted);letter-spacing:0">fido-labs-extraction</span>
    </div>
  </div>`
);

html = html.replace(
  /<span class="mb-brand">🛡 FIDO-LABS<\/span>/,
  `<span class="mb-brand" style="display:inline-flex;align-items:center;gap:6px;font-weight:700;color:#00e5c3">
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#00e5c3" stroke-width="2.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><circle cx="12" cy="11" r="3"/></svg>
    FidEx (fido-labs-extraction)
  </span>`
);

// 3. TOPBAR ANTI-BUZZ & CAPTURE QUICK ACTIONS
if (!html.includes('id="btn-anti-buzz"')) {
  html = html.replace(
    /<span id="licbadge" class="lc-badge dem"/,
    `<div style="display:flex;align-items:center;gap:6px">
      <button class="btn btn-ghost btn-sm" id="btn-anti-buzz" onclick="toggleAntiBuzz()" title="Mode Silencieux / Anti-Buzz Forensique (Supprime les bruits et alertes intempestives pendant l'extraction)" style="font-size:11.5px;border:1px solid var(--border)">
        <span id="anti-buzz-ico">🔕</span> <span id="anti-buzz-lbl">Anti-Buzz : Actif</span>
      </button>
      <button class="btn btn-ghost btn-sm" onclick="exportModuleReportHTMLSVG()" title="Export du module actuel en format interactif HTML-SVG" style="font-size:11.5px;border:1px solid var(--border)">
        📄 HTML-SVG
      </button>
      <button class="btn btn-ghost btn-sm" onclick="captureActiveModuleScreen('png')" title="Capture d'écran haute résolution du module" style="font-size:11.5px;border:1px solid var(--border)">
        📸 Capture
      </button>
    </div>
    <span id="licbadge" class="lc-badge dem"`
  );
}

// 4. ADD FOOTER BEFORE #main CLOSING TAG OR TERMINAL
const footerHtml = `
  <!-- PIED DE PAGE JUDICIAIRE OBLIGATOIRE -->
  <footer id="app-footer" class="app-footer">
    <div class="footer-left">
      <span class="footer-dot pulse"></span>
      <span class="footer-brand">FidEx ( fido-labs-extraction )</span>
      <span class="footer-sep">·</span>
      <span class="footer-info">Moteur Forensique ISO/IEC 27037 · Scellés MD5/SHA-256</span>
    </div>
    <div class="footer-center" id="footer-status-indicator">
      <span>● Prêt pour acquisition & analyse</span>
    </div>
    <div class="footer-right">
      <span class="footer-copy">copyrith ( FidEx ), Fido - labs  extraction 2026</span>
    </div>
  </footer>
`;

if (!html.includes('id="app-footer"')) {
  // Insert before </div> <!-- #main -->
  html = html.replace('</div>\n\n  <div id="terminal">', `${footerHtml}\n  </div>\n\n  <div id="terminal">`);
}

// 5. CSS STYLES FOR SPLASH, LOADER, I2, CARTO BURKINA, ACTION BARS, FOOTER, AND DRILLDOWN
const extraStyles = `
/* ==========================================================================
   FIDEX (FIDO-LABS-EXTRACTION) - HIGH-TECH FORENSIC STYLES & ANIMATIONS
   ========================================================================== */

/* FOOTER */
.app-footer {
  height: 28px;
  min-height: 28px;
  background: #080a0f;
  border-top: 1px solid var(--border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  font-size: 11px;
  color: var(--muted);
  user-select: none;
  z-index: 15;
}
.footer-left { display: flex; align-items: center; gap: 8px; }
.footer-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--accent); }
.footer-dot.pulse { box-shadow: 0 0 8px var(--accent); animation: pulseDot 2s infinite ease-in-out; }
@keyframes pulseDot { 0%, 100% { opacity: 1; transform: scale(1); } 50% { opacity: 0.5; transform: scale(0.85); } }
.footer-brand { font-weight: 700; color: var(--accent); font-family: var(--mono); font-size: 10.5px; }
.footer-sep { color: var(--faint); }
.footer-center { font-size: 10.5px; color: var(--blue); font-family: var(--mono); }
.footer-right { display: flex; align-items: center; font-weight: 600; color: #a0aec0; letter-spacing: 0.02em; }
.footer-copy { background: linear-gradient(90deg, #94a3b8, #cbd5e1); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }

/* MODULE STANDARDIZED ACTION BAR */
.module-header-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 14px;
  padding: 10px 14px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
}
.module-header-title { display: flex; align-items: center; gap: 10px; }
.module-header-title h2 { margin: 0; font-size: 16px; font-weight: 700; }
.module-actions { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }

/* SPLASH SCREEN INTRO ANIMATION */
#fidex-splash-screen {
  position: fixed;
  inset: 0;
  z-index: 999999;
  background: radial-gradient(circle at center, #0f172a 0%, #030712 100%);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  font-family: var(--sans);
  transition: opacity 0.6s ease, visibility 0.6s ease;
}
#fidex-splash-screen.fade-out {
  opacity: 0;
  visibility: hidden;
  pointer-events: none;
}
.splash-grid-bg {
  position: absolute;
  inset: 0;
  background-image: 
    linear-gradient(rgba(0, 229, 195, 0.04) 1px, transparent 1px),
    linear-gradient(90deg, rgba(0, 229, 195, 0.04) 1px, transparent 1px);
  background-size: 40px 40px;
  perspective: 500px;
  pointer-events: none;
}
.splash-radar-sweep {
  position: absolute;
  width: 480px;
  height: 480px;
  border-radius: 50%;
  border: 1px dashed rgba(0, 229, 195, 0.25);
  animation: radarRotate 8s linear infinite;
  pointer-events: none;
}
@keyframes radarRotate { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
.splash-logo-box {
  position: relative;
  width: 110px;
  height: 110px;
  margin-bottom: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.splash-logo-glow {
  position: absolute;
  inset: -15px;
  background: radial-gradient(circle, rgba(0, 229, 195, 0.35) 0%, transparent 70%);
  animation: pulseLogoGlow 2.5s ease-in-out infinite;
}
@keyframes pulseLogoGlow { 0%, 100% { opacity: 0.6; transform: scale(0.95); } 50% { opacity: 1; transform: scale(1.1); } }
.splash-title {
  font-size: 32px;
  font-weight: 800;
  letter-spacing: 0.12em;
  font-family: var(--mono);
  background: linear-gradient(135deg, #00e5c3 0%, #4d9fff 50%, #a78bfa 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  text-shadow: 0 0 30px rgba(0, 229, 195, 0.4);
}
.splash-subtitle {
  font-size: 13px;
  font-weight: 600;
  color: #94a3b8;
  letter-spacing: 0.25em;
  text-transform: uppercase;
  margin-top: 4px;
  margin-bottom: 28px;
}
.splash-progress-track {
  width: 380px;
  max-width: 85vw;
  height: 4px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 4px;
  overflow: hidden;
  position: relative;
}
.splash-progress-bar {
  height: 100%;
  width: 0%;
  background: linear-gradient(90deg, #00e5c3, #4d9fff);
  box-shadow: 0 0 12px #00e5c3;
  transition: width 0.3s ease;
}
.splash-terminal {
  margin-top: 16px;
  font-family: var(--mono);
  font-size: 11.5px;
  color: #00e5c3;
  height: 22px;
  display: flex;
  align-items: center;
  gap: 8px;
}
.splash-skip-btn {
  margin-top: 28px;
  background: transparent;
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #94a3b8;
  padding: 6px 18px;
  border-radius: 20px;
  font-size: 11px;
  cursor: pointer;
  transition: all 0.2s;
}
.splash-skip-btn:hover {
  border-color: #00e5c3;
  color: #00e5c3;
}

/* REALISTIC LOADING OVERLAY */
#loading-overlay {
  position: fixed;
  inset: 0;
  background: rgba(5, 7, 12, 0.92);
  display: none;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  z-index: 99999;
  backdrop-filter: blur(8px);
}
#loading-overlay.show { display: flex; }
.forensic-loader-card {
  background: #0f141f;
  border: 1px solid var(--border2);
  border-radius: 14px;
  padding: 32px 40px;
  width: 480px;
  max-width: 90vw;
  display: flex;
  flex-direction: column;
  align-items: center;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.8), 0 0 30px rgba(0, 229, 195, 0.15);
  position: relative;
  overflow: hidden;
}
.forensic-loader-card::before {
  content: '';
  position: absolute;
  top: 0; left: 0; right: 0; height: 3px;
  background: linear-gradient(90deg, #00e5c3, #4d9fff, #a78bfa);
}
.forensic-hud-ring {
  width: 90px;
  height: 90px;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 20px;
}
.hud-outer-ring {
  position: absolute;
  inset: 0;
  border: 2px dashed rgba(0, 229, 195, 0.35);
  border-radius: 50%;
  animation: hudSpin 8s linear infinite;
}
.hud-inner-ring {
  position: absolute;
  inset: 8px;
  border: 2px solid transparent;
  border-top-color: #00e5c3;
  border-right-color: #4d9fff;
  border-radius: 50%;
  animation: hudSpinReverse 2.5s linear infinite;
}
@keyframes hudSpin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
@keyframes hudSpinReverse { from { transform: rotate(0deg); } to { transform: rotate(-360deg); } }
.hud-pct-text {
  font-family: var(--mono);
  font-size: 20px;
  font-weight: 800;
  color: #fff;
  text-shadow: 0 0 10px #00e5c3;
}
.forensic-loader-stage {
  font-size: 15px;
  font-weight: 700;
  color: #f1f5f9;
  text-align: center;
  margin-bottom: 6px;
}
.forensic-loader-sub {
  font-size: 12px;
  color: var(--muted);
  text-align: center;
  margin-bottom: 18px;
  font-family: var(--mono);
  min-height: 18px;
}
.forensic-progress-wrap {
  width: 100%;
  height: 7px;
  background: var(--surface2);
  border-radius: 4px;
  overflow: hidden;
  position: relative;
  border: 1px solid var(--border);
}
.forensic-progress-fill {
  height: 100%;
  width: 0%;
  background: linear-gradient(90deg, #00e5c3, #4d9fff);
  box-shadow: 0 0 10px #00e5c3;
  transition: width 0.25s ease;
}
.forensic-loader-stats {
  display: flex;
  justify-content: space-between;
  width: 100%;
  font-size: 11px;
  color: var(--muted);
  margin-top: 8px;
  font-family: var(--mono);
}
.forensic-loader-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  margin-top: 20px;
  padding-top: 14px;
  border-top: 1px solid var(--border);
}

/* DATA ANALYSIS DRILL-DOWN MODAL */
.drill-modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border);
  padding-bottom: 12px;
  margin-bottom: 14px;
}
.drill-stat-badges {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 14px;
}

/* IBM ANALYST NOTEBOOK (i2) GRAPH VIEW */
.i2-workspace {
  background: #090c14;
  border: 1px solid var(--border);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  height: calc(100vh - 210px);
  min-height: 540px;
}
.i2-toolbar {
  height: 42px;
  background: #0f1422;
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  gap: 8px;
  flex-wrap: wrap;
  user-select: none;
}
.i2-toolgroup { display: flex; align-items: center; gap: 4px; }
.i2-body { display: flex; flex: 1; min-height: 0; position: relative; }
.i2-canvas-area { flex: 1; position: relative; overflow: hidden; background: radial-gradient(circle at center, #101728 0%, #060911 100%); }
.i2-side-panel {
  width: 290px;
  background: #0d121c;
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  padding: 14px;
}
.i2-matrix-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 11px;
}
.i2-matrix-table th, .i2-matrix-table td {
  border: 1px solid var(--border);
  padding: 4px 6px;
  text-align: center;
}

/* BURKINA FASO CARTOGRAPHY STYLES */
.carto-burkina-container {
  display: grid;
  grid-template-columns: 310px 1fr;
  gap: 14px;
  height: calc(100vh - 200px);
  min-height: 540px;
}
@media(max-width: 900px) {
  .carto-burkina-container { grid-template-columns: 1fr; height: auto; }
}
.carto-sidebar {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.carto-search-box {
  padding: 12px;
  border-bottom: 1px solid var(--border);
  background: var(--surface2);
}
.carto-map-viewport {
  background: #070a12;
  border: 1px solid var(--border);
  border-radius: 10px;
  position: relative;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.carto-map-header {
  position: absolute;
  top: 12px;
  left: 12px;
  right: 12px;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: space-between;
  pointer-events: none;
}
.carto-map-header > * { pointer-events: auto; }
.carto-hud-card {
  background: rgba(13, 18, 28, 0.88);
  border: 1px solid var(--border2);
  border-radius: 8px;
  padding: 8px 14px;
  backdrop-filter: blur(8px);
  box-shadow: 0 8px 24px rgba(0,0,0,0.5);
}
.carto-legend-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  padding: 3px 8px;
  border-radius: 4px;
  background: var(--surface2);
  border: 1px solid var(--border);
  margin-right: 4px;
  margin-bottom: 4px;
}
`;

// Insert styles before </style>
html = html.replace('</style>', `${extraStyles}\n</style>`);

// 6. INSERT SPLASH SCREEN MARKUP AT BODY TOP
const splashHtml = `
<!-- FIDEX SPLASH INTRO SCREEN -->
<div id="fidex-splash-screen">
  <div class="splash-grid-bg"></div>
  <div class="splash-radar-sweep"></div>
  <div class="splash-logo-box">
    <div class="splash-logo-glow"></div>
    <svg width="86" height="86" viewBox="0 0 100 100" fill="none">
      <defs>
        <linearGradient id="fxGrad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#00e5c3" />
          <stop offset="50%" stop-color="#4d9fff" />
          <stop offset="100%" stop-color="#a78bfa" />
        </linearGradient>
      </defs>
      <!-- Shield Outer -->
      <path d="M50 5 L88 20 C88 56 68 84 50 95 C32 84 12 56 12 20 Z" stroke="url(#fxGrad)" stroke-width="3.5" fill="#090d16" />
      <!-- Inner Radar Ring -->
      <circle cx="50" cy="48" r="24" stroke="#4d9fff" stroke-width="1.5" stroke-dasharray="4 3" opacity="0.6"/>
      <circle cx="50" cy="48" r="14" stroke="#00e5c3" stroke-width="2"/>
      <!-- Forensic Eye / Lens Cross -->
      <line x1="50" y1="28" x2="50" y2="68" stroke="#00e5c3" stroke-width="1.5" opacity="0.75"/>
      <line x1="30" y1="48" x2="70" y2="48" stroke="#00e5c3" stroke-width="1.5" opacity="0.75"/>
      <circle cx="50" cy="48" r="4" fill="#00e5c3"/>
    </svg>
  </div>
  <div class="splash-title">FidEx</div>
  <div class="splash-subtitle">fido-labs-extraction</div>
  <div class="splash-progress-track">
    <div class="splash-progress-bar" id="splash-pbar"></div>
  </div>
  <div class="splash-terminal" id="splash-term">
    <span>●</span> <span id="splash-log">Initialisation du noyau forensique FidEx v3.2...</span>
  </div>
  <button class="splash-skip-btn" onclick="skipSplash()">Passer l'introduction ▶</button>
</div>
`;

if (!html.includes('id="fidex-splash-screen"')) {
  html = html.replace('<div id="app">', `${splashHtml}\n<div id="app">`);
}

// 7. REPLACE OLD LOADING OVERLAY WITH REALISTIC HUD LOADER
const newLoadingOverlay = `
<!-- REALISTIC FORENSIC LOADER OVERLAY (ANTI-BUZZ & VRAIE PROGRESSION) -->
<div id="loading-overlay">
  <div class="forensic-loader-card">
    <div class="forensic-hud-ring">
      <div class="hud-outer-ring"></div>
      <div class="hud-inner-ring"></div>
      <div class="hud-pct-text" id="loading-percent">0%</div>
    </div>
    <div class="forensic-loader-stage" id="loading-stage">Initialisation de l'acquisition...</div>
    <div class="forensic-loader-sub" id="loading-details">Négociation de la couche matérielle USB/ADB</div>
    <div class="forensic-progress-wrap">
      <div class="forensic-progress-fill" id="loading-progress-fill"></div>
    </div>
    <div class="forensic-loader-stats">
      <span id="loading-rate">Vitesse: 0.0 Mo/s</span>
      <span id="loading-transferred">0 Mo / --</span>
      <span id="loading-eta">ETA: Calcul...</span>
    </div>
    <div class="forensic-loader-footer">
      <span style="font-size:11px;color:var(--muted)">Mode Silencieux / Anti-Buzz : <b style="color:var(--accent)">Actif</b></span>
      <button class="btn btn-ghost btn-sm" id="btn-cancel-loading" onclick="abortForensicExtraction()">Interrompre</button>
      <button class="btn btn-accent btn-sm" id="save-btn" onclick="saveLocally()" style="display:none">💾 Sauvegarder .fidex</button>
    </div>
  </div>
</div>
`;

html = html.replace(/<div id="loading-overlay">[\s\S]*?<\/div>/, newLoadingOverlay);

// 8. ADD DRILL-DOWN MODAL FOR DATA ANALYSIS
const drilldownModalHtml = `
<!-- MODALE DE CONSULTATION DÉTAILLÉE AU CLIC SUR DIAGRAMME (DATA ANALYSE) -->
<div id="modal-chart-drilldown" class="modal-layer" hidden onclick="if(event.target===this)closeModal('modal-chart-drilldown')">
  <div class="modal" style="max-width:860px;max-height:85vh;display:flex;flex-direction:column">
    <div class="drill-modal-header">
      <div style="display:flex;align-items:center;gap:10px">
        <span style="font-size:24px" id="drill-icon">🥧</span>
        <div>
          <h3 style="margin:0" id="drill-title">Détail des données de la répartition</h3>
          <div style="font-size:11.5px;color:var(--muted)" id="drill-subtitle">Données individuelles rattachées à ce segment</div>
        </div>
      </div>
      <button class="btn btn-ghost btn-sm" onclick="closeModal('modal-chart-drilldown')">✕</button>
    </div>

    <div class="drill-stat-badges" id="drill-badges">
      <!-- Injected by JS -->
    </div>

    <div style="display:flex;gap:8px;margin-bottom:12px;align-items:center">
      <input type="text" id="drill-search" placeholder="🔍 Filtrer les enregistrements de cette sélection..." oninput="filterDrillTable()" style="flex:1">
      <button class="btn btn-ghost btn-sm" onclick="exportDrillData('csv')">📊 Export CSV</button>
      <button class="btn btn-ghost btn-sm" onclick="exportDrillData('json')">📋 JSON</button>
      <button class="btn btn-blue btn-sm" onclick="exportDrillHTMLSVG()">📄 Export HTML-SVG</button>
    </div>

    <div class="twrap" style="flex:1;overflow:auto;max-height:46vh;border:1px solid var(--border);border-radius:6px">
      <table>
        <thead id="drill-table-head">
          <!-- Dynamic headers -->
        </thead>
        <tbody id="drill-table-body">
          <!-- Dynamic rows -->
        </tbody>
      </table>
    </div>

    <div class="modal-footer" style="margin-top:14px;display:flex;justify-content:space-between;align-items:center">
      <div style="font-size:11.5px;color:var(--muted)" id="drill-count-info">0 enregistrement(s)</div>
      <div style="display:flex;gap:6px">
        <button class="btn btn-ghost" onclick="closeModal('modal-chart-drilldown')">Fermer</button>
        <button class="btn btn-accent" id="drill-action-btn" onclick="openDrillTargetModule()">🔍 Ouvrir dans le module dédié</button>
      </div>
    </div>
  </div>
</div>
`;

if (!html.includes('id="modal-chart-drilldown"')) {
  html = html.replace('<!-- DIV PROTECTEUR UNIQUE POUR TOUTES LES MODALES -->', `<!-- DIV PROTECTEUR UNIQUE POUR TOUTES LES MODALES -->\n${drilldownModalHtml}\n`);
}

// Write the intermediate updated HTML
fs.writeFileSync('index.html', html, 'utf8');
console.log("HTML structure and styles successfully updated!");
