# Prajna — Build Roadmap

This is a multi-year engineering roadmap. Each phase has a concrete milestone
that proves correctness before the next phase begins.

---

## Phase 1 — Storage Foundations (Years 1–2)

Build the lowest layers of the storage engine correctly.
No cognitive features until the foundation is crash-safe.

### Q1–Q2: Block I/O, Page Format, Buffer Pool
- [x] Block I/O with CRC32c checksums
- [x] Embedding-aware slotted page format (16 KB)
- [x] Salience-weighted buffer pool (skeleton)
- [x] MemoryObject type system
- [x] Salience model + MMR ranking

**Milestone:** write 1M random pages, crash the process at a random point,
restart, read all pages back with checksums valid. Zero data loss.

### Q3–Q4: WAL and Crash Recovery
- [ ] ARIES-style write-ahead log with LSN
- [ ] Physical redo logging
- [ ] Crash recovery: analysis → redo → undo passes
- [ ] Group commit and fsync batching
- [ ] Cognitive transaction record types in WAL

**Milestone:** kill -9 under random write load, restart, verify all committed
operations present and all uncommitted ones rolled back. Run 24-hour chaos test.

### Q1–Q2 (Year 2): Records and B-tree
- [ ] Variable-length record serialisation
- [ ] Overflow pages for large content blobs
- [ ] Free space management (FSM bitmap pages)
- [ ] Concurrent B-tree (lock-coupling)
- [ ] Range scans with snapshot consistency

**Milestone:** ingest 100M records, point lookup P99 < 1ms, 1000-record range
scan P99 < 10ms.

### Q3–Q4 (Year 2): Cognitive Page Layout Specialisation
- [ ] Tiered storage routing by compression tier
- [ ] Salience-weighted buffer pool — full implementation (max-heap eviction)
- [ ] Salience score refresh as a background job
- [ ] Embedding region prefetching for batch SIMD scan

**Milestone:** vector-only scan throughput 4× higher than full-record scan.
Salience-weighted eviction outperforms LRU on agent workload benchmark.

---

## Phase 2 — Transactions and Indexes (Year 3)

### Cognitive MVCC
- [ ] Version chains as DAG
- [ ] Fork (create a reasoning branch from current HEAD)
- [ ] Merge (combine two branches, resolve conflicts)
- [ ] Diff (compare two memory states)
- [ ] Snapshot reads consistent at any past version
- [ ] Lock manager for within-branch write conflicts

**Milestone:** 1000-branch stress test with concurrent merges, no corruption.

### The Unified CogTree Index
- [ ] CogTree node format (key · embedding centroid · temporal span · adjacency)
- [ ] Mixed-predicate query traversal
- [ ] Concurrent inserts and deletes
- [ ] Periodic salience score refresh within the index

**Milestone:** mixed-predicate queries (vector + temporal + graph) 2× faster
than separate-index baseline.

---

## Phase 3 — Cognitive Engine (Year 4–5)

### Context Compiler
- [ ] Greedy MMR-based context compilation (v1)
- [ ] Structured ContextPacket output
- [ ] Coverage-based dimension analysis (v2)
- [ ] Token budget accounting

### Identity Kernel
- [ ] Persistent per-agent self-representation
- [ ] Identity-conditioned retrieval (context biased by identity)
- [ ] Identity update pipeline

### Memory Compression Pipeline
- [ ] Tier 0→1: session summarisation
- [ ] Tier 1→2: episode clustering
- [ ] Tier 2→3: pattern extraction
- [ ] Compression quality benchmark (< 10% task degradation at > 50% token reduction)

### Contradiction Engine
- [ ] Pairwise semantic conflict detection
- [ ] Conflict classifier (semantic negation)
- [ ] Contradiction surfacing in ContextPacket

**Milestone (Phase 3 end):** Prajna v0.1 public release.
PCB benchmark results published.

---

## Phase 4 — SDKs and Multi-Agent (Year 6)

- [ ] Python SDK (`pip install prajna`)
- [ ] TypeScript SDK (`npm install @prajna/client`)
- [ ] Go SDK
- [ ] Shared memory pools (multi-agent)
- [ ] Scoped cognitive access control
- [ ] Cognitive consensus for conflicting agent beliefs

---

## Phase 5 — Distributed Cognition (Years 7–10)

- [ ] Replication (Raft or Paxos)
- [ ] Sharding by agent ID
- [ ] Edge-deployable embedded mode (like SQLite)
- [ ] Cognitive observability (query traces, salience dashboards)
- [ ] Enterprise governance layer

---

## Benchmarks and Publications

| Target | Timeline |
|--------|----------|
| PCB benchmark suite (standalone, runs on any system) | Phase 1 Q4 |
| Blog: "Why Vector DBs Fail for Agents — Measured" | Phase 1 Q4 |
| Paper: "Goal-Conditioned Context Compilation" | Phase 2 Q2 |
| Paper: "PCB: A Benchmark Suite for Agent Memory" | Phase 2 Q2 |
| Paper: "Salience-Weighted Buffer Pool Management" | Phase 2 Q4 |
| Paper: "Hierarchical Compression for Agent Memory" | Phase 3 Q2 |
| Paper: "Prajna: A Cognitive Database Engine" (full system) | Phase 3 end |

---

*"The first implementation teaches you the problem. The second one ships."*
