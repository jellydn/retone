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
   sh scripts/model-bench.sh --name new-prompt                   # results under a name, not the date
   ```

3. Read `bench/results/NAME.md` (the table and the top 3) and `NAME.json` (all numbers, also per style). NAME is today's date unless you pass `--name`. Each model's answers are in `target/bench/NAME/`; read some of them before you trust a number. The checks cannot see every wrong meaning.

## Add a model

Add its ID to `bench/models.txt`, one per line, as the server names it: `user/repo:Q4_K_M` for Llama.app and llama-server, `hf.co/user/repo:Q4_K_M` or a tag such as `qwen2.5:3b` for Ollama.

## What it measures

| Column      | Meaning                                                                                                                                                             |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Pass        | Answers that met every check of their style: Concise at most 70% of the length, Fix close to the text, Summarize at most half, Expand 120% to 200%, Bullets a list. |
| Invented    | Answers with a number the text does not have, or a detail copied from the prompt's example (Safari, login, yesterday, …).                                           |
| Dropped     | Answers that lost a number of the text. Summarize is not counted, as a summary may leave out details.                                                               |
| First token | Median time to the first word of the answer. Retone asks the server not to think first (`reasoning_effort: "none"`); a model that thinks anyway is slow here.       |
| Total (p90) | Median time per rewrite, and the time 90% of rewrites finished in. The first request is not counted, because it can load the model.                                 |
| RAM         | Resident memory of the server process that holds the model, after the run.                                                                                          |

A model **qualifies** when it answered every request, passed at least 85% of the checks, invented a detail in at most 5% of its answers, kept the five tones apart on every sample, and took at most 5 s per rewrite. The top 3:

Speed and size come first:

- **fastest**: the qualifying model with the lowest median time.
- **smallest**: of the others, the one that uses the least memory.
- **quality**: of the others, the highest pass rate among models at most twice as slow as the fastest.

Times depend on the Mac. Write the Mac and its memory next to a result you share.
