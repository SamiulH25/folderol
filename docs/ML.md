# Learning and retrieval

**Status:** the hashing embedder, hybrid ranking and logistic classifier are
shipped (M0). The ONNX embedder, calibration and event-based learning are
planned (M2–M4). This document separates what exists from what is contracted.

## 1. What “learns where your files are” means

Folderol does not learn a map of the filesystem. It learns three things:

1. **Representation** — what a file *is*, from its name, path, type and shape.
2. **Categories** — which class this person files similar things under.
3. **Association** — which files are used together, and where they tend to live.

The first is a pretrained embedding model (or a deterministic baseline). The
second is a tiny classifier trained on corrections. The third is an event model
that arrives with M4. None of them move a file.

## 2. Pipeline

```text
FileRecord
   │
   ├─ feature_text() ──▶ tokenizer ──▶ Embedder ──▶ vector (384 × f32)
   │                                                   │
   ├─ extension/MIME ──▶ category prior (p₀, c₀)       │
   │                                                   ▼
   └─ human labels ──▶ Classifier ──▶ p̂ ──▶ blend ──▶ stored category
                                                         │
query ──▶ tokenizer ──▶ Embedder ──▶ cosine ──┐         │
                                              ├─▶ hybrid score ──▶ results
query ──▶ FTS5 tokens ──▶ bm25 rank ──────────┘
```

## 3. Features

### 3.1 Feature text

The string handed to the embedder is assembled in `folderol-core`:

```text
<all path components, tokenized>
<basename, tokenized again>
ext:<extension>
mime:<type/subtype>
kind:<file|directory|symlink|other>
size:<bucket>
age:<today|week|month|year|old>
```

Why each part:

| Feature | Why it is there | Noise risk |
| --- | --- | --- |
| Basename, twice | The strongest human signal. “Invoice_Acme” says more than its folder. | Low |
| Path components | Filing habits: `Finance/2024`, `Projects/Client`, `Pictures/2024`. | Medium: a deep path adds many generic words |
| Extension | Cheap, reliable type signal and a fallback for odd names. | Low |
| MIME | Catches extensionless files and ambiguous types. | Low |
| Kind | Distinguishes directories and links when the catalog grows to hold them. | None |
| Size bucket | Invoices are small PDFs; video is large. Weak but free. | Low |
| Age bucket | Recency correlates with working set and with category. | Medium: ages drift, so vectors go stale |

Deliberate omissions until M2:

- File contents. Reading every file makes a scan unbounded and copies secrets
  into the index.
- Thumbnails or image embeddings. Expensive and duplicative of the category
  prior.
- Owner, group, permissions. Rarely semantic; potentially sensitive.

### 3.2 Tokenization

- Unicode-aware alphanumeric runs.
- Split on every non-alphanumeric character.
- Split `camelCase`, `PascalCase`, and letter/digit boundaries only where a
  case change occurs (`TaxReturn2024` → `tax`, `return`, `2024`).
- Lowercase with Unicode case folding.
- Drop tokens shorter than 2 characters and longer than 64.
- Keep numeric tokens: years, versions, invoice numbers and dates matter.
- No stemming and no stop-word list in v0. The embedding model handles
  morphology; a hand-written stop list would remove “the” from “The Who” and
  damage exact matches.

Examples:

```text
2024/TaxReturn-final_v2.pdf → 2024 tax return final v2 pdf
Rechnung_Übersicht.md       → rechnung übersicht md
Screenshot 2026-10-23.png   → screenshot 2026 10 23 png
```

### 3.3 Query text

Natural-language queries are tokenized and embedded with the same embedder.
No query expansion, spell correction or synonym table in v0. Those are
retrieval features, not embedding features, and belong behind an evaluation
gate.

## 4. Embedding tier

### 4.1 The contract

```rust
pub trait Embedder: Send + Sync {
    fn dim(&self) -> usize;
    fn embed(&self, text: &str) -> Vec<f32>;
    fn embed_record(&self, record: &FileRecord) -> Vec<f32>;
    fn embed_query(&self, query: &str) -> Vec<f32>;
    fn model_id(&self) -> &str;
    fn semantic_floor(&self) -> f32;
}
```

Rules:

- Deterministic for a given `(model_id, text)`.
- Output is L2-normalised; cosine is a dot product.
- `model_id` is written to `meta.embedder_id`. A mismatch invalidates the
  catalog’s vectors and forces a rescan.
- Embedders never perform I/O and never block on a network.
- The query and document paths may diverge later (asymmetric models); the trait
  keeps them separate for that reason.

### 4.2 Shipped baseline: signed hashing

`HashingEmbedder`, dimension 384, `model_id = "hashing-v1"`.

For every token:

```text
i = fnv1a(token) mod 384
s = sign(fnv1a(token, seed))
count[i] += s
```

Then:

```text
v[i] = sign(count[i]) · ln(1 + |count[i]|)
v     = v / ||v||₂
```

Properties:

- Zero dependencies, zero download, deterministic across machines.
- Exact token overlap produces high similarity.
- Collisions produce false positives, especially for a one-token query.
- It has no notion of synonymy. “Lease” does not match “Mietvertrag”.

That is why it is a baseline and why `semantic_floor()` is 0.5: without a high
bar, an unknown single token can collide with an unrelated document and return
nonsense. The floor exists to make the baseline honest, not to make it good.

### 4.3 Target: pretrained sentence embeddings

Selection is evaluation-driven. Candidates as of 2026:

| Model | Dim | Params | Context | License | Notes |
| --- | --- | --- | --- | --- | --- |
| `sentence-transformers/all-MiniLM-L6-v2` | 384 | 22.7M | 256 | Apache-2.0 | Small, fast, well-supported ONNX; strong default |
| `BAAI/bge-small-en-v1.5` | 384 | 33M | 512 | MIT | Often better retrieval; needs a query prefix |
| `intfloat/e5-small-v2` | 384 | 33M | 512 | MIT | Requires `query:` / `passage:` prefixes |
| `nomic-ai/nomic-embed-text-v1.5` | 768 | 137M | 8192 | Apache-2.0 | Matryoshka; larger, better for long content; overkill for names |

Minimum requirements:

- Permissive license compatible with GPL-3.0-or-later distribution.
- ONNX export available and reproducible.
- ≤ 120 MB at the chosen precision; ≤ 40 MB preferred.
- ≤ 5 ms per short text on 4 CPU threads, batch 16.
- 384 dimensions preferred: the index stays small and brute-force search stays
  within budget.
- Handles Unicode and path-like strings without pathological tokenization.
- Multilingual only if it does not cost more than 2× latency; otherwise a
  separate model is a later option.

The chosen model and its checksum live in a manifest:

```json
{
  "id": "minilm-l6-v2-int8",
  "dim": 384,
  "file": "model.onnx",
  "sha256": "…",
  "license": "Apache-2.0",
  "source": "https://…",
  "tokenizer": "tokenizer.json",
  "max_tokens": 256,
  "size_bytes": 0
}
```

### 4.4 Runtime

- `ort` (ONNX Runtime) with the CPU execution provider.
- One session per process, created lazily on first semantic query.
- Intra-op threads: `min(4, available_parallelism - 1)`, floor 1.
- Batch size 16 for scanning; single text for queries.
- Tokenizer: Hugging Face `tokenizers`, loaded from the manifest.
- Warm-up: one empty inference after load; the UI shows “warming up the local
  model” once, under one second.
- Arena memory is released when the window closes; the CLI exits.

### 4.5 Download and offline behaviour

- **Never automatic.** Settings shows model name, size, license and source, and
  requires a click.
- Download to a temp file, verify SHA-256, then atomically rename.
- Store under `$XDG_DATA_HOME/folderol/models/<id>/`.
- If the model is missing, unreadable or the checksum fails: fall back to
  `hashing-v1`, show a banner in search once, and keep working.
- Air-gapped users can place a verified model directory manually; the manifest
  is checked the same way.
- No model, query or vector is ever uploaded.

## 5. Category prior

The taxonomy is seeded, not learned from scratch:

```text
documents images audio video code archives data config fonts 3d executables other
```

`Category::guess()` maps extension first (confidence 0.92), then MIME prefix
(0.70), then `other` (0.25). Directories are never confidently classified.

The prior exists so the first launch is useful. It is intentionally simple,
testable and explainable. A `.pdf` is a document until this user says otherwise.

## 6. Classifier

### 6.1 Model

One-vs-rest logistic regression over the embedding:

```text
z_c = w_c · x + b_c
p_c = σ(z_c)
q_c = p_c / Σ_k p_k
```

- One weight vector per category, one bias.
- 12 categories × 384 floats = 18 KB at f32; JSON is larger but still trivial.
- Training is full-batch restarts from the `labels` table, not incremental
  drift. Removing a label therefore removes its influence.

### 6.2 Training

```text
for epoch in 1..=80:
  shuffle samples deterministically
  for (x, y) in samples:
    p = forward(x)
    for c in classes:
      target = 1 if c == y else 0
      g = (p_c - target) · η
      w_c -= g · x
      b_c -= g
```

Current hyperparameters: 80 epochs, η = 0.35, BCE loss for reporting. They are
deliberately boring; the dataset is tiny and the model is convex.

Guards:

- Refuse to train with zero samples.
- Refuse dimension mismatch rather than silently scoring zero.
- Do not reclassify the catalog with fewer than 3 labels. With one or two
  samples the model memorises and relabels everything; the correction is still
  stored and the model is saved.
- Every prediction can be explained by the weights that fired.

### 6.3 Blending with the prior

Shipped behaviour: once there are ≥3 labels, the classifier’s normalised
probability is stored as the category and confidence; the label row itself is
confidence 1.0.

Target behaviour (M2):

```text
α = clamp((labels_for_class - 2) / 20, 0, 1)
p_final = α · q_classifier + (1 - α) · p_prior
```

- With no personal evidence, the prior wins.
- With 20+ examples of a class, the classifier wins.
- The blend is per class, so a well-labelled “documents” does not destabilise a
  barely-labelled “3d”.

### 6.4 Calibration and display

- A probability is not a promise. Calibrate with temperature scaling on the
  user’s own labels once there are ≥50.
- Show a category chip only at ≥70% final confidence.
- Show confidence as a number in the inspector, never colour alone.
- A low-confidence prediction is not an error; it is an absence of a claim.
- Calibration error (ECE) is measured in the evaluation harness, not assumed.

### 6.5 Explanations

For a category `c`, the inspector can show:

- the top positive-weighted features that overlap the file’s vector,
- the prior and its confidence,
- human labels for this file and for neighbours,
- the number of training samples behind the class.

With hashing, feature names are not recoverable from dimensions, so the shipped
explanation is a summary (“name, folder and extension, 92% confidence”). With a
tokenizer-backed ONNX model, attribution can map dimensions back to tokens
through occlusion or a linear surrogate. Until then, do not fake token-level
attribution.

## 7. Learning signals

| Signal | Strength | Source | Status |
| --- | --- | --- | --- |
| Explicit category correction | strong (+1.0) | chip menu, drag to category | shipped |
| Label removal | strong (retrain without it) | settings / chip | stored, UI planned |
| Open | weak (+0.1) | event table | M4 |
| Reveal in folder | weak (+0.05) | event table | M4 |
| Move after open | medium | event table + operation journal | M4 |
| Co-access within 10 min | association (not a label) | event pairs | M4 |
| Dwell time > 30 s | weak | event table | M4 |
| Query followed by open | medium for ranking, not category | search events | M4 |

Rules:

- Only explicit labels train the classifier. Weak signals adjust ranking and
  associations; they never silently become category truth.
- Events are local, capped (90 days), and deletable from Settings.
- No cross-user federated learning. There is no upload path.

## 8. Association and smart folders

M4. The design constraints are fixed now:

- Build a co-access graph from opens and moves with a 10-minute window.
- Weight an edge by PMI rather than raw co-occurrence, so popular folders do not
  dominate.
- Propose clusters as smart folders with a generated name from the top
  distinctive tokens, never “Cluster 7”.
- A proposal is a saved query. It stores no files and can be deleted without
  touching them.
- Do not propose a folder the user already has as a real directory with the same
  contents; that is noise.

## 9. Evaluation

### 9.1 Corpus

- 2,000 synthetic files with realistic names and paths, plus a small set of
  real, permissively licensed documents.
- No personal data in the repository.
- Categories and query judgments are versioned alongside the corpus.
- A held-out set is never used for tuning weights.

### 9.2 Queries

80 queries across:

- exact filename fragments (`acme invoice`),
- path-aware (`finance 2024`),
- natural language (`the lease for the flat`),
- fuzzy memory (`that paper about sleep and memory`),
- scopes (`type:pdf before:2024`),
- negatives (nonsense, absent files).

Each query has 0–3 acceptable results. Judgments are stored as paths, not ranks,
so the metric is stable when the corpus moves.

### 9.3 Metrics

| Metric | Meaning | Gate |
| --- | --- | --- |
| Precision@3 | acceptable results in the top 3 | ≥ 0.80 |
| MRR@10 | rank of the first acceptable result | ≥ 0.65 |
| Recall@20 | acceptable results found in top 20 | ≥ 0.90 |
| Category accuracy | correct category after N labels | ≥ prior + 15 pts at N=50 |
| ECE | calibration error of confidence | ≤ 0.10 |
| p95 latency | query time at 100k | ≤ 30 ms |

Baselines: FTS-only, hashing hybrid, ONNX hybrid. A model change ships only if
it beats the current baseline on the corpus and does not regress any gate by
more than 3 points.

### 9.4 User evaluation

Twelve participants, scripted tasks, no moderator hints:

1. Find a document from a half-remembered description.
2. Find a file seen yesterday.
3. File 20 receipts with minimal corrections.
4. Recover a file from trash.
5. Rename a batch of photos.

Measure task success, time, corrections and trust rating. Qualitative notes are
as important as the numbers.

## 10. Privacy and security

- No network code in the engine; the optional model download is a separate,
  explicit action.
- The index contains full paths and is `0600`. It is not encrypted at rest;
  Folderol relies on full-disk encryption and documents that. App-level
  encryption is deferred (key management is a product of its own).
- Content extraction is off by default and stores extracted text separately
  with a retention policy.
- Events are local, capped and deletable.
- Model files are verified by SHA-256; a corrupt or substituted model is
  rejected.
- Filenames are data, never instructions. Folderol does not execute them and
  does not feed them to an instruction-following model in v1.

## 11. Failure modes

| Failure | Symptom | Response |
| --- | --- | --- |
| Cold start | Everything is “Other” | Prior still useful; corrections cheap; never show a chip below 70% |
| Sparse hashing collision | Nonsense query returns a file | `semantic_floor`; lexical support required for weak matches |
| Label noise | One wrong correction skews a class | Retrain from labels; show label counts; allow removal |
| Distribution shift | A new job creates a new folder pattern | Corrections update the model; smart folders propose, not impose |
| Model missing/corrupt | Semantic results absent | Fall back to lexical + prior with a visible banner |
| Embedder changed | Vectors incomparable | `meta.embedder_id` mismatch forces rebuild; never mix |
| Dense model slow on old CPU | Search stutters | Thread cap, int8, batch, degrade to lexical while warming |
| Unicode edge cases | Garbled tokens | Tokenizer tests with CJK, emoji, combining marks, RTL |
| Adversarial filenames | Weird categories | No execution, no network, confidence visible |
| Huge directory | Scan memory spike | Streaming walker, 512-row batches, bounded channel |

## 12. Versioning

- `embedder_id` identifies the vector space. Change it whenever the model,
  dimension, normalisation or feature template changes.
- `model.json` carries its own `dim` and `classes`; loading a mismatched file is
  an error, not a silent reset.
- Labels are the durable truth. Models are disposable.
- A future fine-tune gets a new id and an A/B evaluation before becoming the
  default.

## 13. Open questions

1. **Fine-tuning vs prompting.** Is a small contrastive fine-tune on file names
   worth the maintenance, or is a general model plus a linear head enough?
2. **Content embeddings.** If content is indexed, do we embed chunks and pool,
   or keep metadata similarity separate and combine at ranking time?
3. **Multilingual.** A separate model per locale, or one multilingual model with
   a latency cost for everyone?
4. **LLM query rewriting.** A local 1–3B model could rewrite “the lease for the
   flat” into better search terms. It must be optional, cited and never the
   only path to a result.
5. **Per-folder models.** A user’s `Photos` and `Code` may deserve different
   notions of similarity. Is a single global space enough?
6. **Calibration data.** Temperature scaling needs labels; how many before it
   is better than raw probabilities?
