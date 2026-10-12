const fs = require('fs');

console.log("Applying FidEx logo & 100% counter updates, removing continuous fido-labs animation...");

let html = fs.readFileSync('index.html', 'utf8');

// 1. UPDATE SPINNER CSS (REMOVE .spinner-text AND @keyframes spinText)
const oldSpinnerCSS = `.spinner-circle{width:80px;height:80px;border:4px solid var(--border);border-top-color:var(--accent);border-radius:50%;animation:spin 1s linear infinite;margin-bottom:20px}
@keyframes spin{to{transform:rotate(360deg)}}
.spinner-text{font-size:48px;font-weight:800;font-family:var(--mono);background:linear-gradient(90deg,var(--accent),var(--blue),var(--purple));-webkit-background-clip:text;-webkit-text-fill-color:transparent;animation:spinText 2s linear infinite}
@keyframes spinText{0%{transform:rotate(0deg)}100%{transform:rotate(360deg)}}
#loading-percent{font-size:24px;font-weight:700;font-family:var(--mono);color:var(--accent);margin-top:12px}
#save-btn{display:none;margin-top:20px;padding:10px 24px;background:var(--accent);color:#0a0c12;border:none;border-radius:8px;font-weight:600;cursor:pointer;font-family:var(--sans);font-size:14px}
#save-btn:hover{opacity:0.9}`;

const newSpinnerCSS = `.spinner-circle{width:70px;height:70px;border:3px solid var(--border);border-top-color:var(--accent);border-radius:50%;animation:spin 1s linear infinite;margin-bottom:16px}
@keyframes spin{to{transform:rotate(360deg)}}
#loading-percent{font-size:24px;font-weight:700;font-family:var(--mono);color:var(--accent);margin-top:10px}
#save-btn{display:none;margin-top:16px;padding:8px 20px;background:var(--accent);color:#0a0c12;border:none;border-radius:6px;font-weight:600;cursor:pointer;font-family:var(--sans);font-size:13px}
#save-btn:hover{opacity:0.9}`;

if (html.includes(oldSpinnerCSS)) {
  html = html.replace(oldSpinnerCSS, newSpinnerCSS);
  console.log("✓ Replaced old spinner CSS and removed continuous spinText");
} else {
  console.warn("! oldSpinnerCSS not found directly, checking partial replacement");
  html = html.replace(/\.spinner-text\{[^}]+\}\s*@keyframes spinText\{[^}]+\}/g, '');
}

// 2. UPDATE SPLASH SCREEN CSS (CLEAN LOGO + 100% COUNTER, NO INFINITE TEXT OR RADAR)
const splashCssStart = html.indexOf('/* SPLASH SCREEN INTRO ANIMATION */');
const splashCssEnd = html.indexOf('.splash-skip-btn:hover {', splashCssStart);
if (splashCssStart !== -1 && splashCssEnd !== -1) {
  const fullEnd = html.indexOf('}', splashCssEnd) + 1;
  const newSplashCSS = `/* SPLASH SCREEN INTRO ANIMATION : UNIQUEMENT LE LOGO ET LE COMPTE DE 100% */
#fidex-splash-screen {
  position: fixed;
  inset: 0;
  z-index: 999999;
  background: radial-gradient(circle at center, #0d1527 0%, #030712 100%);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  font-family: var(--sans);
  cursor: pointer;
  transition: opacity 0.45s cubic-bezier(0.4, 0, 0.2, 1), visibility 0.45s ease;
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
    linear-gradient(rgba(0, 229, 195, 0.03) 1px, transparent 1px),
    linear-gradient(90deg, rgba(0, 229, 195, 0.03) 1px, transparent 1px);
  background-size: 40px 40px;
  pointer-events: none;
}
.splash-content-wrap {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  z-index: 2;
}
.splash-logo-box {
  width: 115px;
  height: 115px;
  margin-bottom: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  filter: drop-shadow(0 0 25px rgba(0, 229, 195, 0.4));
  transition: transform 0.3s ease;
}
.splash-pct {
  font-family: var(--mono);
  font-size: 42px;
  font-weight: 800;
  letter-spacing: 0.04em;
  color: #00e5c3;
  text-shadow: 0 0 24px rgba(0, 229, 195, 0.65);
  margin-bottom: 18px;
  min-width: 120px;
  text-align: center;
}
.splash-progress-track {
  width: 240px;
  max-width: 78vw;
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
  box-shadow: 0 0 10px #00e5c3;
  transition: width 0.05s linear;
}`;
  html = html.substring(0, splashCssStart) + newSplashCSS + html.substring(fullEnd);
  console.log("✓ Updated splash screen CSS");
} else {
  console.warn("! Could not find splashCssStart or splashCssEnd");
}

// 3. UPDATE SPLASH SCREEN HTML MARKUP (LOGO & COMPTE DE 100% ONLY)
const splashHtmlStart = html.indexOf('<!-- FIDEX SPLASH INTRO SCREEN -->');
const splashHtmlEnd = html.indexOf('<div id="app">', splashHtmlStart);
if (splashHtmlStart !== -1 && splashHtmlEnd !== -1) {
  const newSplashHTML = `<!-- FIDEX SPLASH INTRO SCREEN : LOGO & COMPTEUR 100% -->
<div id="fidex-splash-screen" onclick="skipSplash()" title="Cliquer pour accéder à FidEx">
  <div class="splash-grid-bg"></div>
  <div class="splash-content-wrap">
    <div class="splash-logo-box">
      <svg width="112" height="112" viewBox="0 0 100 100" fill="none">
        <defs>
          <linearGradient id="fxGrad" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="#00e5c3" />
            <stop offset="50%" stop-color="#4d9fff" />
            <stop offset="100%" stop-color="#8b5cf6" />
          </linearGradient>
          <radialGradient id="fxGlow" cx="50%" cy="48%" r="42%">
            <stop offset="0%" stop-color="#00e5c3" stop-opacity="0.35" />
            <stop offset="100%" stop-color="#00e5c3" stop-opacity="0" />
          </radialGradient>
        </defs>
        <!-- Shield Background & Border -->
        <path d="M50 5 L88 20 C88 56 68 84 50 95 C32 84 12 56 12 20 Z" stroke="url(#fxGrad)" stroke-width="3.2" fill="#090d16" />
        <path d="M50 11 L82 24 C82 52 65 76 50 86 C35 76 18 52 18 24 Z" stroke="rgba(0, 229, 195, 0.22)" stroke-width="1.2" fill="none" />
        <!-- Glowing Core -->
        <circle cx="50" cy="48" r="28" fill="url(#fxGlow)" />
        <!-- Reticle Precision Rings -->
        <circle cx="50" cy="48" r="24" stroke="#4d9fff" stroke-width="1.5" stroke-dasharray="4 3" opacity="0.6"/>
        <circle cx="50" cy="48" r="14" stroke="#00e5c3" stroke-width="2"/>
        <!-- Forensic Eye / Crosshair -->
        <line x1="50" y1="28" x2="50" y2="68" stroke="#00e5c3" stroke-width="1.5" opacity="0.8"/>
        <line x1="30" y1="48" x2="70" y2="48" stroke="#00e5c3" stroke-width="1.5" opacity="0.8"/>
        <circle cx="50" cy="48" r="4" fill="#00e5c3"/>
      </svg>
    </div>
    <div class="splash-pct" id="splash-pct">0%</div>
    <div class="splash-progress-track">
      <div class="splash-progress-bar" id="splash-pbar"></div>
    </div>
  </div>
</div>

`;
  html = html.substring(0, splashHtmlStart) + newSplashHTML + html.substring(splashHtmlEnd);
  console.log("✓ Updated splash screen HTML markup");
} else {
  console.warn("! Could not find splashHtmlStart or splashHtmlEnd");
}

// 4. UPDATE JS: initFidExSplash() & skipSplash()
const jsSplashStart = html.indexOf("// 2. SPLASH SCREEN & ANIMATION D'OUVERTURE");
const jsSplashEnd = html.indexOf("// 3. MOTEUR D'EXTRACTION", jsSplashStart);
if (jsSplashStart !== -1 && jsSplashEnd !== -1) {
  const newJsSplash = `// 2. ANIMATION D'OUVERTURE : LOGO ET COMPTEUR 100% (SANS TEXTE EN BOUCLE)
let splashTimer = null;
function initFidExSplash() {
  const splash = document.getElementById('fidex-splash-screen');
  const pctEl = document.getElementById('splash-pct');
  const pbar = document.getElementById('splash-pbar');
  if (!splash) return;

  // Clic pour passer instantanément
  splash.onclick = () => skipSplash();

  const target = 100;
  const startTime = performance.now();
  const totalDuration = 1800; // progression fluide sur 1.8s

  function step(now) {
    const elapsed = now - startTime;
    const progress = Math.min(elapsed / totalDuration, 1);
    
    // Courbe d'accélération/décélération fluide (ease-out)
    const eased = 1 - Math.pow(1 - progress, 2);
    const val = Math.min(100, Math.round(eased * target));
    
    if (pctEl) pctEl.textContent = val + '%';
    if (pbar) pbar.style.width = val + '%';

    if (progress < 1) {
      splashTimer = requestAnimationFrame(step);
    } else {
      if (pctEl) pctEl.textContent = '100%';
      if (pbar) pbar.style.width = '100%';
      // Pause nette de 250ms à 100% avant d'ouvrir l'application
      setTimeout(() => {
        skipSplash();
      }, 250);
    }
  }

  splashTimer = requestAnimationFrame(step);
}

function skipSplash() {
  if (splashTimer) {
    cancelAnimationFrame(splashTimer);
    clearTimeout(splashTimer);
  }
  const splash = document.getElementById('fidex-splash-screen');
  if (splash) {
    splash.classList.add('fade-out');
    setTimeout(() => { splash.style.display = 'none'; }, 500);
  }
}

`;
  html = html.substring(0, jsSplashStart) + newJsSplash + html.substring(jsSplashEnd);
  console.log("✓ Updated initFidExSplash and skipSplash implementation");
} else {
  console.warn("! Could not find jsSplashStart or jsSplashEnd");
}

// 5. REMOVE STRAY SPINNER AND ORPHAN DIV AROUND LINE 6293
const strayStart = html.indexOf('<div class="spinner-text">fido-labs</div>');
if (strayStart !== -1) {
  const strayEnd = html.indexOf('</div>', html.indexOf('💾 Sauvegarder localement', strayStart)) + 6;
  html = html.substring(0, strayStart) + html.substring(strayEnd);
  console.log("✓ Removed stray spinner-text fido-labs and orphan div");
}

fs.writeFileSync('index.html', html, 'utf8');
console.log("Done updating index.html!");
