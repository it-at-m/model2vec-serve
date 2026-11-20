# Feature Specification: OpenAI Normalize Header

**Feature Branch**: `009-openai-normalize-header`

**Created**: 2026-11-20

**Status**: Draft

**Input**: User description: "normalization via header on openai api like
discussed. move to a new branch." — OpenAI-compatible clients gain per-request
normalization control through a request header, keeping the OpenAI request and
response JSON contract unchanged (follow-up to the 008-model-config-parity
discussion of PR #174).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Per-Request Normalization on `/v1/embeddings` (Priority: P1)

As an OpenAI-compatible client, I want to control whether embeddings from
`/v1/embeddings` are L2-normalized per request by sending an optional request
header, so that I can choose unit-length or raw mean-pooled vectors without
changing the request body or switching to the TEI endpoints.

**Why this priority**: This is the feature's core value — it closes the gap
where OpenAI-compatible clients had no portable way to request raw (or forced
normalized) embeddings, while keeping the documented JSON contract untouched.

**Independent Test**: Send the same `/v1/embeddings` request twice, once with
the normalization header set to "off" and once to "on", and verify the
returned vectors differ in length (norm) as expected while the response JSON
shape is identical.

**Acceptance Scenarios**:

1. **Given** a normal `/v1/embeddings` request without the header, **When** embeddings are produced, **Then** the response is byte-identical in shape and values to the pre-change service (model's configured default normalization).
2. **Given** a request with the header set to "on", **When** embeddings are produced, **Then** each returned vector has a length (norm) of approximately 1.0 regardless of the model's configured default.
3. **Given** a request with the header set to "off", **When** embeddings are produced, **Then** each returned vector is the raw mean-pooled embedding and its norm reflects the model's pooling output, even when the model's default is normalized.
4. **Given** a `base64` `encoding_format` request with the header set, **When** embeddings are produced, **Then** the base64 payload encodes the correspondingly normalized or unnormalized vector.

### User Story 2 - Invalid Header Values Fail Clearly (Priority: P2)

As a client integrating against the service, I want an invalid normalization
header value to produce the service's standard JSON error response, so that I
get an actionable message instead of silently ignored input.

**Why this priority**: Protects clients from a silent no-op that is hard to
debug; standard error handling keeps the API predictable.

**Independent Test**: Send `/v1/embeddings` with the header set to a value
that is neither "on" nor "off" and verify a `400` response with the standard
error body shape.

**Acceptance Scenarios**:

1. **Given** a request whose normalization header contains an unrecognized value, **When** the request is processed, **Then** the response status is `400` with the standard error body (`error` code and human-readable `message`).
2. **Given** a request with an empty normalization header value, **When** the request is processed, **Then** the response status is `400` with the standard error body.
3. **Given** a request with a valid header value in unusual letter case (e.g. mixed upper/lower case), **When** the request is processed, **Then** it is accepted and behaves like the canonical spelling.

### User Story 3 - Documented, Discoverable Extension (Priority: P3)

As a developer evaluating the service, I want the normalization header
documented in the API documentation and interactive OpenAPI docs, so that I
can discover and use it without reading the source code.

**Why this priority**: Documentation completes the feature but does not block
its value; without it the capability is undiscoverable.

**Independent Test**: Consult the API documentation and the interactive docs
page and verify the header is described with its accepted values, default
behavior, and an example.

**Acceptance Scenarios**:

1. **Given** the OpenAI API documentation page, **When** it is read, **Then** the header name, accepted values, absent-header default, and an example request are documented.
2. **Given** the interactive API documentation (`/docs`), **When** the embeddings endpoint is inspected, **Then** the header appears as a documented parameter with a description.

### Edge Cases

- The header is sent to endpoints other than `/v1/embeddings` (e.g. `/embed`, `/health`): unknown headers are ignored there; the header has effect only on the OpenAI embeddings endpoint.
- The header is present but the request body is otherwise invalid (empty input, oversized batch): the existing body validation errors take precedence with their current status codes and messages.
- The model's configured default is already "off": a request with "off" produces the same result as a request without the header for that model.
- Multiple normalization headers in one request: treated as invalid input (`400`), mirroring ambiguous-header handling elsewhere.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `POST /v1/embeddings` MUST accept an optional request header named `X-Normalize` with accepted values `true` and `false` (case-insensitive).
- **FR-002**: An absent `X-Normalize` header MUST keep the model's configured normalization default (behavior identical to the pre-change service).
- **FR-003**: `X-Normalize: true` MUST force L2-normalized embeddings for that request; `X-Normalize: false` MUST return the raw mean-pooled embeddings for that request, overriding the model default either way.
- **FR-004**: A request with an `X-Normalize` value other than a recognized boolean MUST be rejected with status `400` and the standard error body (`{"error": "code", "message": "..."}`), without generating embeddings.
- **FR-005**: The `/v1/embeddings` response shape MUST remain exactly the OpenAI embeddings response (`object`, `data`, `model`, `usage`) for all header states; the header MUST NOT appear in responses.
- **FR-006**: The header MUST have no effect on endpoints other than `/v1/embeddings` (the TEI body field remains the normalization mechanism there).
- **FR-007**: The header MUST be documented in the API documentation and appear as a documented parameter in the interactive OpenAPI docs, marked as a service-specific extension beyond the OpenAI contract.
- **FR-008**: The extension MUST be explicitly documented as a deviation exception in the API contract documentation, satisfying the project's API conformity governance.

### Key Entities *(include if data involved)*

- **Embedding Request Headers**: an optional `X-Normalize` value attached by the client to an OpenAI embeddings request; three states (absent, "on", "off") plus invalid; no persistence.
- **Embedding Response**: unchanged OpenAI embeddings response; the requested normalization affects only vector values, not shape or metadata.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Requests without the header produce responses identical to the pre-change service (100% of automated regression checks pass unchanged).
- **SC-002**: For the same input, a request with the header "on" yields vectors of norm ≈ 1.0 and a request with the header "off" yields a different norm, verified on at least one validation model.
- **SC-003**: 100% of invalid header values tested (unrecognized strings, empty value, duplicated header) return `400` with the standard error body and no embeddings.
- **SC-004**: The header is documented in both the API documentation page and the interactive docs, with zero open documentation gaps found in review.

## Assumptions

- The header name `X-Normalize` is the final choice (custom-header convention; avoids any OpenAI-defined namespace).
- Values are the strings `true` and `false`, compared case-insensitively per HTTP header semantics; no numeric aliases (`1`/`0`) in v1 to keep validation strict and error messages clear.
- Scope is limited to `POST /v1/embeddings`; the TEI endpoints keep their body-field mechanism (feature 008) and gain no header support in v1.
- The OpenAI JSON request/response contract remains unchanged; the header is the only extension, documented per the project's API conformity rules.
- Requires feature 008-model-config-parity (PR #174) to be merged first, as it introduces the per-request normalization plumbing and model metadata this feature builds on.
