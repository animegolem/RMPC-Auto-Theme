---
node_id: AI-IMP-010
tags:
  - IMP-LIST
  - Implementation
  - upstream
  - documentation
kanban_status: completed
depends_on: AI-EPIC-002
confidence_score: 0.8
created_date: 2025-09-30
close_date:
--- 


# AI-IMP-010-rmpc-contingency-plan

## Summary of Issue
If generator-side fixes cannot fully mask gutter artifacts, we may need an upstream rmpc change. This ticket captures the prep work: outline the minimal patch (clearing scrollbar gutters before draw), list affected files, and document testing so we can submit a PR quickly. Completion is a markdown brief stored in `RAG/` describing the plan plus any prototype diff references.

### Out of Scope 
- Implementing or submitting the actual rmpc PR.
- Building alternate UI features beyond gutter clearing.

### Design/Approach  
Review rmpc panes that reserve scrollbars, identify injection points for `Clear` widget calls, and draft a concise change plan with code snippets. Note build/test steps and potential risks. Save as `RAG/AI-EPIC/notes/` or similar for future execution.

### Files to Touch
- `RAG/AI-EPIC/AI-EPIC-002-phase-two-theme-polish.md`: link the plan.
- New doc under `RAG/AI-EPIC/` (e.g., `AI-EPIC-002-contingency.md`).
- No generator code changes.

### Implementation Checklist
<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE> 
- [x] Audit rmpc scrollbar rendering locations and note required edits.
- [x] Draft the contingency markdown outlining code changes, testing, and submission steps.
- [x] Cross-link the document from the epic for discoverability.
 
### Acceptance Criteria
**Scenario:** Future decision to patch rmpc.
**GIVEN** the team reviews the contingency document,
**WHEN** they decide to pursue the upstream fix,
**THEN** they have a ready reference detailing files, code snippets, and tests needed to clear scrollbars without further research.

### Issues Encountered 
_None yet._

<!-- Repeat the Issue pattern above as needed based on the needs of the users request.  --> 
