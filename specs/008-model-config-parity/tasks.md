# Tasks: Model Config Parity

**Input**: Design documents from `/specs/008-model-config-parity/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`

**Tests**: Tests are mandatory. Constitution principle II requires every behavior change to include tests written first and failing before implementation.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the merged `model2vec-rs` 0.5 dependency exposes the API this feature consumes, and prepare the test fixture.

- [x] T001 [P] Verify `model2vec-rs` 0.5 (already merged via PR #171) exposes `StaticModel::max_length()`, `StaticModel::normalize()`, and the `normalize: Option<bool>` parameter on `encode_with_args`; record any drift from `specs/008-model-config-parity/research.md` findings.
- [x] T002 [P] Create the per-model truncation test fixture: a local model directory (copy of a downloaded model2vec model, e.g. under `tests/fixtures/` or `$TMPDIR` per `tests/common/mod.rs` conventions) whose `config.json` declares `max_length: 256`; document how integration tests reference it.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Plumbing in the model wrapper and configuration surface that ALL user stories depend on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [x] T003 Add accessors `max_length() -> Option<usize>` and `normalize() -> bool` to `EmbeddingModel` in `src/model/embedding.rs`, delegating to `StaticModel::max_length()` / `StaticModel::normalize()`, with doc comments (`missing_docs = "warn"` applies).
- [x] T004 Thread a `normalize: Option<bool>` parameter through `EmbeddingModel::encode` and `EmbeddingModel::encode_single` in `src/model/embedding.rs`, passing it as the 4th argument to `StaticModel::encode_with_args`; update existing internal callers in `src/model/embedding.rs` to pass `None` so behavior is unchanged (FR: "absent/null = model's configured default").
- [x] T005 [P] Add unit tests in `tests/config_unit.rs` asserting that the existing config parsing still succeeds after `Config.max_input_length` changes to `Option<usize>` (compile-level regression guard for T006).

**Checkpoint**: Model wrapper exposes 0.5 metadata and per-call normalization; configuration is ready to become per-model.

---

## Phase 3: User Story 1 - Per-Model Maximum Input Length (Priority: P1) 🎯 MVP

**Goal**: Each model truncates inputs at its own resolved limit: explicit operator `MAX_INPUT_LENGTH` override wins; otherwise the model's `config.json` `max_length`; otherwise `512` (contracts/config.md resolution table).

**Independent Test**: Load two models declaring different `max_length` values without `MAX_INPUT_LENGTH` set; `/info` and startup logs report each model's own limit; setting the override replaces both with a warning.

### Tests for User Story 1 (MANDATORY) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation.**

- [x] T006 [P] [US1] Add unit tests for the resolution precedence table in `tests/config_unit.rs`: explicit override wins (including when the model declares a value); model config `max_length` used when unset; absent config → `512`; `max_length: null` → `512` ("the service never runs truly unlimited truncation").
- [x] T007 [P] [US1] Add integration test in `tests/tei_integration.rs` (or a new `tests/model_config_integration.rs`): with the T002 fixture declaring `max_length: 256` and no override, `/info` reports `max_input_length: 256` and inputs longer than 256 tokens produce embeddings identical to manually truncated inputs at 256 tokens.

### Implementation for User Story 1

- [x] T008 [US1] Implement `resolve_max_input_length(explicit: Option<usize>, model_max: Option<usize>) -> usize` as a pure, documented function following the resolution table in `specs/008-model-config-parity/data-model.md`, placed in `src/config.rs` (or `src/model/mod.rs` if it avoids a circular dependency).
- [x] T009 [US1] Change `Config.max_input_length` in `src/config.rs` from `usize` (default 512) to `Option<usize>` (`--max-input-length` / `MAX_INPUT_LENGTH` spelling unchanged, default now "unset"); reject `Some(0)` at startup ("a zero token limit cannot produce embeddings").
- [x] T010 [US1] Update `LoadedModel::load` in `src/model/mod.rs` to accept the explicit `Option<usize>` and store the resolved per-model limit in `LoadedModel.max_input_length` (type stays `usize`); thread the value from `state.rs` startup wiring.
- [x] T011 [US1] Add startup logging in `src/model/mod.rs`: one `tracing::info!` line per loaded model recording the resolved limit, and a `tracing::warn!` when an explicit override replaces a model's configured value (model id, override, configured value).
- [x] T012 [US1] Ensure `src/routes/tei.rs` info handlers and `src/routes/embeddings.rs` encode calls read the resolved per-model `LoadedModel.max_input_length` without further changes (verify no call site assumed the old global default).

**Checkpoint**: User Story 1 is fully functional and independently testable; existing deployments that set `MAX_INPUT_LENGTH` behave identically to before.

---

## Phase 4: User Story 2 - Per-Request `normalize` on TEI Endpoints (Priority: P2)

**Goal**: TEI clients can request raw or L2-normalized embeddings per request via an optional `normalize` boolean; absent/null keeps the model's configured default.

**Independent Test**: Send identical inputs with `normalize: false` and `normalize: true` to `/embed`; vector norms differ accordingly (contracts/tei.md).

### Tests for User Story 2 (MANDATORY) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation.**

- [x] T013 [P] [US2] Add contract tests in `tests/tei_contract.rs`: `normalize` accepted on `/embed` and `/tei/{model_id}/embed`; omitted field and explicit `"normalize": null` produce a 200 with the unchanged response shape (array of float arrays); `"normalize": true`/`false` are accepted as booleans.
- [x] T014 [P] [US2] Add integration test in `tests/tei_integration.rs`: same input with `normalize: false` vs `normalize: true` yields vectors whose L2 norms differ (`false` norm ≠ 1.0 for a normalized model, `true` norm ≈ 1.0 within f32 tolerance).

### Implementation for User Story 2

- [x] T015 [US2] Add `normalize: Option<bool>` with serde default to `TeiEmbedRequest` in `src/routes/dto.rs`, with utoipa schema documentation ("absent/null = model's configured default").
- [x] T016 [US2] Pass `request.normalize` through `EmbeddingModel::encode` in `src/routes/tei.rs` handlers (`tei_embed` and the per-model variant); validation logic unchanged (`validate_tei_request` untouched).
- [x] T017 [US2] Confirm `src/routes/embeddings.rs` (`/v1/embeddings`) gained NO normalization parameter (FR-5: "The OpenAI-compatible endpoints MUST NOT gain a normalization parameter") and `tests/openai_contract.rs` still passes unchanged.

**Checkpoint**: User Stories 1 AND 2 are both independently functional; existing requests (no `normalize` field) are byte-identical to pre-change behavior (spec SC-2).

---

## Phase 5: User Story 3 - Accurate `/info` Metadata (Priority: P3)

**Goal**: `/info` (root and per-model) reports the model's actual `normalize` setting and its resolved `max_input_length`.

**Independent Test**: Query `/info` and per-model `/info`; compare `normalize` against the model's `config.json` and `max_input_length` against the resolution table.

### Tests for User Story 3 (MANDATORY) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation.**

- [x] T018 [P] [US3] Add contract tests in `tests/tei_contract.rs`: `/info` and `/tei/{model_id}/info` responses contain `normalize: bool` and the resolved `max_input_length`, matching contracts/tei.md field table.

### Implementation for User Story 3

- [x] T019 [US3] Add `normalize: bool` to `ModelInfo` in `src/routes/dto.rs` with utoipa schema documentation; populate it in both TEI info handlers in `src/routes/tei.rs` from `EmbeddingModel::normalize()`.
- [x] T020 [US3] Verify `max_input_length` in `ModelInfo` is populated from the per-model resolved value (set in T010) for both info handlers in `src/routes/tei.rs`.

**Checkpoint**: All three user stories are independently functional; clients can validate assumptions against `/info`.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation sync, OpenAPI, release notes, and final validation.

- [x] T021 [P] Update utoipa path/response annotations so `/docs` reflects `TeiEmbedRequest.normalize` and the new `ModelInfo.normalize` field (FR-8: "OpenAPI documentation (`/docs`) MUST reflect the `normalize` field and the updated `ModelInfo` schema").
- [x] T022 [P] Update VitePress docs in `docs/`: TEI endpoints page (new `normalize` field), configuration reference (`--max-input-length` per-model semantics per contracts/config.md), and the `/info` response example.
- [x] T023 [P] Update `README.md` command examples/release notes draft: behavior change callout for models declaring `max_length` in `config.json` (spec Assumptions; Constitution III migration-note rule).
- [x] T024 Run the full validation flow from `specs/008-model-config-parity/quickstart.md` sections 1–5 locally (per-model limits, normalize norms, `/info`, OpenAI unchanged, automated tests).
- [x] T025 Run `cargo fmt -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo bench`; confirm no benchmark regression greater than 10% (Constitution V).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately; T001 and T002 are independent.
- **Foundational (Phase 2)**: Depends on T001 (API surface confirmed); blocks all user stories.
- **US1 (Phase 3)**: Depends on Phase 2 (T003, T004) and T002 (fixture) for its integration test.
- **US2 (Phase 4)**: Depends on Phase 2 (T004 normalize plumbing); independent of US1.
- **US3 (Phase 5)**: Depends on T003 (`normalize()` accessor) and T010 (per-model limit); otherwise independent.
- **Polish (Phase 6)**: Depends on all desired user stories being complete.

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational — no dependencies on other stories. MVP.
- **US2 (P2)**: Can start after Foundational — no dependencies on US1; touches `src/routes/tei.rs` handlers only at the encode call site, so coordinate if run in parallel with US1's T012.
- **US3 (P3)**: Can start after Foundational — uses T003/T010 outputs; independently testable.

### Within Each User Story

- Tests (T006/T007, T013/T014, T018) MUST be written and FAIL before their story's implementation tasks.
- Resolution/config changes (T008–T009) before registry wiring (T010–T012).
- DTO changes (T015, T019) before handler wiring (T016, T019–T020).
- Story complete before moving to the next priority.

### Parallel Opportunities

- Phase 1: T001 ∥ T002.
- Phase 2: T005 ∥ (T003 → T004) — T003 and T004 are sequential edits to the same file.
- Tests within a story marked [P] can be written together (T006 ∥ T007; T013 ∥ T014).
- US2 and US3 are largely disjoint from US1 (dto.rs fields vs. config/registry) and can proceed in parallel once Phase 2 lands — except T012 and T016 both touch `src/routes/tei.rs`, so sequence those two edits.

---

## Parallel Example: User Story 1

```bash
# Launch both US1 test tasks together:
Task: "Unit tests for resolution precedence table in tests/config_unit.rs"
Task: "Integration test for per-model truncation in tests/tei_integration.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 (Setup) and Phase 2 (Foundational).
2. Complete Phase 3 (US1): resolution, per-model limits, startup logging.
3. **STOP and VALIDATE**: run T006/T007 tests and quickstart section 1; existing deployments with `MAX_INPUT_LENGTH` set are unchanged.
4. Deploy/demo if ready.

### Incremental Delivery

1. Setup + Foundational → foundation ready.
2. US1 → validate → MVP (correct per-model truncation).
3. US2 → validate (quickstart section 2) → TEI clients gain normalization control.
4. US3 → validate (quickstart section 3) → metadata accuracy.
5. Polish → docs, OpenAPI, release notes, benchmarks → release via release-plz (minor bump).

### Single-Developer Strategy

Work strictly in phase order; the [P] markers then just indicate file-level independence within each phase.

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
