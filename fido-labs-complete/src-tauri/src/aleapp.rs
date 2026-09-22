// Analyse forensique d'une extraction Android via ALEAPP (abrignoni/ALEAPP).
// ALEAPP parse un dossier/tar/zip/gz d'extraction et produit un rapport HTML pro
// (comptes, appels, SMS, apps installées, localisations, historiques, centaines
// d'artefacts). On l'invoque comme outil externe (venv Python 3.12) et on récupère
// le chemin du rapport index.html généré.

/// Localise le python du venv et le script aleapp.py.
fn resolve_aleapp() -> Result<(String, String), String> {
    if let Ok(script) = std::env::var("FIDO_ALEAPP_SCRIPT") {
        if std::path::Path::new(&script).exists() {
            let py = std::env::var("FIDO_ALEAPP_PYTHON").unwrap_or_else(|_| "python".into());
            return Ok((py, script));
        }
    }
    let candidates = [
        ("E:/fido-labs-complete/fido-labs-complete/tools/aleapp-venv/Scripts/python.exe",
         "E:/fido-labs-complete/fido-labs-complete/tools/ALEAPP-main/aleapp.py"),
        ("tools/aleapp-venv/Scripts/python.exe", "tools/ALEAPP-main/aleapp.py"),
    ];
    for (py, script) in candidates {
        if std::path::Path::new(script).exists() {
            let python = if std::path::Path::new(py).exists() { py.to_string() } else { "python".into() };
            return Ok((python, script.to_string()));
        }
    }
    Err("ALEAPP introuvable (tools/ALEAPP-main) — définissez FIDO_ALEAPP_SCRIPT.".into())
}

/// Parse une extraction Android via ALEAPP → rapport HTML forensique.
/// input_type : "fs" (dossier), "tar", "zip", "gz".
#[tauri::command]
pub fn parse_with_aleapp(input_path: String, input_type: String, output_dir: String) -> Result<serde_json::Value, String> {
    let (python, script) = resolve_aleapp()?;
    if !std::path::Path::new(&input_path).exists() {
        return Err(format!("Entrée introuvable: {}", input_path));
    }
    let itype = match input_type.as_str() {
        "fs" | "tar" | "zip" | "gz" => input_type.as_str(),
        _ => "fs",
    };
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let out = format!("{}/aleapp_{}", output_dir.trim_end_matches(|c| c == '/' || c == '\\'), ts);
    std::fs::create_dir_all(&out).map_err(|e| format!("Création {}: {}", out, e))?;

    let script_dir = std::path::Path::new(&script).parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let output = std::process::Command::new(&python)
        .arg(&script)
        .args(["-t", itype, "-i", &input_path, "-o", &out])
        .current_dir(&script_dir)
        .output()
        .map_err(|e| format!("Lancement ALEAPP échoué ({}): {}", python, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    std::fs::write(format!("{}/aleapp_run.log", out), format!("=== STDOUT ===\n{}\n=== STDERR ===\n{}", stdout, stderr)).ok();

    // ALEAPP crée un sous-dossier avec index.html — on le retrouve.
    let mut report = String::new();
    for entry in walkdir::WalkDir::new(&out).max_depth(3).into_iter().flatten() {
        if entry.file_type().is_file()
            && entry.file_name().to_string_lossy().eq_ignore_ascii_case("index.html")
        {
            report = entry.path().to_string_lossy().to_string();
            break;
        }
    }

    let ok = output.status.success() && !report.is_empty();
    let n = stdout.chars().count();
    let excerpt: String = stdout.chars().skip(n.saturating_sub(1800)).collect();
    Ok(serde_json::json!({
        "success": ok,
        "summary": if ok {
            "Rapport ALEAPP généré".to_string()
        } else {
            "ALEAPP n'a pas produit de rapport — vérifiez l'entrée et le log.".to_string()
        },
        "report": report,
        "output_dir": out,
        "log": excerpt,
    }))
}
