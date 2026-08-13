# Unified Access Account Fallback

`goog` will resolve targeted Google resources through a shared Unified Access module: when no explicit `--account` is provided, commands first try any remembered Resource Account Mapping, then the Active Account, then the remaining logged-in Accounts in config list order.
This fallback applies to every command surface that can provide a target resource identity and an account-scoped access attempt, runs silently on success, updates the mapping when an Account succeeds, and repairs stale mappings when they fail.
Automatic fallback never opens Incremental Authorization for candidate Accounts that are missing the command's required Scope.
The Account must be authorized through the explicit full-scope `goog auth login` flow before it can participate in Unified Access for that service.
Resource Account Mappings are runtime state and live outside the setup config, because they evolve automatically while the config remains user setup data.
Mail direct-message commands can use Unified Access for message IDs, but mailbox list and search commands remain scoped to the Active Account or explicit `--account` because they do not target one known resource.

Explicit `--account` remains strict and disables Account Fallback.
Successful explicit `--account` commands still update Resource Account Mappings for later default invocations.
When multiple Accounts can access the same target, the first successful candidate wins.
Mappings for removed Accounts are ignored and can be pruned opportunistically.
The shared module runs the command's real account-scoped API attempt rather than performing a separate probe.
