# 0010. Recommend local models from a repeatable bench

Date: 2026-11-27

## Status

Accepted

## Context

Local models differ a lot in how well they do Retone's styles. A small model can return almost the same text for every style, add facts, or ignore the length targets of Concise and Expand. A model that thinks before it answers can take 5 to 50 seconds per rewrite, because Retone waits for the thinking to end and shows only the answer. Users had no way to know which model to pick, and new models come out every month.

## Decision

- `retone --eval-styles` gets `--runs N` and `--json FILE`. The JSON has the pass rate, the share of answers with an invented detail (a new number, or a detail copied from a prompt example) or a dropped number, the length ratio per style, and the first-token and total times. One untimed request runs first, so model load time is not counted.
- `scripts/model-bench.sh` runs the eval on a list of models (`bench/models.txt`) against any OpenAI-compatible server, adds RAM and file size, and writes `bench/results/DATE.md` and `DATE.json`. A model slower than 5 s per rewrite gets one run only.
- A model qualifies when it answered every request, passed at least 85% of the checks, invented a detail in at most 5% of its answers, kept the tones apart on every sample, and took at most 5 s per rewrite. The top 3 are the fastest, the best balance (highest pass rate at most twice as slow as the fastest) and the best quality (highest pass rate).
- The top 3 are a constant in `provider::local` and show in **Settings → Provider → Recommended models**. **Use** fills in the model ID for the current server: `hf.co/ID` for Ollama, the Hugging Face ID for Llama.app and llama-server.

## Consequences

- The list is data from one Mac (an M1 Max). Times on another Mac differ, but the order of fast and slow models stays about the same.
- The checks cannot see every wrong meaning (for example "moved the sync" for "talked about moving it"). Read some answers in `target/bench/DATE/` before changing the list.
- To update the list: add the new models to `bench/models.txt`, run the bench, and change `RECOMMENDED`, the locale lines and the README table in one PR with the new results file.
- The LM Studio ID of a model is not its Hugging Face ID, so for LM Studio the user downloads the model and picks it with **List**.
