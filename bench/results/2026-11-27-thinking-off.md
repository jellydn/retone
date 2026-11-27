# Model bench, 2026-11-27-thinking-off

Apple M1 Max, 32 GB. Every style on every sample of
`retone --eval-styles`, 1 or 3 runs per model, through
`http://127.0.0.1:9940/v1`. Times are medians per rewrite.

| Model | Runs | Pass | Failed | Invented | Dropped | Concise | Expand | First token | Total (p90) | RAM | File |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Qwen/Qwen2.5-7B-Instruct-GGUF:Q4_K_M | 3 | 99% | 0% | 0% | 3% | 54% | 143% | 0.25 s | 1.21 s (3.24 s) | 5.3 GB | 4.7 GB |
| bartowski/Meta-Llama-3.1-8B-Instruct-GGUF:Q4_K_M | 3 | 98% | 0% | 0% | 6% | 55% | 161% | 0.26 s | 1.31 s (3.45 s) | 6 GB | 4.9 GB |
| ggml-org/LFM3-Omni-2B-GGUF:Q8_0 | 3 | 94% | 0% | 0% | 3% | 29% | 121% | 0.21 s | 0.56 s (1.16 s) | 2.8 GB | 1.8 GB |
| ggml-org/gemma-4-E4B-it-GGUF:Q4_0 | 3 | 94% | 0% | 0% | 3% | 69% | 172% | 0.19 s | 0.87 s (2.17 s) | 5.7 GB | 4.6 GB |
| unsloth/gemma-4-E4B-it-GGUF:Q4_K_M | 3 | 94% | 0% | 0% | 3% | 60% | 175% | 0.23 s | 1.09 s (2.65 s) | 6.6 GB | 5 GB |
| Qwen/Qwen2.5-3B-Instruct-GGUF:Q4_K_M | 3 | 92% | 0% | 3% | 9% | 26% | 120% | 0.12 s | 0.63 s (1.7 s) | 2.6 GB | 2.1 GB |
| ggml-org/Qwen3.5-4B-GGUF:Q4_K_M | 3 | 92% | 0% | 0% | 0% | 48% | 158% | 0.18 s | 0.9 s (2.53 s) | 5.7 GB | 3 GB |
| bartowski/microsoft_Phi-4-mini-instruct-GGUF:Q4_K_M | 3 | 91% | 1% | 0% | 2% | 47% | 176% | 0.15 s | 0.82 s (2.08 s) | 4.5 GB | 2.5 GB |
| bartowski/Llama-3.2-3B-Instruct-GGUF:Q4_K_M | 3 | 90% | 0% | 3% | 6% | 51% | 175% | 0.13 s | 0.72 s (1.71 s) | 3.4 GB | 2 GB |
| LiquidAI/LFM2.5-2.6B-GGUF:Q4_K_M | 1 | 89% | 3% | 3% | 3% | 46% | 184% | 5.49 s | 5.92 s (19.27 s) | 2.2 GB | 1.7 GB |
| ggml-org/gemma-4-12B-it-GGUF:Q4_0 | 3 | 87% | 0% | 0% | 0% | 64% | 201% | 0.46 s | 1.66 s (4.46 s) | 14.2 GB | 7.2 GB |
| ibm-granite/granite-4.2-3b-GGUF:Q4_K_M | 3 | 86% | 1% | 2% | 0% | 51% | 254% | 0.14 s | 0.75 s (2.09 s) | 3.4 GB | 2.2 GB |
| unsloth/Qwen3-4B-Instruct-2507-GGUF:Q4_K_M | 3 | 86% | 0% | 3% | 0% | 46% | 238% | 0.16 s | 0.83 s (2.18 s) | 4.8 GB | 2.5 GB |
| ggml-org/gpt-oss-20b-GGUF:MXFP4 | 1 | 86% | 0% | 0% | 3% | 63% | 179% | 5.8 s | 6.35 s (33.29 s) | 12.4 GB | 12.1 GB |
| ggml-org/gemma-4-E2B-it-GGUF:Q8_0 | 3 | 85% | 0% | 3% | 6% | 46% | 232% | 0.12 s | 0.74 s (1.8 s) | 5.2 GB | 5 GB |
| unsloth/gemma-4-E2B-it-GGUF:Q4_K_M | 3 | 83% | 0% | 3% | 6% | 54% | 237% | 0.13 s | 0.65 s (1.62 s) | 4.4 GB | 3.1 GB |
| ggml-org/Ministral-3-8B-Instruct-2512-GGUF:Q8_0 | 3 | 81% | 0% | 9% | 6% | 47% | 249% | 0.2 s | 1.58 s (4.39 s) | 9.5 GB | 9 GB |
| ggml-org/Qwen3.5-0.8B-GGUF:Q8_0 | 3 | 77% | 0% | 2% | 4% | 44% | 202% | 0.05 s | 0.32 s (0.84 s) | 2.3 GB | 0.8 GB |
| mistralai/Ministral-3-3B-Instruct-2512-GGUF:Q4_K_M | 3 | 74% | 1% | 7% | 5% | 54% | 230% | 0.13 s | 0.93 s (2.71 s) | 4 GB | 2.1 GB |
| LiquidAI/LFM3-1.7B-GGUF:Q4_K_M | 3 | 54% | 17% | 12% | 7% | 122% | 204% | 0.25 s | 0.96 s (1.9 s) | 1.5 GB | 1.1 GB |
| ggml-org/LFM3-1.7B-GGUF:Q8_0 | 3 | 43% | 31% | 20% | 5% | 91% | 225% | 0.22 s | 1.09 s (2.96 s) | 2.2 GB | 1.8 GB |

Pass: answers that met every check of their style. Failed: requests with no answer, mostly the 60 s limit.
Invented: answers with a number the text does not have, or a detail copied from the prompt's example.
Dropped: answers that lost a number of the text (Summarize is not counted). Concise and Expand: median length
against the text (targets: at most 70%, and 120% to 200%). A model slower than 5 s per rewrite gets 1 run only.

## Top 3

- **fastest**: Qwen/Qwen2.5-3B-Instruct-GGUF:Q4_K_M
- **smallest**: bartowski/Llama-3.2-3B-Instruct-GGUF:Q4_K_M
- **quality**: Qwen/Qwen2.5-7B-Instruct-GGUF:Q4_K_M
