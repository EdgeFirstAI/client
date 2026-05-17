#!/bin/bash
# Generate complete SBOM using scancode-toolkit
#
# This script implements SBOM generation per the Au-Zone Software Process Specification.
# Last synchronized with policy version: 2.0 (2025-11-22)
#
# This script generates a comprehensive Software Bill of Materials (SBOM)
# in CycloneDX format by:
#   1. Scanning source code directories with scancode
#   2. Scanning package manifests for dependencies
#   3. Merging all SBOMs into a single file
#   4. Validating license policy compliance
#   5. Validating NOTICE file (if present)
#
# CUSTOMIZE FOR YOUR PROJECT:
# - Update PROJECT_NAME and PROJECT_TYPE below
# - Update SOURCE_DIRS list for your source directories
# - Update VERSION_FILE location (single source of truth for version)
# - Update MANIFEST_FILES for your package managers
# - For C/C++ projects: Add system dependencies in Step 3
# - For multi-language: Adjust language-specific sections

set -e  # Exit on error

# ===========================================================================
# PROJECT CONFIGURATION - CUSTOMIZE THESE
# ===========================================================================

PROJECT_NAME="edgefirst-client"
PROJECT_TYPE="library"  # Options: library, application, framework
VERSION_FILE="Cargo.toml"  # Single source of truth for version

# Source directories to scan (space-separated)
SOURCE_DIRS="crates examples"

# Package manifest files (for dependency parsing)
MANIFEST_FILES="Cargo.toml Cargo.lock requirements.txt"

# ===========================================================================
# SBOM GENERATION
# ===========================================================================

echo "=================================================="
echo "Generating Complete SBOM for $PROJECT_NAME"
echo "=================================================="
echo

# Extract version from Cargo.toml workspace version
if [ -f "$VERSION_FILE" ]; then
    VERSION=$(grep -m 1 '^version = ' "$VERSION_FILE" | sed 's/version = "\(.*\)"/\1/')
    if [ -z "$VERSION" ]; then
        echo "❌ Could not extract version from $VERSION_FILE"
        exit 1
    fi
    echo "Detected version: $VERSION"
else
    VERSION="unknown"
    echo "Warning: VERSION file not found, using version: $VERSION"
fi
echo

# Step 1: Generate source code SBOM with scancode
echo "[1/6] Generating source code SBOM with scancode..."
if [ ! -f "venv/bin/scancode" ]; then
    echo "Error: scancode not found. Please install:"
    echo "  python3 -m venv venv"
    echo "  venv/bin/pip install scancode-toolkit"
    exit 1
fi

# Scan each source directory separately (MUCH faster than scanning all at once)
SBOM_FILES=""
for dir in $SOURCE_DIRS; do
    if [ -d "$dir" ]; then
        echo "  Scanning $dir/..."
        OUTPUT_FILE="source-sbom-$(basename $dir).json"
        venv/bin/scancode -clpieu \
            --cyclonedx "$OUTPUT_FILE" \
            --only-findings \
            --timeout 300 \
            "$dir/"
        SBOM_FILES="$SBOM_FILES $OUTPUT_FILE"
    fi
done

# Scan manifest files
for file in $MANIFEST_FILES; do
    if [ -f "$file" ]; then
        echo "  Scanning $file..."
        OUTPUT_FILE="source-sbom-$(basename $file .txt | tr '.' '-').json"
        venv/bin/scancode -clpieu \
            --cyclonedx "$OUTPUT_FILE" \
            --only-findings \
            --timeout 300 \
            "$file"
        SBOM_FILES="$SBOM_FILES $OUTPUT_FILE"
    fi
done

echo "✓ Generated individual SBOM files"
echo

# Step 2: Merge and clean source SBOMs
echo "[2/6] Merging and cleaning source SBOMs..."

export VERSION PROJECT_NAME PROJECT_TYPE SBOM_FILES
python3 << 'EOF'
import json
import sys
import os

VERSION = os.environ['VERSION']
PROJECT_NAME = os.environ['PROJECT_NAME']
PROJECT_TYPE = os.environ['PROJECT_TYPE']

def load_sbom(filename):
    """Load an SBOM file if it exists"""
    if not os.path.exists(filename):
        return None
    with open(filename, 'r') as f:
        return json.load(f)

def clean_sbom_properties(sbom):
    """Remove problematic metadata that violates CycloneDX spec"""
    if 'metadata' in sbom and 'properties' in sbom['metadata']:
        sbom['metadata']['properties'] = [
            p for p in sbom['metadata']['properties']
            if isinstance(p.get('value'), str)
        ]
    return sbom

# Load all individual SBOMs
sbom_files = os.environ['SBOM_FILES'].split()
all_components = []

for filename in sbom_files:
    sbom = load_sbom(filename)
    if not sbom:
        continue

    # Clean the SBOM
    sbom = clean_sbom_properties(sbom)

    # Extract components, filtering out the main project component
    if 'components' in sbom:
        for component in sbom['components']:
            # Skip main project component from scancode - we define it in metadata
            if component.get('name') == PROJECT_NAME.lower():
                continue
            all_components.append(component)

# Create merged source SBOM
merged_sbom = {
    'bomFormat': 'CycloneDX',
    'specVersion': '1.6',
    'version': 1,
    'metadata': {
        'component': {
            'type': PROJECT_TYPE,
            'name': PROJECT_NAME,
            'version': VERSION,
            'licenses': [
                {'license': {'id': 'Apache-2.0'}}
            ]
        }
    },
    'components': all_components
}

# Save merged version
with open('source-sbom.json', 'w') as f:
    json.dump(merged_sbom, f, indent=2)

print(f"Merged {len(sbom_files)} source SBOMs into source-sbom.json")
print(f"Total components: {len(all_components)}")
sys.exit(0)
EOF

echo "✓ Generated source-sbom.json (merged and cleaned)"
echo

# Step 3: Generate dependency SBOM (language-specific)
echo "[3/6] Generating dependency SBOM..."

# For Rust projects with cargo-cyclonedx
if [ -f "Cargo.toml" ] && command -v cargo-cyclonedx &> /dev/null; then
    echo "  Generating Rust dependencies with cargo-cyclonedx..."
    # cargo-cyclonedx 0.5+ emits one SBOM per workspace member at
    # ``crates/<name>/<name>.cdx.json``. We merge them into a single
    # deps-sbom.json with python (deduplicating by ``purl`` so a
    # transitive dependency reachable from multiple workspace members
    # only appears once). Without this merge the script fell back to
    # the empty stub below, and inherited license declarations
    # (``license.workspace = true``) never reached the final SBOM —
    # which surfaced as spurious "unknown license" warnings on every
    # workspace crate.
    cargo cyclonedx --format json --quiet
    python3 << 'EOF'
import glob
import json
import os
import sys

per_crate = sorted(glob.glob('crates/*/*.cdx.json'))
if not per_crate:
    sys.exit(0)

components_by_key = {}
workspace_members = set()
for path in per_crate:
    with open(path) as f:
        sbom = json.load(f)
    # The main component of each per-crate SBOM is the workspace
    # member itself — include it (those are the entries that carry
    # the inherited Apache-2.0 license).
    main = sbom.get('metadata', {}).get('component')
    if main and main.get('name'):
        key = main.get('purl') or f"{main['name']}@{main.get('version','')}"
        components_by_key.setdefault(key, main)
        workspace_members.add(main['name'])
    for comp in sbom.get('components', []):
        key = comp.get('purl') or f"{comp.get('name','')}@{comp.get('version','')}"
        components_by_key.setdefault(key, comp)

with open('deps-sbom.json', 'w') as f:
    json.dump({
        'bomFormat': 'CycloneDX',
        'specVersion': '1.6',
        'version': 1,
        'components': list(components_by_key.values()),
    }, f, indent=2)

# Clean up per-crate intermediates so they don't pollute git.
for path in per_crate:
    os.remove(path)
print(f"Merged {len(per_crate)} per-crate SBOMs "
      f"({len(workspace_members)} workspace members, "
      f"{len(components_by_key)} unique components)")
EOF
fi

# For Python projects (scancode already parsed requirements.txt/pyproject.toml)
# Dependencies are included in source-sbom.json

# For C/C++ projects - manually define system dependencies
if [ ! -f "deps-sbom.json" ]; then
    echo "  Creating empty deps-sbom.json (customize for system dependencies)..."
    cat > deps-sbom.json << 'EOFPYTHON'
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.6",
  "version": 1,
  "components": []
}
EOFPYTHON
fi

echo "✓ Generated deps-sbom.json"
echo

# Step 4: Merge SBOMs using cyclonedx-cli
echo "[4/6] Merging source and dependency SBOMs..."
if ! command -v cyclonedx &> /dev/null; then
    if ! command -v ~/.local/bin/cyclonedx &> /dev/null; then
        echo "Error: cyclonedx CLI not found. Please install from https://github.com/CycloneDX/cyclonedx-cli"
        exit 1
    fi
    CYCLONEDX=~/.local/bin/cyclonedx
else
    CYCLONEDX=cyclonedx
fi

$CYCLONEDX merge \
    --input-files source-sbom.json deps-sbom.json \
    --output-file sbom-temp.json

# Post-merge cleanup:
#  1. Drop the duplicate metadata-vs-component entry for the project
#     itself (the workspace package shows up in both places).
#  2. Drop "versionless / unlicensed" scancode placeholders for any
#     component that also appears with a version and license from
#     cargo-cyclonedx. Scancode walks the manifest tree and emits
#     bare ``pkg:cargo/<name>`` rows that don't resolve workspace-
#     inherited license fields; without dedup they pollute the
#     compliance report as "unknown license" duplicates of entries
#     that DO carry the inherited Apache-2.0.
export PROJECT_NAME
python3 << 'EOF'
import json
import os

PROJECT_NAME = os.environ['PROJECT_NAME']

with open('sbom-temp.json', 'r') as f:
    sbom = json.load(f)

components = sbom.get('components', [])

# Map name -> True if any entry for that name has a version (the
# versioned entry is the authoritative one from cargo-cyclonedx).
has_versioned = {}
for c in components:
    name = c.get('name')
    if name and c.get('version'):
        has_versioned[name] = True

def keep(c):
    name = c.get('name')
    # Drop the project's own metadata-mirror entry.
    if name == PROJECT_NAME.lower():
        return False
    # Drop versionless placeholders that have a versioned twin.
    if not c.get('version') and has_versioned.get(name):
        return False
    return True

sbom['components'] = [c for c in components if keep(c)]

with open('sbom.json', 'w') as f:
    json.dump(sbom, f, indent=2)
EOF

rm -f sbom-temp.json
echo "✓ Generated sbom.json (merged: source + dependencies)"
echo

# Step 5: Check license policy
echo "[5/6] Checking license policy compliance..."
if [ -f ".github/scripts/check_license_policy.py" ]; then
    python3 .github/scripts/check_license_policy.py sbom.json
    POLICY_EXIT=$?
else
    echo "Warning: License policy checker not found, skipping..."
    POLICY_EXIT=0
fi
echo

# Step 6: Validate NOTICE file
echo "[6/6] Validating NOTICE file..."
if [ -f "NOTICE" ] && [ -f ".github/scripts/validate_notice.py" ]; then
    python3 .github/scripts/validate_notice.py NOTICE sbom.json
    NOTICE_EXIT=$?
    if [ $NOTICE_EXIT -ne 0 ]; then
        echo "⚠️  NOTICE file validation failed - please update NOTICE manually"
    else
        echo "✓ NOTICE file validated (matches first-level dependencies)"
    fi
else
    echo "Skipping NOTICE validation (file or validator not found)"
    NOTICE_EXIT=0
fi
echo

# Cleanup temporary files
rm -f $SBOM_FILES source-sbom.json deps-sbom.json

echo "=================================================="
echo "SBOM Generation Complete"
echo "=================================================="
echo "Files generated:"
echo "  - sbom.json (merged SBOM)"
echo
echo "Files validated:"
echo "  - NOTICE (third-party attributions)"
echo

# Exit with error if either check failed
if [ $POLICY_EXIT -ne 0 ] || [ $NOTICE_EXIT -ne 0 ]; then
    exit 1
fi
exit 0
