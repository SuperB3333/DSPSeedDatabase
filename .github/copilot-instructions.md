# Copilot Agent Configuration

## Main Agent

```yaml
name: main
description: Primary agent for DSPSeedDatabase tasks
instructions: .ai/agent-instructions.md
auto_load: true
```

All agents inherit operating instructions from `.ai/agent-instructions.md`, which defines:
- Read order (agent-instructions → memory.md → local.md)
- Core behavior (don't hallucinate, be concise, log durable findings)
- Memory management and precedence rules

For project-specific context, see `.ai/memory.md`.
For personal preferences, see `.ai/local.md`.
