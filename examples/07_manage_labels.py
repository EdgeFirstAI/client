# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Manage label indices using Coffee Cup as a read-only reference.

Coffee Cup (ds-145f) has non-contiguous label_index values on SaaS.
This example reads those indices, then reproduces them on an ephemeral
sandbox dataset — it does not modify ds-145f.

CLI reference:
  edgefirst-client dataset ds-145f --labels
"""

import os
import random
import string
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from examples import (  # noqa: E402
    COFFEE_CUP_DATASET_ID,
    EXAMPLES_PROJECT_NAME,
    get_client,
    resolve_project,
)


def main() -> None:
    client = get_client()
    client.verify_token()

    source = client.dataset(COFFEE_CUP_DATASET_ID)
    source_labels = client.labels(source.id)
    if not source_labels:
        raise RuntimeError("Coffee Cup has no labels")

    print(f"Coffee Cup labels ({len(source_labels)}) — sample indices:")
    for label in source_labels[:8]:
        print(f"  index={label.index} name={label.name!r}")
    indices = sorted({int(label.index) for label in source_labels})
    if indices != list(range(len(indices))):
        print("  (non-contiguous indices — typical for Coffee Cup on SaaS)")
    print()

    project = resolve_project(client, EXAMPLES_PROJECT_NAME or None)
    suffix = "".join(random.choices(string.ascii_uppercase + string.digits, k=6))
    dataset_name = f"Example Labels {suffix}"
    dataset_id = client.create_dataset(
        str(project.id),
        dataset_name,
        "DE-2762 examples: label index sandbox",
    )
    print(f"Sandbox dataset: {dataset_id}")

    # Copy a subset of label names with source-faithful indices
    subset = source_labels[: min(5, len(source_labels))]
    names = [label.name for label in subset]
    indices_to_set = [int(label.index) for label in subset]

    client.add_labels(dataset_id, names, indices_to_set)
    time.sleep(1)

    created = client.labels(dataset_id)
    by_name = {label.name: int(label.index) for label in created}
    for name, expected in zip(names, indices_to_set):
        actual = by_name.get(name)
        assert actual == expected, (
            f"Label {name!r}: expected index {expected}, got {actual}"
        )
        print(f"  OK {name!r} → index {actual}")

    # Demonstrate set_index on one label
    if created:
        target = created[0]
        new_index = 9000 + int(target.index)
        target.set_index(client, new_index)
        client.update_label(target)
        refreshed = client.labels(dataset_id)
        updated = next(lbl for lbl in refreshed if lbl.name == target.name)
        assert int(updated.index) == new_index
        print(f"\nUpdated {target.name!r} index → {new_index}")

    skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
    if skip_cleanup:
        print("SKIP_CLEANUP=1 — dataset left on server:", dataset_id)
    else:
        client.delete_dataset(dataset_id)
        print("Cleaned up sandbox dataset:", dataset_id)


if __name__ == "__main__":
    main()
