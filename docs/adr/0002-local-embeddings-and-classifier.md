# ADR 0002 — Local embeddings plus an online classifier

- Status: accepted
- Date: 2026-10-23

## Context

The product claim is that Folderol "learns where your files are and what
category they fall under". That can be built three ways:

1. A local LLM that reads names and contents and answers questions.
2. A pretrained embedding model plus a small classifier trained on user
   behaviour.
3. A model trained from scratch inside the project.

Constraints: CPU-only must be usable, first launch must be useful, the index
must stay small, predictions must be explainable, and nothing may leave the
machine.

## Decision

Two models with different jobs:

- **Embeddings** turn feature text (name, path, type, size/age bucket) into a
  384-dimensional vector. The `Embedder` trait defines the contract. v0 ships a
  deterministic signed-hashing embedder; v1 adds a quantized ONNX
  sentence-transformer (`all-MiniLM-L6-v2`, with `nomic-embed-text-v1.5` as a
  candidate) behind the same trait, downloaded explicitly with a pinned
  checksum.
- **Classification** is one-vs-rest logistic regression trained on this user's
  explicit corrections. A category prior from extension/MIME provides cold
  start; the classifier's confidence blends with it.

Retrieval is hybrid: FTS5 for exact identifiers, embeddings for meaning,
category confidence and recency as light priors.

## Why not an LLM-first design

A 1–3 GB model download, multi-second first token, and a chat surface would
change the product into something slower and less predictable than a file
browser. It also makes every category unexplainable. The embedding + linear
classifier pair runs in milliseconds, fits in a few megabytes, and can show the
exact weights that produced an answer.

An LLM can still be added later as an optional query rewriter or a
natural-language explainer, behind the same `Embedder`/provider boundary. It
must never be required for browsing.

## Why not train from scratch

A from-scratch transformer is a fun project and a bad retrieval model at this
scale. Pretrained sentence embeddings are the correct prior; the personalisation
belongs in the cheap classifier on top, which is where the user's data actually
changes the outcome.

## Consequences

- The hashing baseline is honest but weak semantically. It ships so the pipeline
  is testable offline; the UI copy must not oversell it before v1.
- `semantic_floor` exists because sparse hashed vectors collide. It drops when
  dense embeddings land.
- Vectors are tied to a model. The catalog records `embedder_id` and `dim`;
  changing either triggers a rebuild instead of comparing incompatible spaces.
- Content-based understanding is delayed to M2 and remains opt-in. Metadata-only
  works for a surprising amount of filing, and it avoids copying secrets into
  an index.
- Personalisation needs labels. The correction flow and low-confidence prompts
  are therefore product features, not settings.
