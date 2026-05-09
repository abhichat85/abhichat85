<div align="center">

# Prajna

**A cognitive database engine, built from scratch, for AI agents.**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](./LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-dea584.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-pre--alpha-red.svg)]()
[![Phase](https://img.shields.io/badge/phase-1%20%E2%80%94%20storage%20foundations-orange.svg)]()

</div>

---

> Prajna is a multi-year project to build a database engine purpose-built for
> autonomous AI agents — written in Rust from the block layer up, with no
> existing storage engine as a foundation.
>
> The repository is private during the foundational phases and will open to
> the public when the storage engine is correct, the cognitive layer ships,
> and a reproducible benchmark suite proves it works.

---

## What This Is

Prajna is **not** a vector database.
It is **not** a wrapper around an existing storage engine.
It is **not** a RAG framework or memory middleware.

It is a **native cognitive storage engine** — designed from the page layer
up — for the workloads that AI agents actually have: temporal memory,
goal-conditioned retrieval, identity continuity, and long-horizon reasoning
across time.

The argument for building from scratch is simple: every existing storage
engine was designed for a different workload. PostgreSQL was built for
transactional consistency over tabular data. RocksDB was built for Facebook's
key-value patterns. Vector DBs are bolted-on similarity search on top of those
substrates. None of them have a storage layer that knows what cognition is.

A storage engine designed natively for cognitive workloads will outperform
retrofitted ones at the workloads that matter for agents — not because the
higher layers are smarter, but because the bytes on disk are organised
differently.

---

## The Problem

Vector databases answer: *"What text is most similar to this text?"*

That is not the question agents need answered.

Agents need: *"Given what I am trying to accomplish right now, what is the
optimal cognitive state to reason from?"*

These are fundamentally different problems. **Retrieval is not cognition.**
Prajna is built to solve the second problem from the storage primitives up —
page layout, buffer pool, indexes, query planner, and cognitive compiler — all
designed natively for cognitive workloads.

---

## Five Things Prajna Does That Nothing Else Does

### 1. Embedding-Aware Page Layout
Every 16 KB page separates embedding vectors into a dedicated, 32-byte-aligned
region. A SIMD similarity scan reads only this region — 4–8× less I/O than
reading full records.

### 2. Salience-Weighted Buffer Pool
Instead of LRU, the buffer pool evicts based on cognitive salience. Identity
kernels and strategic memories stay in RAM. Old raw event logs fall cold first.
LRU and Clock have no concept of data importance; this one does.

### 3. Cognitive MVCC *(planned, Phase 2)*
Agents fork their memory state for speculative reasoning, then merge
trajectories back. The version model is a DAG, closer to Git's object model
than PostgreSQL's transaction MVCC.

### 4. The Unified CogTree Index *(planned, Phase 2)*
One index structure supporting key lookup, vector similarity, temporal range,
and graph traversal in a single traversal. No join between indexes. No
cross-index coordination at query time.

### 5. Goal-Conditioned Context Compilation *(planned, Phase 3)*
The Context Compiler assembles a structured cognitive state packet given a
goal and a token budget. Not a list of similar chunks — a curated working
memory, strategic context, open loops, identity priors, and contradiction
alerts. **State reconstruction, not retrieval.**

---

## Architecture

```
Applications / Agent Runtimes
           │
           ▼
  ┌─────────────────────────┐
  │   Python / TS / Go SDK   │
  └───────────┬─────────────┘
              │
  ┌───────────▼─────────────────────────────┐
  │   Context Compiler  (Prajna QL)          │  goal-conditioned state reconstruction
  ├─────────────────────────────────────────┤
  │   Salience Engine                        │  dynamic cognitive prioritisation
  ├─────────────────────────────────────────┤
  │   Cognitive Query Executor               │
  ├─────────────────────────────────────────┤
  │   CogTree  (Unified Index)               │  key · vector · temporal · graph
  ├─────────────────────────────────────────┤
  │   Transaction Manager  (Cognitive MVCC)  │  branching · merging · versioning
  ├─────────────────────────────────────────┤
  │   Write-Ahead Log                        │  durability · crash recovery
  ├─────────────────────────────────────────┤
  │   Buffer Pool                            │  salience-weighted page cache      ◄─ L2  ✅
  ├─────────────────────────────────────────┤
  │   Page Manager                           │  embedding-aware 16 KB pages       ◄─ L1  ✅
  ├─────────────────────────────────────────┤
  │   Block I/O                              │  tiered file I/O · CRC32c checks   ◄─ L0  ✅
  └─────────────────────────────────────────┘
              │
  ┌───────────▼─────────────────────────────┐
  │   Storage  (NVMe → SSD → HDD → cold)    │
  └─────────────────────────────────────────┘
```

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the full design specification.

---

## Current Status

| Phase | Layer | Status |
|-------|-------|--------|
| 1 | **Block I/O (L0)** — file-backed pages, CRC32c checksums | ✅ done |
| 1 | **Embedding-aware page format (L1)** — 16 KB slotted, SIMD-aligned | ✅ done |
| 1 | **Salience-weighted buffer pool (L2)** — full skeleton | ✅ done |
| 1 | **MemoryObject type system** — 9 kinds, 7-tier hierarchy | ✅ done |
| 1 | **Salience model** — formal decay + reinforcement function | ✅ done |
| 1 | **MMR ranking** — diverse goal-conditioned selection | ✅ done |
| 1 | Write-ahead log (L3) | next |
| 1 | Free space management (L4) | planned |
| 2 | B-tree index (L5) | planned |
| 2 | Unified CogTree index | planned |
| 2 | Cognitive MVCC | planned |
| 3 | Context Compiler | planned |
| 3 | Identity Kernel | planned |
| 3 | Memory compression pipeline | planned |
| 4 | Python / TypeScript SDKs | planned |
| 4 | Multi-agent shared memory | planned |

**30 tests, all passing.** See [docs/ROADMAP.md](./docs/ROADMAP.md) for the
full multi-year build plan with milestones.

---

## Quick Start

Requires Rust 1.75 or later.

```bash
git clone <this-repo>
cd prajna
cargo build
cargo test    # 30 tests across 2 crates
```

---

## Repository Layout

```
prajna/
├── crates/
│   ├── prajna-storage/    # L0–L2: block I/O, page format, buffer pool
│   │   └── src/
│   │       ├── block/     # File-backed page I/O with checksums
│   │       ├── page/      # Embedding-aware slotted pages
│   │       ├── buffer/    # Salience-weighted buffer pool
│   │       └── error.rs
│   └── prajna-core/       # Cognitive types
│       └── src/
│           └── memory/    # MemoryObject + salience model
├── docs/
│   ├── PHILOSOPHY.md      # Theoretical foundations & Vedic mapping
│   └── ROADMAP.md         # Multi-year build plan
└── bench/
    └── pcb/               # Prajna Cognitive Benchmark suite spec
```

---

## Why Open Source (Eventually)

Foundational infrastructure wins through ecosystems. PostgreSQL, SQLite,
Linux, Redis — all are open source, and that openness is inseparable from
their dominance.

This repository is private during Phase 1 and Phase 2 to allow rapid
iteration without premature commitment to APIs. It will open to the public
when:

1. The storage engine is crash-safe and correct (end of Phase 1).
2. The first cognitive layer (Context Compiler v1) ships (Phase 3).
3. The Prajna Cognitive Benchmark (PCB) demonstrates measurable improvement
   over existing systems (Letta, Mem0, Zep, plain RAG).

When that happens, the project will be released under Apache 2.0 with the
full intent of becoming the foundational cognitive substrate for the
autonomous agent era.

---

## Design Documents

- [ARCHITECTURE.md](./ARCHITECTURE.md) — full architectural specification
- [docs/ROADMAP.md](./docs/ROADMAP.md) — multi-year build plan
- [docs/PHILOSOPHY.md](./docs/PHILOSOPHY.md) — theoretical foundations
- [bench/pcb/README.md](./bench/pcb/README.md) — cognitive benchmark suite

---

## License

Apache 2.0. See [LICENSE](./LICENSE).

---

<div align="center">

*"Retrieval is not cognition. Prajna reconstructs cognitive state."*

</div>
