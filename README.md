## ⚡ What I believe

AI agents will replace everyone in the knowledge economy. Not a prediction — already happening. Most people are still sleeping on it.

I build the infrastructure for the companies making that transition real.

---

## ⭐ Highlights

<table>
<tr>
<td align="center" width="50%">
<strong>2026 — Flagship OSS</strong>
<br><br>
<a href="https://github.com/abhichat85/agent-stream">
<img src="https://opengraph.githubassets.com/1/abhichat85/agent-stream" alt="agent-stream" width="320">
</a>
<br>
<a href="https://github.com/abhichat85/agent-stream"><strong>agent-stream</strong></a>
<br>
<sub>The SSE event protocol for AI agents. Extracted from 36 tools, thousands of production runs.</sub>
<br><br>
<sub><code>pip install agent-event-stream</code> &nbsp;·&nbsp; <code>npm install @agent-stream/react</code></sub>
</td>
<td align="center" width="50%">
<strong>2026 — New</strong>
<br><br>
<a href="https://github.com/abhichat85/ant">
<img src="https://opengraph.githubassets.com/1/abhichat85/ant" alt="ANT" width="320">
</a>
<br>
<a href="https://github.com/abhichat85/ant"><strong>ANT — Agent-Native Terminal</strong></a>
<br>
<sub>Like Warp, redesigned from first principles for AI agents. Rust. Approval-gated. Never runs arbitrary shell commands.</sub>
</td>
</tr>
</table>

| | |
|:--|:--|
| [Why SSE for AI agents keeps breaking at 2am — DEV.to](https://dev.to/abhishek_chatterjee_33b9d/why-sse-for-ai-agents-keeps-breaking-at-2am-55ie) | The four bugs every team hits. The protocol that fixes them. |
| [I've burnt 43 Million tokens in Claude Code — Medium](https://medium.com/@abhishekchatterjee) | What building at this scale actually teaches you. |
| [The Anatomy of an Agent Harness — Medium](https://medium.com/@abhishekchatterjee) | "On the engineering that lives between a rented model and a user who trusts it." |

---

## 🏗️ Einstein Labs — Operating Stack for AI-Native Companies

Five products. One thesis: AI-native companies need infrastructure built for them — not San Francisco pricing and assumptions.

<table>
<tr>
<td align="center" width="33%">
<a href="https://www.praxiomai.xyz">
<img src="https://opengraph.githubassets.com/1/abhichat85/praxiom-workspace-r2c" alt="Praxiom AI" width="240">
</a>
<br>
<a href="https://www.praxiomai.xyz"><strong>Praxiom AI</strong></a>
<br>
<sub><strong>Autonomous AI product manager.</strong> 36 agent tools shipped. Multi-agent harness, overnight research cycles, quality-scored PRDs. Integrates with Linear, GitHub, Jira.</sub>
</td>
<td align="center" width="33%">
<a href="https://github.com/abhichat85/polaris">
<img src="https://opengraph.githubassets.com/1/abhichat85/polaris" alt="Polaris" width="240">
</a>
<br>
<a href="https://github.com/abhichat85/polaris"><strong>Polaris</strong></a>
<br>
<sub><strong>Spec-driven AI coding agent.</strong> Real agent in E2B sandbox. Sees its own runtime errors. Raw Anthropic SDK — no abstraction wrappers. Built for India + SEA.</sub>
</td>
<td align="center" width="33%">
<a href="https://github.com/abhichat85/einsteins-pulse">
<img src="https://opengraph.githubassets.com/1/abhichat85/einsteins-pulse" alt="Pulse AI" width="240">
</a>
<br>
<a href="https://github.com/abhichat85/einsteins-pulse"><strong>Pulse AI</strong></a>
<br>
<sub><strong>Product intelligence engine.</strong> LLM-powered behavioral analytics. The layer between your event stream and a decision a PM can act on.</sub>
</td>
</tr>
<tr>
<td align="center" width="33%">
<strong>Prism</strong>
<br><br>
<sub><strong>Autonomous content intelligence.</strong> Multi-tenant AI-native content OS. Generates at scale, maintains brand voice across channels. Runs independently.</sub>
</td>
<td align="center" width="33%">
<strong>Pylon</strong>
<br><br>
<sub><strong>Billing infrastructure for AI companies.</strong> Per-token, per-agent-run, per-insight pricing. Not Stripe bolted on top — built for the AI billing model from scratch.</sub>
</td>
<td align="center" width="33%">
</td>
</tr>
</table>

---

## 🔧 Open Source

Tools built from production, not from ideas.

<table>
<tr>
<td align="center" valign="top" width="50%">
<a href="https://github.com/abhichat85/agent-stream">
<img src="https://opengraph.githubassets.com/1/abhichat85/agent-stream" alt="agent-stream" width="300">
</a>
<br>
<a href="https://github.com/abhichat85/agent-stream"><strong>agent-stream</strong></a>
<br>
<sub>The SSE event protocol for AI agents</sub>
<br><br>

```
9 event types: token · thinking · tool_use
tool_result · turn · progress · creation
done · error

Python emitter (FastAPI/Starlette)
React hook: useAgentStream
AgentStreamRecorder: JSONL session replay
Zero external dependencies
```

<sub><code>pip install agent-event-stream</code></sub><br>
<sub><code>npm install @agent-stream/react</code></sub>
</td>
<td align="center" valign="top" width="50%">
<a href="https://github.com/abhichat85/ant">
<img src="https://opengraph.githubassets.com/1/abhichat85/ant" alt="ANT" width="300">
</a>
<br>
<a href="https://github.com/abhichat85/ant"><strong>ANT — Agent-Native Terminal</strong></a>
<br>
<sub>AI-aware CLI, built in Rust</sub>
<br><br>

```
ant diagnose tests
→ detects test runner
→ captures output
→ LLM explains failures

Shows: command · cwd · risk level
before running anything.
Model never runs arbitrary shell commands.
Run history: ~/.ant/ant.db
```

</td>
</tr>
</table>

| | |
|:--|:--|
| [**pmeval**](https://github.com/abhichat85/pmeval) | Standardized benchmark for AI PM agents. 336 tasks across 14 PM categories. LLM judge with hallucination detection. 5 scoring dimensions (0–100). |
| [**knights-planning-core**](https://github.com/abhichat85/knights-planning-core) | Manus-style persistent markdown planning for Claude Code. Inspired by the pattern behind the $2B acquisition. |

---

## 📝 Writing

> Production notes from building AI infrastructure at scale.

| | |
|:--|:--|
| [**The Anatomy of an Agent Harness**](https://medium.com/@abhishekchatterjee) | May 2026 · On the engineering that lives between a rented model and a user who trusts it |
| [**I've burnt 43 Million tokens in Claude Code**](https://medium.com/@abhishekchatterjee) | Apr 2026 · What building at scale actually teaches you |
| [**Why SSE for AI agents keeps breaking at 2am**](https://dev.to/abhishek_chatterjee_33b9d/why-sse-for-ai-agents-keeps-breaking-at-2am-55ie) | Mar 2026 · The four bugs every team hits — and the protocol that fixes them |
| [**The Last Human Advantage: Judgement, Taste, and the 10x Question**](https://medium.com/@abhishekchatterjee) | Mar 2026 · What remains uniquely human when AI handles everything else |
| [**Starting Up in the Age of AI**](https://medium.com/@abhishekchatterjee) | Jun 2025 · The startup game has changed. What that means for founders. |

---

## 🛠️ Stack

<p>
<img src="https://img.shields.io/badge/Python-3776AB?style=flat-square&logo=python&logoColor=white" />
<img src="https://img.shields.io/badge/Rust-000000?style=flat-square&logo=rust&logoColor=white" />
<img src="https://img.shields.io/badge/TypeScript-3178C6?style=flat-square&logo=typescript&logoColor=white" />
<img src="https://img.shields.io/badge/React-20232A?style=flat-square&logo=react&logoColor=61DAFB" />
<img src="https://img.shields.io/badge/Next.js-000000?style=flat-square&logo=next.js&logoColor=white" />
<img src="https://img.shields.io/badge/FastAPI-009688?style=flat-square&logo=fastapi&logoColor=white" />
<img src="https://img.shields.io/badge/Anthropic_SDK-000000?style=flat-square&logoColor=white" />
<img src="https://img.shields.io/badge/PostgreSQL-336791?style=flat-square&logo=postgresql&logoColor=white" />
<img src="https://img.shields.io/badge/Convex-EE342F?style=flat-square&logoColor=white" />
<img src="https://img.shields.io/badge/Stripe-635BFF?style=flat-square&logo=stripe&logoColor=white" />
</p>

**Philosophy:** Raw SDK over abstraction wrappers. If you don't know what's underneath, you will at 2am.

---

## 📊 By the numbers

| | |
|:--|:--|
| 🛠️ 36 | Agent tools shipped in production |
| 🔥 43M | Tokens burned in Claude Code |
| 📋 336 | PM benchmark tasks (PMEval) |
| 🏗️ 5 | Products in the Einstein Labs stack |
| ✍️ 17 | Years shipping software |

---

<p align="center">
<a href="https://www.linkedin.com/in/abhishekchatterjee85">LinkedIn · 12K+</a> &nbsp;·&nbsp;
<a href="https://medium.com/@abhishekchatterjee">Medium</a> &nbsp;·&nbsp;
<a href="https://www.praxiomai.xyz">praxiomai.xyz</a> &nbsp;·&nbsp;
<a href="https://x.com/abhichat85">X / Twitter</a> &nbsp;·&nbsp;
<a href="mailto:chatterjee.85@gmail.com">Email</a>
</p>
