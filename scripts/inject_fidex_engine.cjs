const fs = require('fs');

console.log("Injecting fidex_engine.js into index.html...");
let html = fs.readFileSync('index.html', 'utf8');
const engineCode = fs.readFileSync('scripts/fidex_engine.js', 'utf8');

if (!html.includes('FIDEX (FIDO-LABS-EXTRACTION) FORENSIC ENGINE v3.2')) {
  html = html.replace('</script>', `\n\n// INJECTED FIDEX ENGINE\n${engineCode}\n</script>`);
}

// Ensure slices in pie chart call openSliceDetailsModal
html = html.replace(
  /onclick="filterFromSlice\('\$\{esc\(slice\.label\)\}'\)"/g,
  `onclick="openSliceDetailsModal(dataset.id || chartIdx, sIdx, slice.label);event.stopPropagation()"`
);

html = html.replace(
  /onclick="filterFromSlice\('\$\{esc\(s\.label\)\}'\)"/g,
  `onclick="openSliceDetailsModal(dataset.id || chartIdx, idx, s.label);event.stopPropagation()"`
);

fs.writeFileSync('index.html', html, 'utf8');
console.log("Successfully injected into index.html!");
