---
pr-number: 2948
pr-url: https://github.com/torrust/torrust-tracker/pull/2948
last-updated-utc: "2026-11-18 14:39"
---

<!-- skill-link: process-pr-review -->

# PR #2948 Review Audit

Source: pull-request reviews and inline review threads for https://github.com/torrust/torrust-tracker/pull/2948.

<!-- Copied verbatim into each audit record after placeholder replacement. -->

## Ownership

The PR author owns this tracked audit record. Reviewers, including repository review agents,
deliver findings through GitHub and have no repository-artifact obligation.

- Post-merge workflow approval: N/A

<!-- Copied verbatim into each audit record. -->

## Status Values

- Relationship: `ORIGINAL`, `RE_RAISE_OF:<FindingId>`
- Disposition: `FIXED`, `NO_ACTION`, `SUPERSEDED`, `FOLLOW_UP`
- Thread state: `OPEN`, `RESOLVED`, `NON_RESOLVABLE`, `SUPERSEDED`. `OPEN` applies prospectively
  to audits created or updated for approved follow-up work; historical audits remain valid without
  bulk migration.
- Severity: `Blocker`, `Major`, `Minor`, `Nit`, `Suggestion`; append `(inferred)` when derived
  from free prose.
- Author class: `Copilot`, `Human`, `Unknown`
- Category: `link-integrity`, `formatting`, `metadata`, `testing`, `correctness`,
  `documentation`, `maintainability`, `security`, `other`
- An outdated thread whose concern was fixed is `FIXED`/`RESOLVED`, even when GitHub marks the
  original thread outdated after the push. For in-PR feedback, use `NO_ACTION`/`SUPERSEDED` only
  for a duplicate, superseded, or no-change concern. A post-merge `NO_ACTION` requires maintainer
  approval to decline the follow-up work.
- When the current identity cannot resolve a thread (`viewerCanResolve` is `false`), keep the
  disposition established from the current tree and record `Thread state=OPEN`. The reviewer or PR
  author resolves it; permission says nothing about whether the concern was fixed.

## Findings

Round 1 is the approving review `5958211073` of `da2ce7` (F1 and F2, reviewer-provided IDs) on the first head; Copilot's
review `5958205525` on the same head recommends approval with no findings and has no row.
Round 2 is the review `5958517782` of `da2ce7` (COMMENTED, F20, reviewer-provided ID) on the head that carried the round 1
fixes, before the audit commit reached the branch.

| Finding ID | Review finding reference | Author class | Severity | Category | Relationship | Disposition | Thread state |
| ---------- | ------------------------ | ------------ | -------- | -------- | ------------ | ----------- | ------------ |
| F1 | `review-finding:pr-2948-f1` | Human | Minor | correctness | ORIGINAL | FIXED | RESOLVED |
| F2 | `review-finding:pr-2948-f2` | Human | Minor | documentation | ORIGINAL | FIXED | RESOLVED |
| F20 | `review-finding:pr-2948-f20` | Human | Minor | documentation | ORIGINAL | FIXED | RESOLVED |

## Finding Details

### F1 - F16 cites the reword commit, not the commit that made the change

- PR number: 2948
- Source review ID: 5958211073
- Reviewer finding ID: F1
- Source URL: https://github.com/torrust/torrust-tracker/pull/2948#discussion_r4615242881
- Concern: The change F16 asked for landed in `docs(pr-reviews): [#2904] record the late review of PR #2920 and correct the verification of F15`; the cited reword commit only reorders that line, so "each with its fixing commit's subject" did not hold for F16.
- Solution: The row of F16 cites both subjects, the correction first, separated by a semicolon, and a correction entry in that record's log says so.
- Current-tree verification: The Resolution reference of F16 in `docs/pr-reviews/pr-2920-review/PR-REVIEW.md` names both subjects; `validate-audit-record.py --pr-number 2920` exited 0 (19 rows).
- Resolution reference: `docs(pr-reviews): [#2904] cite the correcting commit of F16 in the review audit of PR #2920`
- Follow-up PR URL: N/A
- Reply URL: https://github.com/torrust/torrust-tracker/pull/2948#discussion_r4615429797

### F2 - The record stops before the end of step 6

- PR number: 2948
- Source review ID: 5958211073
- Reviewer finding ID: F2
- Source URL: https://github.com/torrust/torrust-tracker/pull/2948#discussion_r4615242891
- Concern: Step 6 of Reviews Submitted After Merge ends with recording zero unresolved threads after a GraphQL refresh; the record said only that the threads would be resolved after the push.
- Solution: A new entry records the push time, the resolution of the four threads by 13:49:23Z and the refresh with 0 unresolved threads.
- Current-tree verification: A GraphQL refresh on pull request #2920 shows 0 unresolved threads; the new entry gives the times.
- Resolution reference: `docs(pr-reviews): [#2904] record the resolution of F16 to F19 in the review audit of PR #2920`
- Follow-up PR URL: N/A
- Reply URL: https://github.com/torrust/torrust-tracker/pull/2948#discussion_r4615430354

### F20 - The two answered threads of this pull request have no review audit record

- PR number: 2948
- Source review ID: 5958517782
- Reviewer finding ID: F20
- Source URL: https://github.com/torrust/torrust-tracker/pull/2948#discussion_r4615505372
- Concern: At the reviewed head `0d9309651` the threads of F1 and F2 were answered and resolved, but the branch carried no `docs/pr-reviews/pr-2948-review/PR-REVIEW.md`.
- Solution: The record was written before the replies and validated before the threads were resolved, but its first commit attempt failed the pre-commit gate (markdownlint MD038), so it reached the branch only with the next push, after the reviewed head. This entry and the log record the delay.
- Current-tree verification: `git log --format='%cI %s' -- docs/pr-reviews/pr-2948-review/PR-REVIEW.md` shows the record's first commit at 14:24:54Z, pushed at 14:28:10Z; the Processing Log gives the validator run at 14:20:51Z and the resolutions by 14:20:55Z.
- Resolution reference: `docs(pr-reviews): [#2904] record round 1 in the review audit of PR #2948`
- Follow-up PR URL: N/A
- Reply URL: https://github.com/torrust/torrust-tracker/pull/2948#discussion_r4615661680

## Processing Log

- 2026-11-18 14:17 UTC - Started audit, before the replies of round 1.
- 2026-11-18 14:21 UTC - Round 1: the approving review `5958211073` of `da2ce7` (submitted 14:07:07Z, forge `submitted_at`) with two findings. The fixes were committed at 14:14:30Z and 14:16:14Z (`git log --format=%cI`, in UTC) and pushed; `git ls-remote` showed the fork branch at the local head at 14:20:24Z (`date -u`). The two replies were posted at 14:20:34Z and 14:20:36Z (forge `created_at`), after the push. `validate-audit-record.py --pr-number 2948 --base torrust/develop`, with its `gh` calls made through `gh-bot`, exited 0 (2 rows, 0 failures) at 14:20:51Z; the two threads were then resolved by their thread IDs by 14:20:55Z, and a GraphQL refresh showed 0 unresolved threads.
- 2026-11-18 14:39 UTC - Late commit of this record: its first commit attempt at 14:21 failed the pre-commit gate (markdownlint MD038 on a code span holding a semicolon and a space), so the round 1 entry above was committed only at 14:24:54Z (`git log --format=%cI`, in UTC) and pushed at 14:28:10Z (`git ls-remote`, `date -u`), after the replies and resolutions it records.
- 2026-11-18 14:39 UTC - Round 2: the review `5958517782` of `da2ce7` (COMMENTED, submitted 14:25:42Z, forge `submitted_at`), on `0d9309651`, with one finding, recorded as F20: the record was missing at that head. It reached the branch at the next push, 14:28:10Z, before the reply, posted at 14:36:31Z (forge `created_at`). `validate-audit-record.py --pr-number 2948 --base torrust/develop`, with its `gh` calls made through `gh-bot`, exited 0 (3 rows, 0 failures) at 14:39:46Z; the thread of F20 was then resolved by its thread ID at 14:39:48Z.

<!-- Copied verbatim into each audit record. -->

## Completion Rules

- Re-derive the reply claim against the current tree before replying or resolving a thread.
- Reply on every resolvable thread before resolving it.
- For an outdated thread whose concern was fixed, record `Disposition=FIXED` and
  `Thread state=RESOLVED`, even if GitHub marks the thread outdated after the push. For a
  duplicate, superseded, or no-change in-PR thread, reply exactly
  `Superseded by <FindingId>: <reason>.`, record `Disposition=NO_ACTION` and
  `Thread state=SUPERSEDED`, then resolve it. A post-merge `NO_ACTION` requires maintainer
  approval to decline the follow-up work. When the current identity cannot resolve a thread
  (`viewerCanResolve` is `false`), still reply, keep the disposition established from the current
  tree, and record `Thread state=OPEN`; the reviewer or PR author resolves it.
- A consolidated PR conversation response may cover multiple review rounds only when it names
  every review ID and every finding ID with its disposition and resolution reference. Record its
  durable URL in each related row.
- Cite a fix by its unique Conventional Commit subject or durable reply URL, never by a branch SHA
  that can change after a rebase.
- Refresh review threads using GraphQL and confirm that no unresolved actionable thread remains.
