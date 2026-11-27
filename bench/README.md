# Model bench

`scripts/model-bench.sh` runs every Retone style on the sample texts of `retone --eval-styles` with each model, three times, and ranks the models. Use it again when new models come out, and update the recommended models in Settings and the README from its result.

## Run it

1. Start a server that loads a model when Retone asks for it:
   - **Llama.app**: install the models in Llama.app. The script uses Llama.app's address by default.
   - **llama-server** in router mode serves every model you downloaded:

     ```sh
     llama download -hf unsloth/Qwen3-4B-Instruct-2507-GGUF:Q4_K_M
     llama-server --port 9940 --models-max 1 --ctx-size 4096
     ```

     `--models-max 1` keeps one model in memory, so the RAM column shows that model only.

   - **Ollama** or **LM Studio**: pull the models first. RAM and file size stay empty for them.

2. Run the bench:

   ```sh
   sh scripts/model-bench.sh                                     # bench/models.txt, Llama.app
   sh scripts/model-bench.sh --endpoint http://127.0.0.1:9940/v1 # llama-server
   sh scripts/model-bench.sh --endpoint http://127.0.0.1:11434/v1 --models my-ollama-models.txt
   sh scripts/model-bench.sh --runs 1 unsloth/gemma-4-E2B-it-GGUF:Q4_K_M  # one model, one run
   ```

3. Read `bench/results/DATE.md` (the table and the top 3) and `DATE.json` (all numbers, also per style). Each model's answers are in `target/bench/DATE/`; read some of them before you trust a number. The checks cannot see every wrong meaning.

## Add a model

Add its ID to `bench/models.txt`, one per line, as the server names it: `user/repo:Q4_K_M` for Llama.app and llama-server, `hf.co/user/repo:Q4_K_M` or a tag such as `qwen2.5:3b` for Ollama.

## What it measures

| Column      | Meaning                                                                                                                                                             |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Pass        | Answers that met every check of their style: Concise at most 70% of the length, Fix close to the text, Summarize at most half, Expand 120% to 200%, Bullets a list. |
| Invented    | Answers with a number the text does not have, or a detail copied from the prompt's example (Safari, login, yesterday, …).                                           |
| Dropped     | Answers that lost a number of the text. Summarize is not counted, as a summary may leave out details.                                                               |
| First token | Median time to the first word of the answer. A model that thinks first is slow here: Retone shows only the answer, not the thinking.                                |
| Total (p90) | Median time per rewrite, and the time 90% of rewrites finished in. The first request is not counted, because it can load the model.                                 |
| RAM         | Resident memory of the server process that holds the model, after the run.                                                                                          |

A model **qualifies** when it answered every request, passed at least 85% of the checks and invented a detail in at most 5% of its answers. The top 3:

- **fastest**: the qualifying model with the lowest median time.
- **quality**: the highest pass rate, then the fewest invented and dropped details.
- **balance**: of the other models, the highest pass rate among those at most twice as slow as the fastest.

Times depend on the Mac. Write the Mac and its memory next to a result you share.
