## Context

The current pairing implementation uses the application-provided challenge correctly at the protocol level, but the wallet terminal also prints the challenge value before asking the user to enter it. That duplicates the source of truth and weakens the intended human verification step.

The better ceremony is:
- the application shows the challenge to the user
- the application sends the same challenge in the pairing request
- the wallet asks the user to enter the challenge they see in the application
- the wallet validates the entered value against the request payload without echoing the challenge in the wallet UI

This is a small UX refinement, but it affects both the wallet prompt copy and the example application's reference role.

## Goals / Non-Goals

**Goals:**
- Remove redundant challenge display from the wallet terminal pairing prompt.
- Preserve the existing application-provided challenge protocol.
- Make the example app clearly present the challenge as the value the user must enter into the wallet.
- Keep the change narrowly scoped to pairing UX.

**Non-Goals:**
- Changing the wire protocol.
- Changing session structure or approval rules.
- Redesigning the broader pairing flow.
- Adding new capabilities beyond the current pairing and reference app scope.

## Decisions

### 1. The web application is the user-visible source of truth for the challenge
The challenge remains part of the pairing request, but the wallet prompt no longer displays it back to the user. Instead, the wallet prompt asks the user to enter the challenge shown in the application.

**Rationale:** this restores the intended verification ceremony and reduces redundant or potentially confusing terminal output.

### 2. The wallet still validates against the request challenge directly
The internal validation behavior remains unchanged: the wallet compares the entered value with the challenge carried in the request.

**Rationale:** the protocol and safety property already exist; only the user-facing prompt needs refinement.

### 3. The example app should explicitly frame the challenge as the wallet-entry value
The example app UI and text should make it obvious that the challenge shown in the browser is the value the user must paste or type into the wallet prompt.

**Rationale:** the example app is an integration reference and should demonstrate the intended pairing ceremony clearly.

## Risks / Trade-offs

- **[Removing the terminal echo could make debugging slightly less convenient]** → Mitigation: keep the prompt wording explicit about using the application-displayed challenge.
- **[The example app could still under-communicate the interaction if its copy is vague]** → Mitigation: update the UI copy so the challenge is clearly presented as the source of truth.

## Migration Plan

1. Update the wallet pairing prompt copy.
2. Remove challenge echoing from the wallet-side pairing request log.
3. Update the example app UI text to present the challenge as the value to enter in the wallet.
4. Update any affected docs or tests.

## Open Questions

None currently.
