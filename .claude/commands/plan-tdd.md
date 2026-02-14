---
description: Plan feature implementation using strict TDD methodology
---

# TDD Planning Request

Plan the implementation of: **$ARGUMENTS**

Use the planning-agent to create a detailed implementation plan following **strict TDD methodology**.

## Requirements

1. **Structure the plan as TDD cycles** (RED → GREEN → REFACTOR)
2. **Start with the smallest testable unit**
3. **Each cycle must specify:**
   - The exact test to write (with assertions)
   - The minimal code to make it pass
   - Any refactoring needed

## Output Format

```
## Task: [Feature Name]

### Cycle 1: [Smallest testable unit]
- RED: Write test `test_name` that verifies [behavior]
  - Assert: [specific assertion]
- GREEN: Implement [minimal code description]
- REFACTOR: [if needed]

### Cycle 2: [Next unit]
...
```

## Context

- Read the existing codebase to understand patterns
- Follow existing test conventions
- Ensure each test is independent and isolated
- Tests go in the `tests` module within each file (inline tests)

Now, invoke the planning-agent to create this TDD plan.
