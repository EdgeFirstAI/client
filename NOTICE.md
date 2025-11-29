# NOTICE

## EdgeFirst Client

**Copyright 2025 Au-Zone Technologies**

This product includes software developed at Au-Zone Technologies ([https://au-zone.com/](https://au-zone.com/)).

This product is licensed under the Apache License, Version 2.0 (see [LICENSE](LICENSE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0)).

---

## Third-Party Software Notices and Information

This project incorporates components from third-party open source projects. The original copyright notices and the licenses under which we received such components are set forth below and in the Software Bill of Materials (SBOM).

### Complete List of Dependencies

For a complete and authoritative list of third-party dependencies, their licenses, and attribution information, see:

- **Release artifacts**: [https://github.com/EdgeFirstAI/client/releases](https://github.com/EdgeFirstAI/client/releases)
  - Download `sbom.json` from the latest release assets for the complete SBOM in CycloneDX format
  - The SBOM includes all dependencies, their versions, and license information

- **Generate locally**:

  ```bash
  make sbom
  ```

  This generates `sbom.json` in CycloneDX 1.6 format with complete dependency information.

**License Compliance**: All third-party dependencies are compatible with Apache-2.0. For detailed license information and compliance verification, consult the generated SBOM file.

---

## Apache License Compliance

This NOTICE file satisfies the requirements of the Apache License, Version 2.0, Section 4(d):

> "If the Work includes a 'NOTICE' text file as part of its distribution, then any Derivative Works that You distribute must include a readable copy of the attribution notices contained within such NOTICE file..."
