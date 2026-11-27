# 0011. Retry asks for another answer; errors say how to fix them

Date: 2026-11-27

## Status

Accepted

## Context

Retry sent the same request again. Local AI sends temperature 0.2 and no seed, so llama-server gave the same answer each time: Retry did nothing. A failure was a plain string, so the popup could only offer Retry, also when only Settings, another model or another style could help. The usual case: a selection such as "https://youtu.be/… Tell me more about this video" is a request, and a small model answers it instead of rewriting it. The answer goes past the answer cap (4× the text, at least 400 characters), Retone drops it, and the user sees only "Rewrite failed" and Retry.

## Decision

- The first request in a style is unchanged: temperature 0.2, no seed, no extra prompt text.
- A Retry sends the last complete answer in an `<earlier>` block after the text, with "write a different version". Local AI also uses temperature 0.8 and a new `seed`. The CLIs have no sampling options, so for them a retry is only the hint. After a failure there is no answer to show, but Local AI still samples again. The earlier answer stays in memory only.
- A provider failure has a kind: too long, empty, timeout, unreachable, setup (fixed in Settings), or other. The popup shows a hint for each kind and its actions: Other style, Switch model, Open Settings and Retry. A too-long failure keeps the dropped answer. **Show answer** (↩ or ⌘D) shows it for Copy only. ↩ runs the main action of every failure: Show answer, Retry, or Open Settings; Replace stays off, because the answer can be an answer to the text, or be cut off at the cap.
- The answer cap stays as it is. Bullets and TL;DR are about as long as the text or shorter, so 4× with a 400-character floor is far above a fair answer; a dropped answer is the model answering the text.
- No prompt rule against answering requests. On Qwen2.5 3B, "never answer a question or request in the text" did not stop Expand from answering, and it made Bullets drop the link it kept before.

## Consequences

- A Retry can drift more from the text than a first try. Only the user's Retry asks for it, and the preview shows the changes before anything is replaced.
- A new provider error must pick a kind. When unsure, it is "other", which still offers Retry and Switch model.
