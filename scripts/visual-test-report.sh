#!/usr/bin/env bash
# Generates a visual HTML report from `cargo test` output.
set -euo pipefail

INPUT="${1:-test-output.log}"
REPORT="${2:-test-report.html}"

PASSED=$(grep -E '^test .* \.\.\. ok$' "$INPUT" | wc -l || true)
FAILED=$(grep -E '^test .* \.\.\. FAILED$' "$INPUT" | wc -l || true)
IGNORED=$(grep -E '^test .* \.\.\. ignored$' "$INPUT" | wc -l || true)

if [ "$FAILED" -eq 0 ]; then
  STATUS="PASS"
  COLOR="#22c55e"
else
  STATUS="FAIL"
  COLOR="#ef4444"
fi

cat > "$REPORT" <<EOF
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Test Report</title>
  <style>
    body { font-family: system-ui, sans-serif; margin: 2rem; background: #0f172a; color: #e2e8f0; }
    .card { background: #1e293b; border-radius: 1rem; padding: 1.5rem; max-width: 600px; margin: 0 auto; box-shadow: 0 10px 30px rgba(0,0,0,0.3); }
    h1 { margin-top: 0; }
    .status { font-size: 3rem; font-weight: bold; color: $COLOR; }
    .stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1rem; margin-top: 1rem; }
    .stat { background: #334155; padding: 1rem; border-radius: 0.5rem; text-align: center; }
    .stat .number { font-size: 2rem; font-weight: bold; }
    pre { background: #0f172a; padding: 1rem; border-radius: 0.5rem; overflow-x: auto; }
  </style>
</head>
<body>
  <div class="card">
    <h1>Standard-Tools Rust Test Report</h1>
    <div class="status">$STATUS</div>
    <div class="stats">
      <div class="stat"><div class="number">$PASSED</div><div>Passed</div></div>
      <div class="stat"><div class="number">$FAILED</div><div>Failed</div></div>
      <div class="stat"><div class="number">$IGNORED</div><div>Ignored</div></div>
    </div>
    <h2>Raw Output</h2>
    <pre>$(sed 's/&/\&amp;/g; s/</\&lt;/g; s/>/\&gt;/g' "$INPUT")</pre>
  </div>
</body>
</html>
EOF

echo "Report written to $REPORT"

if [ "$FAILED" -gt 0 ]; then
  exit 1
fi
