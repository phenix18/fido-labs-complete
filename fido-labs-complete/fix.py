import re

with open("index.html", "r", encoding="utf-8") as f:
    html = f.read()

# 1. Fixer l'id=modal-back dupliqué. Le premier reste id="modal-back".
# Les autres deviennent modal-back-evt, modal-back-csm, modal-back-map, modal-back-flt, modal-back-rpt
# Mais en fait, l'architecture d'origine utilise `document.getElementById('modal-back')` pour tous
# et affiche/masque. La meilleure solution est d'avoir un SEUL modal-back, avec tous les modals dedans !
# Ou de renommer les IDs et de modifier le JS en conséquence. 
# Re-écrivons la partie modales proprement avec des identifiants uniques.

# Extraction des modales
modals_section = html[html.find('<!-- MODAL DOSSIER -->'):html.find('<script>')]

new_modals = """
<!-- DIV PROTECTEUR UNIQUE POUR TOUTES LES MODALES -->

<div id="modal-case" class="modal-layer" hidden>
  <div class="modal">
    <h3>📁 Nouveau dossier</h3>
    <div class="frow"><label>Nom</label><input id="cn"></div>
    <div class="frow"><label>Référence</label><input id="cr"></div>
    <div class="frow"><label>Opérateur</label><input id="cop"></div>
    <div class="frow"><label>Notes</label><textarea id="cno"></textarea></div>
    <div class="modal-footer"><button class="btn btn-ghost" onclick="closeModal('modal-case')">Annuler</button><button class="btn btn-accent" onclick="createCase()">Créer</button></div>
  </div>
</div>

<div id="modal-evt" class="modal-layer" hidden>
  <div class="modal">
    <h3>🗓 Nouvel événement</h3>
    <div class="frow"><label>Titre</label><input id="evt-title"></div>
    <div class="frow"><label>Description</label><textarea id="evt-desc"></textarea></div>
    <div class="frow"><label>Début</label><input id="evt-start" type="datetime-local"></div>
    <div class="frow"><label>Fin</label><input id="evt-end" type="datetime-local"></div>
    <div class="frow"><label>Catégorie</label><select id="evt-cat"><option>Extraction</option><option>Analysis</option><option>Transfer</option><option>Hearing</option><option>Deadline</option><option>Meeting</option><option>Reminder</option></select></div>
    <div class="frow"><label>Dossier</label><select id="evt-case"><option value="">— Tous —</option></select></div>
    <div class="frow"><label>Assigné à</label><input id="evt-assign"></div>
    <div class="frow"><label>Lieu</label><input id="evt-loc"></div>
    <div class="frow"><label>Priorité</label><select id="evt-prio"><option>Normal</option><option>High</option><option>Critical</option><option>Low</option></select></div>
    <div class="modal-footer"><button class="btn btn-ghost" onclick="closeModal('modal-evt')">Annuler</button><button class="btn btn-accent" onclick="createEvent()">Créer</button></div>
  </div>
</div>

<div id="modal-csm" class="modal-layer" hidden>
  <div class="modal">
    <h3>🔗 Nouvelle chaîne de garde</h3>
    <div class="frow"><label>Identifiant preuve</label><input id="csm-eid"></div>
    <div class="frow"><label>Type</label><select id="csm-type"><option>Mobile</option><option>SD Card</option><option>Disk Image</option><option>Document</option></select></div>
    <div class="frow"><label>Description</label><textarea id="csm-desc"></textarea></div>
    <div class="frow"><label>Hash SHA256</label><input id="csm-hash" style="font-family:var(--mono)"></div>
    <div class="frow"><label>Taille (octets)</label><input id="csm-size" type="number"></div>
    <div class="modal-footer"><button class="btn btn-ghost" onclick="closeModal('modal-csm')">Annuler</button><button class="btn btn-accent" onclick="createCustody()">Créer</button></div>
  </div>
</div>

<div id="modal-map" class="modal-layer" hidden>
  <div class="modal">
    <h3>🗺 Nouvelle carte</h3>
    <div class="frow"><label>Nom</label><input id="map-name"></div>
    <div class="modal-footer"><button class="btn btn-ghost" onclick="closeModal('modal-map')">Annuler</button><button class="btn btn-accent" onclick="createMap()">Créer</button></div>
  </div>
</div>

<div id="modal-flt" class="modal-layer" hidden>
  <div class="modal">
    <h3>🔎 Nouveau filtre</h3>
    <div class="frow"><label>Nom</label><input id="flt-name"></div>
    <div class="frow"><label>Date début</label><input id="flt-from" type="date"></div>
    <div class="frow"><label>Date fin</label><input id="flt-to" type="date"></div>
    <div class="frow"><label>Catégories (virgule)</label><input id="flt-cat" placeholder="MESSAGE,CALL,TRANSACTION"></div>
    <div class="frow"><label>Mots-clés (virgule)</label><input id="flt-kw" placeholder="suspect, transfert, or"></div>
    <div class="modal-footer"><button class="btn btn-ghost" onclick="closeModal('modal-flt')">Annuler</button><button class="btn btn-accent" onclick="createFilter()">Créer</button></div>
  </div>
</div>

<div id="modal-rpt" class="modal-layer" hidden>
  <div class="modal">
    <h3>📄 Générer rapport</h3>
    <div class="frow"><label>Nom</label><input id="rpt-name"></div>
    <div class="frow"><label>Type</label><select id="rpt-type"><option value="General">Rapport général</option><option value="SpecificUser">Suspect individuel</option><option value="DeviceReport">Appareil</option><option value="TimelineReport">Timeline</option><option value="CustodyReport">Chain of Custody</option><option value="MoneyFlowReport">Flux financiers</option><option value="EvidenceCatalog">Catalogue preuves</option></select></div>
    <div class="frow"><label>Format</label><select id="rpt-format"><option value="PDF">PDF</option><option value="DOCX">DOCX</option><option value="TXT">TXT</option><option value="HTML">HTML</option></select></div>
    <div class="frow"><label>Filtres JSON</label><textarea id="rpt-filters" style="font-family:var(--mono);min-height:100px">{}</textarea></div>
    <div class="modal-footer"><button class="btn btn-ghost" onclick="closeModal('modal-rpt')">Annuler</button><button class="btn btn-accent" onclick="generateReport()">✓ Générer</button></div>
  </div>
</div>
"""

html = html.replace(modals_section, new_modals)

# Remplacement CSS pour modal-layer
html = html.replace('#modal-back{', '.modal-layer{')
html = html.replace('#modal-back[hidden]{', '.modal-layer[hidden]{')

# Modifications JS
js_fixes = [
    ("openCaseModal()", "openModal('modal-case')"),
    ("openCM()", "openModal('modal-case')"),
    ("closeCM()", "closeModal('modal-case')"),
    
    ("openEventModal(){document.getElementById('modal-back').hidden=false;", "openEventModal(){openModal('modal-evt');"),
    ("closeEvt(){document.getElementById('modal-back').hidden=true}", "closeEvt(){closeModal('modal-evt')}"),
    
    ("openCustodyModal(){document.getElementById('modal-back').hidden=false;document.getElementById('modal-back')", "openCustodyModal(){openModal('modal-csm');document.getElementById('modal-csm')"),
    ("closeCSM(){document.getElementById('modal-back').hidden=true}", "closeCSM(){closeModal('modal-csm')}"),
    
    ("openMapModal(){document.getElementById('modal-back').hidden=false;document.getElementById('modal-back')", "openMapModal(){openModal('modal-map');document.getElementById('modal-map')"),
    ("closeMap(){document.getElementById('modal-back').hidden=true}", "closeMap(){closeModal('modal-map')}"),
    
    ("openFilterModal(){document.getElementById('modal-back').hidden=false;document.getElementById('modal-back')", "openFilterModal(){openModal('modal-flt');document.getElementById('modal-flt')"),
    ("closeFlt(){document.getElementById('modal-back').hidden=true}", "closeFlt(){closeModal('modal-flt')}"),
    
    ("openReportModal(){document.getElementById('modal-back').hidden=false;document.getElementById('modal-back')", "openReportModal(){openModal('modal-rpt');document.getElementById('modal-rpt')"),
    ("closeRpt(){document.getElementById('modal-back').hidden=true}", "closeRpt(){closeModal('modal-rpt')}"),
    
    # Remplacements plus larges pour modal-back non identifiés spécifiquement 
    ("#modal-back", ".modal-layer")
]

for find, repl in js_fixes:
    html = html.replace(find, repl)
    
# Ajouter la fonction openModal, closeModal, removeFilter, mods et addDays juste après 'const S=...'
add_utils = """
function openModal(id){
    document.querySelectorAll('.modal-layer').forEach(m=>m.hidden=true);
    document.getElementById(id).hidden=false;
}
function closeModal(id){
    document.getElementById(id).hidden=true;
    document.querySelectorAll(`#${id} .frow input, #${id} .frow textarea`).forEach(f=>f.value='');
}
function removeFilter(id){
    S.filters = S.filters.filter(f => f.id !== id);
    renderFilters();
    renderFilterChips();
}
function mods(){
    return [
        {id:'mobile_logical',name:'Extraction Mobile',description:'ADB/Agent logically'},
        {id:'mobile_brom',name:'BROM/EDL',description:'Extraction physique MTK/QC'},
        {id:'sdcard_image',name:'Image SD',description:'Image bit par bit SD'},
        {id:'sdcard_carve',name:'Carving SD',description:'Récupération fichiers SD'},
        {id:'disk_dd',name:'Image Disque DD',description:'Copie physique binaire'},
        {id:'disk_e01',name:'Image Disque E01',description:'Image format EnCase'},
        {id:'disk_carve',name:'Carving Disque',description:'Récupération HDD/SSD'},
        {id:'whatsapp',name:'WhatsApp',description:'Décodeur msgstore.db'},
        {id:'money_flow',name:'Money Flow',description:'Parsing transactions'},
        {id:'timeline',name:'Timeline',description:'Chronologie unifiée'},
        {id:'link_graph',name:'Graphes',description:'Réseaux relationnels'},
        {id:'reports',name:'Rapports',description:'Export chain custody'},
        {id:'csm',name:'CSM',description:'Conduite des scellés'},
        {id:'calendar',name:'Calendrier',description:'Planning'},
        {id:'cartography',name:'Cartographie',description:'Cartes geo'},
        {id:'filters',name:'Filtres',description:'Recherche'},
        {id:'reports_gen',name:'Rapports Doc',description:'PDF/Word'}
    ];
}
function addDays(dateStr, days){
    const d = new Date(dateStr);
    d.setDate(d.getDate() + days);
    return d.toISOString().slice(0,10);
}
"""

html = html.replace("const S={", add_utils + "\nconst S={")

with open("index.html", "w", encoding="utf-8") as f:
    f.write(html)
