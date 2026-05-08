# Prajna Architecture

This document is the authoritative single-page architecture reference.
It states *what* Prajna is, *why* each layer exists, and *what it does differently*.
Implementation details live in module-level doc comments.

---

## What Prajna Is

A **cognitive database engine**: a storage and retrieval system whose primitive
unit is not a document, row, or vector — but a *memory object* carrying
temporal, semantic, causal, and salience metadata.

Prajna is written in Rust from the block layer up. No existing database engine
is used as a foundation. This is intentional: every storage engine built to date
was designed for a different workload. Prajna's architecture is built around
the workload of autonomous agents.

---

## The Five Things Prajna Does That Nothing Else Does

**1. Embedding-aware page layout.**
The 16 KB page format places vector embeddings in a dedicated, SIMD-aligned
region, separated from record payloads. Vector-only similarity scans read
only this region — 4–8× less I/O than general record scans.

**2. Salience-weighted buffer pool eviction.**
The buffer pool eviction policy scores pages by a combination of recency and
cognitive salience. High-salience pages (identity kernels, frequently reinforced
strategies) outlast low-salience pages (old raw event logs) in RAM regardless
of access order. No existing database has this concept; it does not exist in
LRU or Clock because those algorithms have no concept of data importance.

**3. Cognitive MVCC.**
Version control is a DAG, not a linear chain. Agents fork memory state for
speculative reasoning branches, then merge successful trajectories. This is
closer to Git's object model than PostgreSQL's transaction MVCC. It enables
cognitive experimentation without corrupting the main memory state.

**4. The Unified CogTree Index.**
A single index structure that answers key lookup, vector similarity, temporal
range, and graph traversal queries without joining separate indexes. The inner
node of a CogTree carries: key range, embedding centroid, temporal span, and
adjacency summary. A query with mixed predicates traverses one tree.

**5. Goal-conditioned context compilation.**
The Context Compiler is the query planner of the cognitive layer. Given a goal
and a token budget, it does not retrieve a ranked list. It assembles a
structured `ContextPacket`: working memory, strategic context, identity priors,
open loops, and contradiction alerts — an optimal cognitive state from which the
agent reasons. This is state reconstruction, not retrieval.

---

## Layer Map

```
L11  Context Compiler          goal-conditioned ContextPacket assembly
L10  Cognitive Query Executor  Prajna QL → physical plan
L9   CogTree Index             unified key · vector · temporal · graph index
L8   Catalog & Schema          metadata, agent registry, type definitions
L7   Transaction Manager       cognitive MVCC, fork/merge, snapshot isolation
L6   Lock Manager              per-object write locking within a branch
L5   Record Format             variable-length MemoryObject serialisation
L4   Free Space Management     extent allocation, FSM bitmap pages
L3   Write-Ahead Log           ARIES-style redo/undo, cognitive tx records
L2   Buffer Pool               salience-weighted page cache, dirty tracking
L1   Page Manager              embedding-aware 16 KB slotted page format
L0   Block I/O                 file-backed page read/write, CRC32c checksums
```

Layers L0–L2 are implemented in `crates/prajna-storage`.
Cognitive types (MemoryObject, salience) are in `crates/prajna-core`.
L3 and above are planned.

---

## Memory Object

The fundamental storage unit. Every fact, episode, goal, strategy, identity
model, and world belief is a `MemoryObject`.

Core fields:
- `kind` — semantic type (Episode, Fact, Goal, Pattern, Strategy, …)
- `content` — serialised payload
- `embedding` — semantic vector (model-dependent dimensionality)
- `compression_tier` — position in the 7-level memory hierarchy (0–6)
- `created_at`, `last_accessed_at`, `last_reinforced_at` — temporal provenance
- `base_salience`, `access_count`, `reinforcement_count` — salience signals
- `confidence` — epistemic weight
- `causal_parent_ids`, `compressed_from_ids`, `contradicts_ids` — graph edges

---

## Salience Model

```
salience(m, t) = base_salience(m)
               × decay_rate(tier(m)) ^ age_days(m, t)
               × (1 + ln(reinforcement_count(m) + 1))
```

Decay rates by tier: `[0.95, 0.80, 0.60, 0.40, 0.20, 0.05, 0.01]`

Raw events (tier 0) half-life: ~13 days.
World models (tier 6) half-life: ~2 years.

---

## Context Compilation (v1 — greedy MMR)

Input: `goal_embedding`, `agent_id`, `token_budget`

1. Score all candidate memories: `λ × salience + (1-λ) × cosine(m, goal)`
2. Select iteratively using Maximal Marginal Relevance (balances relevance
   against redundancy — avoids selecting 20 near-identical episodes)
3. Assemble into a structured `ContextPacket` with ordered sections:
   identity preamble → open loops → strategic context → working memory → facts

Output is not a list of strings. It is a typed packet with token budget
accounting and a compilation trace for debugging.

---

## Memory Hierarchy

```
Tier 0  Raw events       — individual tool calls, observations, actions
Tier 1  Sessions         — bounded interaction periods
Tier 2  Episodes         — goal-bounded experience clusters
Tier 3  Patterns         — recurring behavioural regularities
Tier 4  Strategies       — generalised heuristics
Tier 5  Identity models  — persistent self-representation
Tier 6  World models     — long-horizon environmental beliefs
```

Compression moves memories up the hierarchy. Raw events are summarised into
sessions, sessions into episodes, episodes into patterns and strategies.
Each compression step must preserve: causal structure, failures as explicitly
as successes, confidence and uncertainty, decision utility.

---

## Page Format

```
 offset 0      PageHeader (64 bytes)
 offset 64     Embedding Region (32-byte aligned, SIMD-scannable)
               vec[0]: f32 × dim
               vec[1]: f32 × dim
               …
 offset emb_end  Record Region (grows forward →)
               record[0], record[1], …
               ← free space →
               … slot[1], slot[0]  (grows backward ←)
 offset PAGE_SIZE  Slot Directory end
```

Page size: 16 384 bytes (16 KB).
Header: 64 bytes (CRC32c checksum, LSN, type, tier, slot count, free pointers).
Slot entry: 12 bytes (record offset, length, embedding index).

---

## Storage Tiers

```
Tier-0 data (hot)    NVMe / RAM-backed
Tier-1/2 data        SSD
Tier-3/4 data (warm) HDD or object store
Tier-5/6 (frozen)    Cold archival
```

Page IDs encode tier in the high bits. Migration between tiers happens
transparently through the block manager as salience decays.

---

## Cognitive MVCC

Each agent maintains a HEAD version pointer. Operations that mutate memory
create a new version node in the DAG. A fork creates a new branch from HEAD.
A merge creates a new version with two parent pointers (like a Git merge commit).

WAL records include a `CognitiveTransaction` variant that captures semantic
operations: episode formation, reinforcement, contradiction, identity update.
This makes the WAL a complete audit log of an agent's cognitive history,
replayable for debugging or forking at any historical point.

---

## What Prajna Is Not

- Not a vector database (Pinecone, Weaviate, Chroma, Qdrant)
- Not a graph database (Neo4j, TigerGraph)
- Not a time-series database
- Not a document database
- Not a memory middleware or RAG framework
- Not a wrapper around any existing storage engine

Prajna is a new category: **Cognitive Database**.

The closest analogies: PostgreSQL for relational data, SQLite for embedded
relational, RocksDB for key-value. Prajna aims to be the equivalent for
cognitive agent memory — the foundational substrate, not the application layer.

---

*Last updated: 2026-05. Architecture version: 0.1.*
