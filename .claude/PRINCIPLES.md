# Engineering Principles for AgentRoot

## Core Principle: Correctness is Life and Death

**"This is not an 'important question', this is the alpha and omega of what we do. If our changes are not correct, people die."**

This is the fundamental truth of software engineering:
- Verification is NOT an afterthought
- Verification is NOT optional
- Verification is NOT something you do "if time permits"
- **Verification is the FIRST thing, not the last thing**

### Before ANY commit:

1. **Run the complete test suite** - `cargo test --workspace --all-targets`
2. **Check ALL compilation** - `cargo build --workspace --all-targets`
3. **Run clippy** - `cargo clippy --workspace --all-targets`
4. **Build examples** - `cargo build --examples`
5. **Document what was verified** - What tests passed? What was checked?

### Never:

- ❌ Commit first, test later
- ❌ "I'll fix the tests in the next commit"
- ❌ "The tests probably still work"
- ❌ Assume anything - VERIFY EVERYTHING

### Always:

- ✅ Test BEFORE committing
- ✅ Verify BEFORE pushing
- ✅ Document what you verified
- ✅ Be rigorous, thorough, and exhaustive

## Task Management (MANDATORY)

**Every multi-step task MUST use the TodoWrite tool:**

1. **Create TODO list FIRST** - Before writing any code
2. **Mark tasks in_progress** - One at a time
3. **Mark tasks completed** - ONLY after verification
4. **Never skip** - This is not optional

**Why:** As context grows, todos keep you accurate and on track.

### Todo Usage Pattern:
```
1. User requests feature
2. IMMEDIATELY create todos with TodoWrite
3. Mark first todo as in_progress
4. Complete the work
5. VERIFY it works (tests pass, compiles, etc.)
6. Mark todo as completed
7. Move to next todo
```

## Code Quality Principles (NON-NEGOTIABLE)

### SOLID Principles
- **S**ingle Responsibility: Each function/module does ONE thing
- **O**pen/Closed: Open for extension, closed for modification
- **L**iskov Substitution: Subtypes must be substitutable
- **I**nterface Segregation: Many specific interfaces > one general
- **D**ependency Inversion: Depend on abstractions, not concretions

### DRY (Don't Repeat Yourself)
- Extract common code into functions
- Use traits for shared behavior
- Never copy-paste code

### KISS (Keep It Simple, Stupid)
- Prefer simple solutions over clever ones
- Avoid unnecessary abstractions
- Write code humans can understand
- If it's complex, it's probably wrong

### Additional Quality Standards
- **Modular code** - Small, focused functions (<50 lines)
- **Early returns** - Fail fast, avoid nesting
- **Type safety** - Explicit types, no `unwrap()` in libraries
- **Clear naming** - No abbreviations, be explicit

## Zero-Tolerance Policy

### NEVER ALLOWED:
- ❌ `TODO` comments
- ❌ `FIXME` comments
- ❌ Placeholder functions
- ❌ Stub implementations
- ❌ Mock data (except in tests)
- ❌ "I'll fix it later"
- ❌ Commented-out code
- ❌ Incomplete features

### THE CARDINAL SIN: LYING

**NEVER LIE. NEVER FAKE. NEVER PRETEND.**

If you don't know - SAY SO.
If you can't do it - SAY SO.
If it's broken - SAY SO.
If you're unsure - SAY SO.

**Honesty > Ego**
**Truth > Convenience**
**Reality > Appearance**

### Production-Ready or Nothing
Every single line of code must be:
- ✅ Fully implemented
- ✅ Tested and working
- ✅ Production-quality
- ✅ Ready to ship

There is no "good enough for now". There is no "temporary solution". There is no "quick hack".

**If it's not 100% real, working code - DON'T COMMIT IT.**

## Rigor Checklist

Before ANY code change:
```
[ ] TodoWrite used to track all tasks
[ ] Each todo verified as complete before moving on
[ ] All tests run
[ ] All tests documented (what passed, what failed, what's expected)
[ ] All compilation verified
[ ] All examples checked
[ ] No TODOs/FIXMEs/placeholders in code
[ ] Code follows DRY, SOLID, KISS principles
[ ] Breaking changes identified
[ ] Impact analyzed
[ ] Rollback plan exists
```

## Remember

This is not about being careful. This is not about being thorough.

**This is about life and death.**

If you skip verification, you kill the project. If you slack on rigor, you destroy trust. If you commit broken code, you fail at the most fundamental level.

---

*Created: 2026-01-21*
*Reason: Critical reminder after committing without full verification*
