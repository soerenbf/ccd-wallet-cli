## Why

The current pairing flow makes the wallet terminal both display and request the challenge code. That duplicates information and weakens the intended ceremony: the challenge should originate in the web application UI, then be entered into the wallet so the user proves they are looking at the same browser request.

Refining this now keeps the pairing UX clearer and makes the example app a better reference for real integrations.

## What Changes

- Change the wallet pairing UX so the wallet prompt asks the user to enter the challenge shown in the web application instead of showing the challenge value itself in the terminal.
- Keep the browser application responsible for displaying the challenge to the user.
- Update the example web application to present the challenge as the value the user must copy or paste into the wallet prompt.
- Preserve the existing protocol shape: the application still supplies the challenge in the pairing request and the wallet still validates the entered value against it.

## Capabilities

### New Capabilities

### Modified Capabilities
- `browser-wallet-pairing`: refine the pairing confirmation UX so the wallet prompts for the application-displayed challenge without redundantly showing it in the wallet UI
- `connect-example-web-app`: clarify the example app UI and documentation so the challenge is shown in the web application as the user-visible source of truth

## Impact

- Affected code: wallet connect command UX, pairing-related documentation, and the example web application UI copy.
- Affected systems: browser-wallet pairing ceremony and developer-facing integration guidance.
- User-facing behavior: the wallet terminal no longer displays the challenge value during approval; users instead enter the challenge they see in the web application.
