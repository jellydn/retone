#!/bin/sh
# Runs `retone --eval-styles` on each model and ranks them for Retone's styles.
#
#   sh scripts/model-bench.sh [--endpoint URL] [--runs N] [--models FILE] [--name NAME] [MODEL ...]
#
# Models come from the arguments, or from FILE (default bench/models.txt: one model ID per
# line, `#` starts a comment). The server must be able to load each one on request: Llama.app,
# a llama-server router, Ollama or LM Studio. Answers go to target/bench/NAME/. The table and the
# JSON go to bench/results/NAME.md and NAME.json and rank every model benched under that name, so
# models can be benched one at a time; delete target/bench/NAME to start over. NAME: today's date.
set -eu
cd "$(dirname "$0")/.."

endpoint=http://127.0.0.1:9931/v1
runs=3
models_file=bench/models.txt
# A median rewrite slower than this is too slow for the popup: such a model keeps its first run
# and does not qualify, which saves the time of the other runs.
slow_s=5
while [ $# -gt 0 ]; do
  case $1 in
    --endpoint) endpoint=$2; shift 2 ;;
    --runs) runs=$2; shift 2 ;;
    --models) models_file=$2; shift 2 ;;
    --name) name=$2; shift 2 ;;
    -h|--help) sed -n '2,10p' "$0"; exit 0 ;;
    *) break ;;
  esac
done
if [ $# -eq 0 ]; then
  set -- $(sed 's/#.*//' "$models_file")
fi

cargo build -q
date=$(date +%Y-%m-%d)
name=${name:-$date}
case $name in
  *[!A-Za-z0-9._-]*) echo "--name: use only letters, digits, '.', '_' and '-'" >&2; exit 1 ;;
esac
raw=target/bench/$name
mkdir -p "$raw" bench/results
results=bench/results/$name

for model in "$@"; do
  slug=$(printf '%s' "$model" | tr -c 'A-Za-z0-9._-' '_')
  echo "== $model"
  rm -f "$raw/$slug.json"
  # A failed eval still writes its JSON: it only means some answers missed a check.
  target/debug/retone --eval-styles --model "$model" --endpoint "$endpoint" --runs 1 \
    --json "$raw/$slug.json" > "$raw/$slug.md" || true
  if [ ! -s "$raw/$slug.json" ]; then
    echo "   skipped: no result (see $raw/$slug.md)"
    continue
  fi
  if [ "$runs" -gt 1 ] && jq -e --argjson slow "$slow_s" '.total_s <= $slow' "$raw/$slug.json" > /dev/null; then
    # Kept only when complete: a stopped run must not leave the one-run result looking like it.
    rm -f "$raw/$slug.runs.json"
    target/debug/retone --eval-styles --model "$model" --endpoint "$endpoint" --runs "$runs" \
      --json "$raw/$slug.runs.json" > "$raw/$slug.runs.md" || true
    if jq -e --argjson runs "$runs" '.runs == $runs' "$raw/$slug.runs.json" > /dev/null 2>&1; then
      mv "$raw/$slug.runs.json" "$raw/$slug.json"
      mv "$raw/$slug.runs.md" "$raw/$slug.md"
    else
      echo "   the $runs-run eval did not finish: keeping the 1-run result"
      rm -f "$raw/$slug.runs.json"
    fi
  fi
  # The resident memory of the server process that holds this model. Llama.app and a llama-server
  # router start it with `--alias ID`; Ollama does not, so RAM stays empty there.
  rss_kb=$(ps -axo rss=,args= | grep -F -- "--alias $model " | grep -v grep | awk 'NR == 1 {print $1}')
  # The GGUF size, for a user/repo:QUANT ID in the Hugging Face cache (all parts of a split file).
  repo=${model%%:*}
  quant=${model##*:}
  size=$(find "$HOME/.cache/huggingface/hub/models--$(printf '%s' "$repo" | sed 's|/|--|')/snapshots" \
    -iname "*$quant*.gguf" ! -iname 'mmproj*' -exec stat -L -f %z {} + 2>/dev/null | awk '{s += $1} END {print s+0}')
  jq --argjson rss "${rss_kb:-0}" --argjson size "$size" --arg endpoint "$endpoint" \
    '. + {endpoint: $endpoint, ram_gb: ($rss / 1048576), file_gb: ($size / 1e9)}' "$raw/$slug.json" > "$raw/$slug.tmp"
  mv "$raw/$slug.tmp" "$raw/$slug.json"
  jq -r '"   pass \(.pass_rate * 100 | round)%  total \(.total_s * 100 | round / 100) s"' "$raw/$slug.json"
done

# Ranking. A model qualifies when it answered every request, passed at least 85% of the checks,
# invented a detail in at most 5% of its answers, kept the tones apart on every sample, and took at
# most $slow_s s per rewrite (median).
# Speed and size come first: fastest is the qualifying model with the lowest median time; smallest,
# of the others, uses the least memory (a model with no RAM figure, as on Ollama, is skipped);
# quality, of the others, has the highest pass rate (then fewest invented and dropped numbers)
# among models at most twice as slow as the fastest.
jq -s '
  def qualifies: .errors == 0 and .pass_rate >= 0.85 and .invented_rate <= 0.05 and .tone_spread_failures == 0 and .total_s <= '"$slow_s"';
  (map(select(qualifies))) as $ok
  | ($ok | min_by(.total_s)) as $fast
  | ($ok | map(select(.model != $fast.model and .ram_gb > 0)) | min_by(.ram_gb)) as $small
  | ($ok | map(select(.model != $fast.model and .model != $small.model and .total_s <= 2 * ($fast.total_s // 0)))
        | sort_by(-.pass_rate, .invented_rate, .dropped_rate, .total_s) | first) as $best
  | {date: "'"$date"'",
     top: [($fast | select(.) | {pick: "fastest", model}),
           ($small | select(.) | {pick: "smallest", model}),
           ($best | select(.) | {pick: "quality", model})],
     models: sort_by(-.pass_rate, .total_s)}
' $(ls "$raw"/*.json | grep -v '\.runs\.json$') > "$results.json"

{
  echo "# Model bench, $name"
  echo
  echo "$(sysctl -n machdep.cpu.brand_string), $(($(sysctl -n hw.memsize) / 1073741824)) GB. Every style on every sample of"
  echo "\`retone --eval-styles\`, $(jq -r '[.models[].runs] | unique | join(" or ")' "$results.json") runs per model, through"
  echo "$(jq -r '[.models[].endpoint] | unique | map("`" + . + "`") | join(", ")' "$results.json"). Times are medians per rewrite."
  echo
  echo "| Model | Runs | Pass | Failed | Invented | Dropped | Concise | Expand | First token | Total (p90) | RAM | File |"
  echo "|---|---|---|---|---|---|---|---|---|---|---|---|"
  jq -r '
    def pct: . * 100 | round | tostring + "%";
    def s: . * 100 | round / 100 | tostring + " s";
    def gb: if . > 0 then (. * 10 | round / 10 | tostring + " GB") else "-" end;
    def ratio: if . > 0 then (. * 100 | round | tostring + "%") else "-" end;
    .models[] | "| \(.model) | \(.runs) | \(.pass_rate | pct) | \(.errors / .cells | pct) | \(.invented_rate | pct) | \(.dropped_rate | pct) | \(.styles.concise.length_ratio | ratio) | \(.styles.expand.length_ratio | ratio) | \(.first_token_s | s) | \(.total_s | s) (\(.total_p90_s | s)) | \(.ram_gb | gb) | \(.file_gb | gb) |"
  ' "$results.json"
  echo
  echo "Pass: answers that met every check of their style. Failed: requests with no answer, mostly the 60 s limit."
  echo "Invented: answers with a number the text does not have, or a detail copied from the prompt's example."
  echo "Dropped: answers that lost a number of the text (Summarize and TL;DR are not counted). Concise and Expand: median length"
  echo "against the text (targets: at most 70%, and 120% to 200%). A model slower than $slow_s s per rewrite gets 1 run only."
  echo
  echo "## Top 3"
  echo
  jq -r '.top[] | "- **\(.pick)**: \(.model)"' "$results.json"
} > "$results.md"

cat "$results.md"
