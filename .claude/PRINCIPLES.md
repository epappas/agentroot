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

## Rigor Checklist

Before ANY code change:
```
[ ] All tests run
[ ] All tests documented (what passed, what failed, what's expected)
[ ] All compilation verified
[ ] All examples checked
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
