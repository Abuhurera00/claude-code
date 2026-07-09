# What is an Agent Harness?

An **agent harness** is the runtime scaffolding that surrounds an AI agent and
lets it actually *do* things, rather than just produce text. It is the bridge
between a model's reasoning and the outside world.

If the model is the "brain," the harness is the "body and nervous system" —
the part that receives input, hands it to the model, executes the model's
requests, observes the results, and feeds them back so the agent can iterate
toward a goal.

## Core responsibilities

A typical agent harness takes care of:

1. **Loop management** — running the perceive → reason → act → observe cycle,
   often repeatedly, until a task is complete or a stop condition is hit.
2. **Tool / action execution** — providing the agent with capabilities
   (e.g., running shell commands, reading/writing files, calling APIs) and
   safely executing the actions the model chooses.
3. **Context handling** — assembling prompts, managing history, summarizing
   or pruning conversation so the model stays within context limits.
4. **Observation feeding** — capturing the output of each action and returning
   it to the model as the next observation.
5. **Safety & guardrails** — sandboxing commands, confirming risky operations,
   enforcing permissions, and limiting side effects.
6. **State persistence** — keeping track of progress, memory, and
   configuration across turns or sessions.

## A simple mental model

```
   user input / task
          │
          ▼
   ┌──────────────┐
   │  HARNESS     │  assembles context + available tools
   └──────┬───────┘
          │ prompt
          ▼
   ┌──────────────┐
   │  MODEL       │  reasons and decides on an action
   └──────┬───────┘
          │ action (e.g. "run: ls")
          ▼
   ┌──────────────┐
   │  HARNESS     │  executes the action safely
   └──────┬───────┘
          │ observation (output)
          ▼
   ┌──────────────┐
   │  HARNESS     │  feeds result back → next loop iteration
   └──────────────┘
```

The harness owns everything *outside* the model: the environment, the tools,
the loop, and the guardrails. The model owns the reasoning.

## Harness vs. model vs. agent

| Concept   | Role                                                        |
|-----------|-------------------------------------------------------------|
| Model     | The language model that reasons and generates actions.      |
| Harness   | The runtime that executes the loop and tools around it.     |
| Agent     | The combination of model + harness working toward a goal.   |

## Why it matters

A capable model with a weak harness can't accomplish real work. A good harness
is what turns "a chatbot that talks about coding" into "a system that edits
your files, runs your tests, and ships a change." The quality of the harness
— its tools, safety model, and feedback loop — largely determines what an
agent can actually achieve.

## Examples of harness features

- Tool registry (shell, file I/O, web search, APIs)
- Permission prompts and approval flows
- Automatic retries and error recovery
- Conversation summarization to fit context windows
- Logging and audit trails of every action taken
- Pluggable backends (local model, cloud API, etc.)

---

*This README explains the general concept of an agent harness. See the rest of
this repository for the concrete implementation.*
