# Tasks: OpenAI Normalize Header

**Input**: Design documents from `/specs/009-openai-normalize-header/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`

**Tests**: Tests are mandatory. Constitution principle II requires every behavior change to include tests written first and failing before implementation.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the 008 foundation this feature consumes is in place before any implementation work.

- [x] T001 [P] Verify feature 008 (PR #174, `model2vec-rs` 0.5) is merged to `main` or present on this branch: `EmbeddingModel::encode` accepts `normalize: Option<bool>` and `/info` exposes `normalize`; record any API drift from `specs/009-openai-normalize-header/plan.md` Technical Context.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The pure header-parsing helper that US1 (honoring) and US2 (rejecting) both consume.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [x] T002 Implement a pure parsing helper for the `X-Normalize` header in `src/routes/embeddings.rs`: input is all header values for `X-Normalize`, output is `Ok(None)` when absent, `Ok(Some(bool))` for a single case-insensitive `true`/`false`, and an error for any other value, an empty value, or more than one value — per the value-state table and validation rules in `specs/009-openai-normalize-header/data-model.md` (Embedding Request Headers.X-Normalize).
- [x] T003 [P] Add unit tests for the parser in `tests/openai_contract.rs` (plain `#[test]`, no HTTP): absent → `Ok(None)`; `true`/`TRUE`/`False`/`fAlSe` → accepted case-insensitively; `maybe`, `1`, `0`, `on`, empty string → error; two values (duplicate) → error.

**Checkpoint**: Parser behavior is fully pinned down and unit-tested; handler wiring can begin.

---

## Phase 3: User Story 1 - Per-Request Normalization on `/v1/embeddings` (Priority: P1) 🎯 MVP

**Goal**: OpenAI-compatible clients control L2 normalization per request via `X-Normalize` while the JSON contract stays byte-identical.

**Independent Test**: Same `/v1/embeddings` request sent with `X-Normalize: true` vs `false` yields vectors with different norms (true ≈ 1.0), and the response shape is unchanged in both cases.

### Tests for User Story 1 (MANDATORY) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation.**

- [x] T004 [P] [US1] Add contract tests in `tests/openai_contract.rs`: absent header → `200` with unchanged OpenAI response shape and no header echo (SC-001 regression); `X-Normalize: true` → `200`; `X-Normalize: false` → `200`; combined with `encoding_format: "base64"` → `200` with decodable base64 vectors (FR-003, data-model.md Embedding Response).
- [x] T005 [P] [US1] Add integration tests in `tests/openai_integration.rs`: identical input with `true` vs `false` produces vectors whose L2 norms differ, `true` norm ≈ 1.0 within f32 tolerance; mixed casing (`x-normalize: FALSE`) behaves like `false` (FR-003).

### Implementation for User Story 1

- [x] T006 [US1] Wire the header into the embeddings handler in `src/routes/embeddings.rs`: extract all `X-Normalize` values from the request headers, run the T002 parser, and pass the parsed `Option<bool>` as the `normalize` argument to `EmbeddingModel::encode` (data-model.md Embedding Request Headers.X-Normalize value-state table); the response construction is untouched (FR-005).

**Checkpoint**: User Story 1 is fully functional and independently testable; requests without the header are byte-identical to the pre-change service.

---

## Phase 4: User Story 2 - Invalid Header Values Fail Clearly (Priority: P2)

**Goal**: Unrecognized, empty, or duplicated `X-Normalize` values produce the standard `400` error body; body validation errors keep precedence.

**Independent Test**: `/v1/embeddings` with `X-Normalize: maybe` returns `400` with `{"error": "invalid_request", "message": ...}` and no embeddings; with an invalid body AND invalid header, the body error wins.

### Tests for User Story 2 (MANDATORY) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation.**

- [x] T007 [P] [US2] Add contract tests in `tests/openai_contract.rs`: unrecognized value (`maybe`, `1`, `on`) → `400` `invalid_request` with a message naming `X-Normalize` and the accepted values (FR-004); empty value → `400`; duplicate header → `400`; precedence: empty input body + `X-Normalize: maybe` → the body error message and status (`400`), not the header error (research.md "Validation order" decision).

### Implementation for User Story 2

- [x] T008 [US2] Implement the validation order in `src/routes/embeddings.rs`: run the existing body validation first, then parse the header via the T002 helper and map the error to `AppError::BadRequest` with a message naming `X-Normalize`, the offending value, and the accepted values `true`/`false` (data-model.md validation rules; FR-004).

**Checkpoint**: User Stories 1 AND 2 are both independently functional; no pre-existing error path changed behavior (SC-001).

---

## Phase 5: User Story 3 - Documented, Discoverable Extension (Priority: P3)

**Goal**: The header is discoverable in the interactive OpenAPI docs and the API documentation, marked as a service-specific extension.

**Independent Test**: `GET /docs/openapi.json` lists `X-Normalize` as a parameter on the `/v1/embeddings` POST path; `docs/api/openai.md` describes values, default, and an example.

### Tests for User Story 3 (MANDATORY) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation.**

- [x] T009 [P] [US3] Add a contract test in `tests/openai_contract.rs`: `GET /docs/openapi.json` → `200` and the JSON contains `X-Normalize` among the parameters of the `/v1/embeddings` POST operation (FR-007).

### Implementation for User Story 3

- [x] T010 [US3] Add a utoipa header parameter annotation for `X-Normalize` on the embeddings path in `src/routes/embeddings.rs` (description marks it as a service-specific extension with accepted values `true`/`false`, absent = model default; FR-007).
- [x] T011 [P] [US3] Document the header in `docs/api/openai.md`: values table, absent-header default, example request with `-H 'X-Normalize: false'`, error behavior on invalid values, and an explicit "service-specific extension" note referencing `specs/009-openai-normalize-header/contracts/openai_embeddings.md` (FR-007, FR-008).

**Checkpoint**: All three user stories are independently functional; the extension is discoverable and auditable.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Release notes, end-to-end validation, and final gates.

- [x] T012 [P] Add a release-notes/README note describing the `X-Normalize` extension and its contract-governance rationale (Constitution III migration/exception note; reference `specs/009-openai-normalize-header/contracts/openai_embeddings.md`).
- [x] T013 Run the full validation flow from `specs/009-openai-normalize-header/quickstart.md` sections 1–8 (default behavior, forced norms, casing, invalid values, precedence, TEI ignoring the header, docs checks, automated tests).
- [x] T014 Run `cargo fmt -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo bench --no-run`; confirm no compile-time regression in benchmarks (Constitution V; full bench run happens pre-release).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on T001; blocks all user stories (T002 → T003).
- **US1 (Phase 3)**: Depends on T002 (parser) — its handler task T006 consumes it.
- **US2 (Phase 4)**: Depends on T002 (parser error arm) and T006 (same handler file — sequence T008 after T006).
- **US3 (Phase 5)**: Depends on T006 (the annotated path must exist with the header implemented); T009 validates T010's annotation.
- **Polish (Phase 6)**: Depends on all user stories being complete.

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational — no dependencies on other stories. MVP.
- **US2 (P2)**: Consumes the same parser and handler as US1; contract tests (T007) can be written in parallel with US1 tests, but T008's edit to `src/routes/embeddings.rs` must follow T006.
- **US3 (P3)**: Documentation-only after the implementation exists; T009/T010/T011 have no runtime dependency on US2.

### Within Each User Story

- Tests (T004/T005, T007, T009) MUST be written and FAIL before their story's implementation tasks.
- Parser (T002) before wiring (T006, T008).
- Annotation (T010) before its validating test expectation is considered satisfied (T009 written first, passes after T010).

### Parallel Opportunities

- Phase 2: T003 (tests) can be written alongside T002.
- Tests within a story: T004 ∥ T005; T007's cases are one file; T009 ∥ T011 (different files).
- US2 tests (T007) can be written while T006 is in progress (different files), but T008 edits the same handler file as T006 — sequence them.

---

## Parallel Example: User Story 1

```bash
# Launch both US1 test tasks together:
Task: "Contract tests for header state matrix in tests/openai_contract.rs"
Task: "Integration tests for norm differences in tests/openai_integration.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 (Setup) and Phase 2 (Foundational).
2. Complete Phase 3 (US1): parser wiring into the handler.
3. **STOP and VALIDATE**: run T004/T005 and quickstart sections 1–3; requests without the header are unchanged.
4. Deploy/demo if ready.

### Incremental Delivery

1. Setup + Foundational → parser pinned and unit-tested.
2. US1 → validate → MVP (OpenAI clients gain normalization control).
3. US2 → validate (quickstart sections 4–5) → error handling hardens the extension.
4. US3 → validate (quickstart sections 7) → discoverable and documented.
5. Polish → release notes, full quickstart run, final gates → release via release-plz (minor bump).

### Single-Developer Strategy

Work strictly in phase order; the [P] markers indicate file-level independence within each phase.

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
