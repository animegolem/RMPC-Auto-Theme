---
node_id: AI-IMP-*  # For sub-tickets use AI-IMP-NNN-N (e.g., AI-IMP-105-1)
tags:
  - IMP-LIST
  - Implementation
  - {more tags as needed}
kanban_status: {Legal Values: "backlog", "planned", "in-progress", "completed", "cancelled", "deferred"}
depends_on: {list ADRs, IMPs, etc. Optional. Only fill if needed.}
parent_epic: {[[AI-EPIC-XXX]] | Auto-populated by generate-index.sh}
confidence_score: {0.0-1.0}
date_created: {YYYY-MM-DD}
date_completed: { YYYY-MM-DD | Don't fill on creation }
--- 


# AI-IMP-XXX-{{title-in-snake-case}}

<!-- 
Fill out the YAML Frontmatter in full. 
You SHOULD provide your confidence in the accuracy and completeness of your plan on a scale of 0.0 to 1.0.
Fill out all headings below removing these bounded comments.
Be professional in tone. Be concise but complete.
Replace {tags}. {LOC|X} should be replaced by your actual output and indicates the maximum lines per heading. 
--> 

## {Summary of Issue #1}
<!-- 
You MUST define the Current issue, it's scope, and intended remediation
You SHOULD define a single, measurable outcome. What specific state means we are done?
You MAY link to project docs when relevant (eg, adr, imp, log) 
--> 
{LOC|20}

### Out of Scope 
<!-- Explicitly list what is NOT being done. -->
{LOC|10}

### Design/Approach  
<!-- High-level approach, alternatives considered, rationale. Link to diagrams/ADRs. -->
{LOC|25}

### Files to Touch
<!-- 
Implementer SHOULD Review before filling out this document to list the files you have a high confidence will require edits.
This review will help you make better implementation plans.
You MAY provide paths with **extremely** concise reasons. Prefer globs where helpful.
<EXAMPLE>
`src/.../module.ts`: add …
`tests/.../module.spec.ts`: add …
`migrations/20250911_add_index.sql`: new …
</EXAMPLE> 
--> 
{LOC|25}

### Implementation Checklist
<!-- 
Format MUST be a checklist. 
Each item MUST be atomic, verifiable, and executable without ambiguity. Use a simple checklist format. 
<EXAMPLE>
`- [ ]` Action: specific file/function/test with exact change 
</EXAMPLE>
Do not remove the <CRITICAL_RULE> from your final output. Replace {LOC|X} with your checklist. 
--> 

<CRITICAL_RULE>
Before marking an item complete on the checklist MUST **stop** and **think**. Have you validated all aspects are **implemented** and **tested**? 
</CRITICAL_RULE> 

{LOC|75}
 
### Acceptance Criteria
<!-- 
Implementations MUST be validated. The implementer SHOULD use Given-When-Then-(and) testing. 
<EXAMPLE> 
**Scenario:** Customer is placing an online order for sprockets. 
**GIVEN** the online storefront is configured and running. 
**WHEN** A customer places an order for 14 sprockets and we have 18. 
**THEN** The customer gets an order confirmation screen. 
**AND** The inventory is updated and lists 4 remaining.
**AND** The customers credit card is charged.
**THEN** a confirmation email is sent to the customer.
</EXAMPLE> 
You MAY use as many or as few 'THEN AND' patterns as required.  
--> 


### Issues Encountered 
<!-- 
The comments under the 'Issues Encountered' heading are the only comments you MUST not remove 
This section is filled out post work as you fill out the checklists.  
You SHOULD document any issues encountered and resolved during the sprint. 
You MUST document any failed implementations, blockers or missing tests. 
-->  
{LOC|20}

<!-- Repeat the Issue pattern above as needed based on the needs of the users request.  --> 
