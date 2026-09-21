// Génération de rapports PDF forensiques RÉELS (printpdf, polices standard PDF).
// Produit un vrai fichier .pdf : en-tête dossier, preuves + SHA256, statistiques,
// échantillons de données, et sceau d'intégrité (SHA256 du rapport lui-même).

use printpdf::{BuiltinFont, IndirectFontRef, Mm, PdfDocument, PdfDocumentReference, PdfLayerIndex, PdfPageIndex};
use sha2::{Digest, Sha256};
use std::io::BufWriter;

const PAGE_W: f32 = 210.0;
const PAGE_H: f32 = 297.0;
const MARGIN_X: f32 = 18.0;
const TOP: f32 = 282.0;
const BOTTOM: f32 = 16.0;

#[derive(Clone, Copy)]
enum F { Reg, Bold, Mono }

struct Report {
    doc: PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    y: f32,
    reg: IndirectFontRef,
    bold: IndirectFontRef,
    mono: IndirectFontRef,
}

impl Report {
    fn new(title: &str) -> Result<Self, String> {
        let (doc, page, layer) = PdfDocument::new(title, Mm(PAGE_W), Mm(PAGE_H), "Layer 1");
        let reg = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| e.to_string())?;
        let bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|e| e.to_string())?;
        let mono = doc.add_builtin_font(BuiltinFont::Courier).map_err(|e| e.to_string())?;
        Ok(Report { doc, page, layer, y: TOP, reg, bold, mono })
    }

    fn font(&self, f: F) -> &IndirectFontRef {
        match f { F::Reg => &self.reg, F::Bold => &self.bold, F::Mono => &self.mono }
    }

    fn newpage(&mut self) {
        let (p, l) = self.doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Layer 1");
        self.page = p;
        self.layer = l;
        self.y = TOP;
    }

    /// Écrit une ligne de texte, gère le saut de page automatique.
    fn line(&mut self, s: &str, size: f32, f: F) {
        let lh = size * 0.40 + 1.6; // hauteur de ligne approximative en mm
        if self.y - lh < BOTTOM { self.newpage(); }
        let font = self.font(f).clone();
        let layer = self.doc.get_page(self.page).get_layer(self.layer);
        layer.use_text(s, size, Mm(MARGIN_X), Mm(self.y), &font);
        self.y -= lh;
    }

    fn gap(&mut self, mm: f32) {
        self.y -= mm;
        if self.y < BOTTOM { self.newpage(); }
    }

    fn heading(&mut self, s: &str) {
        self.gap(3.0);
        self.line(s, 13.0, F::Bold);
        self.gap(1.5);
    }

    fn finish(self, path: &str) -> Result<Vec<u8>, String> {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut w = BufWriter::new(&mut buf);
            self.doc.save(&mut w).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, &buf).map_err(|e| format!("Écriture {}: {}", path, e))?;
        Ok(buf)
    }
}

fn trunc(s: &str, n: usize) -> String {
    let s: String = s.chars().filter(|c| !c.is_control()).collect();
    if s.chars().count() <= n { s } else { format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>()) }
}

fn s_of<'a>(v: &'a serde_json::Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("")
}

/// Génère un rapport PDF forensique à partir du résultat d'extraction.
#[tauri::command]
pub fn generate_pdf_report(
    case_ref: String,
    case_name: String,
    operator: String,
    extraction: serde_json::Value,
    output_path: String,
) -> Result<serde_json::Value, String> {
    let mut r = Report::new("FIDO-LABS - Rapport Forensique")?;

    // ── En-tête ──────────────────────────────────────────────
    r.line("FIDO-LABS v3", 20.0, F::Bold);
    r.line("RAPPORT FORENSIQUE NUMÉRIQUE", 13.0, F::Bold);
    r.gap(3.0);
    r.line(&format!("Dossier      : {}  —  {}", trunc(&case_ref, 40), trunc(&case_name, 50)), 11.0, F::Reg);
    r.line(&format!("Opérateur    : {}", trunc(&operator, 60)), 11.0, F::Reg);
    r.line(&format!("Date rapport : {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")), 11.0, F::Reg);
    if !s_of(&extraction, "serial").is_empty() {
        r.line(&format!("Appareil     : {}", s_of(&extraction, "serial")), 11.0, F::Reg);
    }
    r.line(&format!("Type         : {}", if s_of(&extraction, "manifest_sha256").is_empty() { "—" } else { "Extraction logique ADB" }), 11.0, F::Reg);

    // ── 1. Résumé ────────────────────────────────────────────
    r.heading("1. Résumé de l'acquisition");
    let summary = s_of(&extraction, "summary");
    if !summary.is_empty() { r.line(&trunc(summary, 90), 10.0, F::Reg); }
    let outdir = s_of(&extraction, "output_dir");
    if !outdir.is_empty() { r.line(&format!("Dossier de preuves : {}", trunc(outdir, 80)), 9.0, F::Reg); }
    let seal = s_of(&extraction, "manifest_sha256");
    if !seal.is_empty() { r.line(&format!("Sceau du manifeste (SHA256) : {}", seal), 9.0, F::Mono); }

    // ── 2. Preuves + SHA256 ──────────────────────────────────
    r.heading("2. Preuves collectées et empreintes SHA256");
    r.line(&format!("{:<32}{:>7}  {:<10} {}", "Source", "Élém.", "Statut", "SHA256"), 9.0, F::Mono);
    r.line(&"-".repeat(92), 9.0, F::Mono);
    if let Some(arts) = extraction.get("artifacts").and_then(|a| a.as_array()) {
        for a in arts {
            let name = trunc(s_of(a, "name"), 31);
            let count = a.get("count").and_then(|c| c.as_i64()).unwrap_or(0);
            let status = trunc(s_of(a, "status"), 10);
            let sha = s_of(a, "sha256");
            let sha_disp = if sha.is_empty() { "—".to_string() } else { format!("{}…", &sha[..sha.len().min(24)]) };
            r.line(&format!("{:<32}{:>7}  {:<10} {}", name, count, status, sha_disp), 9.0, F::Mono);
        }
    } else {
        r.line("Aucune preuve (aucune extraction fournie).", 10.0, F::Reg);
    }

    // ── 3. Statistiques ──────────────────────────────────────
    r.heading("3. Statistiques des données");
    let totals = extraction.get("totals").cloned().unwrap_or(serde_json::json!({}));
    let g = |k: &str| totals.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
    r.line(&format!("SMS      : {}", g("sms")), 10.0, F::Mono);
    r.line(&format!("Appels   : {}", g("calls")), 10.0, F::Mono);
    r.line(&format!("Contacts : {}", g("contacts")), 10.0, F::Mono);

    // ── 4. Échantillon SMS ───────────────────────────────────
    if let Some(sms) = extraction.get("sms").and_then(|s| s.as_array()) {
        if !sms.is_empty() {
            r.heading("4. Échantillon de messages (SMS)");
            r.line(&format!("{:<20}{:<16}{:<9}{}", "Date", "Contact", "Sens", "Message"), 8.5, F::Mono);
            r.line(&"-".repeat(92), 8.5, F::Mono);
            for m in sms.iter().take(30) {
                let d = trunc(s_of(m, "date"), 19);
                let c = trunc(s_of(m, "contact"), 15);
                let dir = trunc(s_of(m, "direction"), 8);
                let body = trunc(s_of(m, "body"), 46);
                r.line(&format!("{:<20}{:<16}{:<9}{}", d, c, dir, body), 8.5, F::Mono);
            }
        }
    }

    // ── 5. Intégrité / chaîne de garde ───────────────────────
    r.heading("5. Intégrité et chaîne de garde");
    r.line("Chaque preuve est scellée par une empreinte SHA256 calculée à l'acquisition.", 9.5, F::Reg);
    r.line("Toute altération d'un fichier modifie son empreinte et invalide la preuve.", 9.5, F::Reg);
    r.line("Usage légal uniquement, avec autorisation judiciaire.", 9.5, F::Reg);
    r.gap(4.0);
    r.line(&format!("Rapport généré par FIDO-LABS v3 — {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")), 9.0, F::Reg);
    r.line("Signature opérateur : ______________________________", 10.0, F::Reg);

    // ── Sauvegarde + sceau du rapport ────────────────────────
    let bytes = r.finish(&output_path)?;
    let mut h = Sha256::new();
    h.update(&bytes);
    let report_sha = hex::encode(h.finalize());

    Ok(serde_json::json!({
        "path": output_path,
        "sha256": report_sha,
        "size": bytes.len(),
        "summary": format!("Rapport PDF généré ({} Ko) — SHA256 {}…", bytes.len() / 1024, &report_sha[..16]),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genere_un_pdf_valide() {
        let extraction = serde_json::json!({
            "summary": "Extraction logique terminée — 7/8 sources OK",
            "serial": "TESTSERIAL123",
            "output_dir": "/tmp/fido_test",
            "manifest_sha256": "a3f8c1d2e4b7f0129a5b6c7d8e9f0011223344556677889900aabbccddeeff00",
            "artifacts": [
                {"name":"SMS","count":342,"status":"OK","sha256":"99aa88bb77cc6600dd11ee22ff334455"},
                {"name":"Contacts","count":96,"status":"OK","sha256":"0f1e2d3c4b5a6978aabbccddeeff0011"}
            ],
            "totals": {"sms":342,"calls":128,"contacts":96},
            "sms": [{"date":"2024-03-15 08:23:11","contact":"+22670123456","direction":"Reçu","body":"Salut, tu es dispo ce soir ?"}]
        });
        let out = std::env::temp_dir().join("fido_test_report.pdf");
        let out_s = out.to_string_lossy().to_string();
        let res = generate_pdf_report(
            "REF-2024-001".into(), "Affaire Test".into(), "Investigateur".into(),
            extraction, out_s.clone(),
        ).expect("génération PDF");
        let bytes = std::fs::read(&out_s).expect("fichier PDF créé");
        assert!(bytes.starts_with(b"%PDF"), "doit être un vrai PDF (%PDF-...)");
        assert!(bytes.len() > 1000, "PDF anormalement petit");
        assert_eq!(res["sha256"].as_str().unwrap().len(), 64, "SHA256 du rapport");
        let _ = std::fs::remove_file(&out_s);
    }
}
