# Research: OpenAI Normalize Header

## Decision: Canonical header name and casing

- **Decision**: `X-Normalize`, documented with canonical casing; matching on the wire is case-insensitive per HTTP semantics (the web framework normalizes header names, so `x-normalize`, `X-NORMALIZE`, etc. all match).
- **Rationale**: Custom extensions use the `X-` prefix convention, steering clear of any OpenAI-defined namespace. Header names are case-insensitive per RFC 9110, so casing enforcement would only create false rejections.
- **Alternatives considered**:
  - A body field (`normalize` in the request JSON) — rejected: the OpenAI embeddings schema has no such field; strict OpenAI SDKs reject unknown body params client-side, and accepting it silently would itself be a contract deviation with worse discoverability.
  - A query parameter — rejected: leaks into URLs/logs, duplicates the retired `?model=` antipattern this service already removed (spec 005), and is non-idiomatic for body-level options.
  - No `X-` prefix (e.g. `Normalize`) — rejected: risks colliding with a future OpenAI-standard header of the same name.

## Decision: Value grammar and strictness

- **Decision**: Only the strings `true` and `false`, compared case-insensitively. Every other value — empty, `1`, `0`, `yes`, `on` — is rejected with `400 invalid_request` naming the header and accepted values.
- **Rationale**: Strictness makes errors actionable and keeps the surface minimal; boolean text is unambiguous and maps 1:1 to the TEI body field introduced in 008, so both mechanisms share one grammar. Numeric or truthy aliases add parsing branches with no demonstrated client need.
- **Alternatives considered**:
  - Accepting `1`/`0` and other truthy spellings — rejected for v1: more parsing surface, weaker error messages; can be added later without breaking anything.
  - Silently ignoring unrecognized values — rejected: a silent no-op is the hardest failure mode to debug and would violate the spec's US2.

## Decision: Validation order relative to body errors

- **Decision**: Body validation runs first; header validation second. A request that is invalid in both respects gets the *body* error. Malformed JSON (which fails at body extraction before handler logic) also takes natural precedence.
- **Rationale**: Matches the spec Edge Case ("existing body validation errors take precedence with their current status codes and messages") and keeps existing error behavior fully unchanged for requests without the header — the regression-free path (SC-001) stays byte-identical.
- **Alternatives considered**:
  - Header validation first — rejected: changes observable error precedence for combined-invalid requests and contradicts the spec's edge case.

## Decision: Duplicate header handling

- **Decision**: More than one `X-Normalize` value in a single request is rejected with `400 invalid_request`.
- **Rationale**: Two conflicting booleans are ambiguous input; the framework exposes all values for a header name, so detecting duplicates is trivial and mirrors the strictness decision above. "First value wins" would make behavior depend on proxy ordering.
- **Alternatives considered**: First-value-wins — rejected: order-dependent, hides misconfiguration.

## Decision: Effect scope — OpenAI endpoint only

- **Decision**: The header is read only by `POST /v1/embeddings`. TEI endpoints ignore it entirely; their normalization mechanism remains the 008 body field.
- **Rationale**: The feature exists to give *OpenAI-compatible* clients a contract-safe channel; TEI clients already have the idiomatic body field, and reading the header in both places would create two overlapping mechanisms on one endpoint family.
- **Alternatives considered**: Reading the header on TEI endpoints as a fallback — rejected: scope creep with real ambiguity questions (which wins: header or body?); revisit only if clients ask.

## Decision: Documentation surface for the deviation

- **Decision**: The header is documented in (1) the OpenAI API docs page with an explicit "service-specific extension" note, (2) the interactive OpenAPI docs as a header parameter on the embeddings path, (3) an API contract document under this feature's `contracts/`, and (4) release notes at merge time.
- **Rationale**: The project's API conformity principle allows deviations only when explicitly documented and user-visible; four linked surfaces make the exception auditable and discoverable (US3).
- **Alternatives considered**: Documenting only in the OpenAPI schema — rejected: insufficient for the governance requirement and less discoverable than a dedicated docs section.
