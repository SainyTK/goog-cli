# Docs Starts With Raw Get and Batch Update

The first Google Docs surface exposes `goog docs get` for raw document retrieval and `goog docs batch-update` for direct `documents.batchUpdate` writes. This keeps Docs focused on document content while Drive remains responsible for discovery and file lifecycle, and it avoids hiding Google Docs' index, tab, style, and structural mutation rules behind premature convenience commands.

`batch-update --requests` accepts the full Google request body, not just the `requests` array, so users can pass native fields such as `writeControl` without learning a `goog`-specific payload shape.

`docs get` exposes Google's `includeTabsContent` parameter as `--include-tabs-content` from the first slice, keeping tab-aware content available while avoiding a higher-level tab abstraction.

Docs commands emit JSON implicitly because raw document structures and batch-update replies do not have a useful table representation. `docs get` writes only to stdout, and `batch-update` reads its request body only from `--requests <path|->`.
