# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Authentication and token management for EdgeFirst Studio.

CLI reference: see CLI.md sections login, logout, token.

Recommended workflow:
  1. edgefirst-client login          # prompts for username and password
  2. python examples/01_authentication.py

The Python Client() loads the same cached token the CLI stored.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from examples import get_client, token_storage_path  # noqa: E402

from edgefirst_client import FileTokenStorage  # noqa: E402


def main() -> None:
    print("Token file (CLI default):", token_storage_path())
    print()

    client = get_client()
    client.verify_token()

    print("Server URL:", client.url)
    print("Username:", client.username)
    print("Token expires:", client.token_expiration)
    print("Token length:", len(client.token()))
    print()

    # Optional: renew before expiry (no-op if not needed)
    try:
        client.renew_token()
        print("Token renewed successfully.")
    except Exception as exc:
        print("Token renew skipped or failed:", exc)

    storage = FileTokenStorage()
    print("FileTokenStorage path:", storage.path)

    print()
    print("Authentication OK. Run 02_explore_dataset.py next.")


if __name__ == "__main__":
    main()
