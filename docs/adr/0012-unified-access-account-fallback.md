# Unified Access Account Fallback

`goog` will resolve targeted Google resources through Unified Access: when no explicit `--account` is provided, commands first try any remembered Resource Account Mapping, then the Active Account, then the remaining logged-in Accounts in config list order.
This fallback applies to both read and write commands targeting existing resources, runs silently on success, updates the mapping when an Account succeeds, and repairs stale mappings when they fail.
Automatic fallback also performs Incremental Authorization for candidate Accounts that are missing the command's required Scope, because the product goal is for users to log in once and use accessible resources without manual account switching.

Explicit `--account` remains strict and disables Account Fallback.
