# Single OAuth App for All Accounts

One GCP project and one OAuth client ID/secret serves all Google Accounts that `goog` manages. Users run `goog auth setup` once to import these credentials, then `goog auth login` issues a separate Token per Account -- all through the same OAuth App.

## Considered Options

The alternative is a per-account model where each Google Account has its own GCP project and OAuth credentials. This was rejected because it forces users to create and maintain N GCP projects for N accounts, and provides no practical security benefit -- the OAuth App is not a secret (client IDs are public).

## Consequences

The Token store must track which Account each Token belongs to. Revoking the OAuth App (e.g., deleting the GCP project) invalidates Tokens for all Accounts simultaneously.
