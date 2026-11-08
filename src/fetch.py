"""Use Vibe's credential lifecycle and pinned-origin HTTP client, never copy keys."""
import asyncio
import json

from vibe.core.config.harness_files import init_harness_files_manager
from vibe.core.platform_auth import MistralPlatformHTTPClient
from vibe.core.platform_auth._api_key import ApiKeyPlatformCredentialProvider
from vibe.core.platform_auth._credentials_port import PlatformCredentialRequired, PlatformCredentialTarget

BASE = "https://api.mistral.ai/v1/"


async def main():
    init_harness_files_manager("user")
    credentials = ApiKeyPlatformCredentialProvider("MISTRAL_API_KEY")
    target = PlatformCredentialTarget(base_url=BASE)
    snapshot = await credentials.resolve(target)
    if isinstance(snapshot, PlatformCredentialRequired):
        return {"status": "auth"}
    async with MistralPlatformHTTPClient(base_url=BASE, credentials=credentials, timeout=4.0) as client:
        budget, subscription = await asyncio.gather(client.get("billing/budget"), client.get("billing/subscription"))
    current = await credentials.resolve(target)
    if isinstance(current, PlatformCredentialRequired) or current.identity != snapshot.identity:
        return {"status": "auth"}
    if budget.status_code != 200:
        status = {401: "auth", 403: "forbidden", 404: "unsupported"}.get(budget.status_code, "unavailable")
        return {"status": status}
    if len(budget.content) > 65536:
        return {"status": "unavailable"}
    data = budget.json()
    fields = ("initial_budget", "usage_percentage", "currency", "reset_at", "payg_enabled")
    result = {"status": "ready"}
    for name in ("vibe_budget", "api_budget"):
        raw = data.get(name)
        if isinstance(raw, dict):
            result[name] = {key: raw.get(key) for key in fields}
    if subscription.status_code == 200 and len(subscription.content) <= 65536:
        result["plan"] = subscription.json().get("plan_name", "")
    return result


try:
    result = asyncio.run(main())
except Exception:
    result = {"status": "unavailable"}
print(json.dumps(result, allow_nan=False))
