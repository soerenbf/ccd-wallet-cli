## MODIFIED Requirements

### Requirement: Pairing uses a richer confirmation ceremony
The wallet SHALL require a pairing ceremony stronger than an origin-only approval. The browser pairing flow SHALL include an application-provided shared confirmation challenge or pairing code that is visible in the browser and validated by the wallet during approval.

The wallet approval UX SHALL prompt the user to enter the challenge shown in the calling application. The wallet SHALL validate the entered value against the pairing request challenge, but SHALL NOT redundantly display the challenge value itself in the wallet terminal during approval.

#### Scenario: Pairing approval prompts for the application-displayed challenge
- **WHEN** a browser dApp requests pairing
- **THEN** the wallet prompts the user to enter the challenge shown in the browser application
- **AND** validates the entered value against the challenge supplied in the pairing request
- **AND** does not display the challenge value itself in the wallet approval prompt
