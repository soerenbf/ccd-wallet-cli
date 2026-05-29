## MODIFIED Requirements

### Requirement: Deploy validation checks whether the module already exists on chain
When `validate: true` is set on a deploy request, the wallet SHALL derive the module reference from `moduleHex` and check whether that module already exists on chain before prompting for approval.

If the module is confirmed to already exist on chain, the wallet SHALL show that result as a warning in the approval prompt, using the message `Validation warning: module already exists on chain for this network.`, and SHALL still allow the user to approve or decline. If the validation check cannot be completed because of node unavailability or another transient failure, the wallet SHALL show that result as a warning in the approval prompt and SHALL still allow the user to approve or decline.

#### Scenario: Validation succeeds and module is not already deployed
- **WHEN** a browser sends `requestDeployModule` with `validate: true`
- **AND** the wallet confirms that the derived module reference is not already present on chain
- **THEN** the wallet proceeds to the approval prompt

#### Scenario: Validation finds an existing module on chain
- **WHEN** a browser sends `requestDeployModule` with `validate: true`
- **AND** the wallet confirms that the derived module reference already exists on chain
- **THEN** the wallet shows that finding as a warning in the approval prompt
- **AND** uses the warning message `Validation warning: module already exists on chain for this network.`
- **AND** still lets the user choose whether to proceed

#### Scenario: Validation cannot complete due to node failure
- **WHEN** a browser sends `requestDeployModule` with `validate: true`
- **AND** the wallet cannot complete the existence check because the node is unavailable
- **THEN** the wallet shows the validation failure as a warning in the approval prompt
- **AND** still lets the user choose whether to proceed

### Requirement: Wallet approval prompt shows deploy-specific review details
The wallet approval prompt for `requestDeployModule` SHALL display:
- the browser origin
- the session-bound network
- the session-bound account address
- the derived module reference
- the module size in bytes
- any validation warning or duplicate-module finding

The approval prompt SHALL NOT display a second hash summary beyond the derived module reference.

#### Scenario: Approval prompt shows derived module details
- **WHEN** the wallet presents a deploy-module approval prompt
- **THEN** the prompt includes the derived module reference
- **AND** includes the module size in bytes
