# Security Policy

## Overview

EdgeFirst Client is a REST API client library and command-line tool for communicating with the EdgeFirst Studio platform. While the primary security controls are implemented server-side by the EdgeFirst Studio platform, this client library follows security best practices for client-side operations.

## Supported Versions

We provide security updates for the following versions:

| Version | Supported          | End of Support |
| ------- | ------------------ | -------------- |
| 2.x.x   | ✅ Yes             | Current        |
| 1.x.x   | ⚠️ Security fixes only | 2026-01-01     |
| < 1.0   | ❌ No              | Ended          |

We recommend always using the latest stable release to receive security updates and improvements.

## Security Considerations for Client Applications

### Authentication & Credentials

**What EdgeFirst Client Does:**
- Transmits credentials securely over HTTPS/TLS
- **Does NOT store usernames or passwords** - only session tokens are persisted
- Stores session tokens in standard user configuration directories:
  - Linux/macOS: `~/.config/EdgeFirst Studio/token`
  - Windows: `%APPDATA%\EdgeFirst\EdgeFirst Studio\config\token`
- Session tokens are time-limited and expire automatically
- Session tokens are refreshed by the client when used if the token is still valid
- Never logs credentials or authentication tokens to console/files
- Supports environment variables (`STUDIO_TOKEN`, `STUDIO_USERNAME`, `STUDIO_PASSWORD`) for CI/CD

> **Note**: Session tokens are currently stored as plaintext files. Future versions will use OS-native secure storage (keyring).

**Your Responsibilities:**
- **Protect your credentials**: Never commit credentials or tokens to version control
- **Use environment variables**: Preferred for automation and CI/CD workflows
- **Rotate credentials regularly**: Follow your organization's security policies
- **Secure token files**: Ensure proper file permissions on configuration directory
- **Revoke compromised tokens**: Use `logout` command to delete the local token file

### Network Security

**What EdgeFirst Client Does:**
- Enforces HTTPS/TLS for all API communications (hardcoded `https://` URLs)
- Uses system default TLS certificate validation via `reqwest` with `rustls-tls` backend
- Uses `rustls-native-certs` for system certificate store integration
- Connects only to `*.edgefirst.studio` domains

**Your Responsibilities:**
- **Be aware of network risks**: Using client on untrusted/public networks exposes traffic metadata
- **DNS security**: Ensure DNS resolution is trustworthy (rogue DNS could redirect to malicious servers)
- **Keep client updated**: Updates include latest TLS libraries and security patches
- **Monitor for certificate warnings**: Report any unexpected TLS certificate errors immediately

> **Note**: Client uses default system certificate validation. There is no option to disable certificate checking.

### Data Handling

**What EdgeFirst Client Does:**
- Transmits all data over encrypted HTTPS connections
- Stores session tokens in user configuration directory (plaintext files)
- Does not persist user credentials (username/password)
- Does not log credentials or tokens in application logs
- Accepts and processes server responses as JSON-RPC format

**Current Limitations:**
- Session tokens stored as plaintext (mitigated by time-limited expiration)
- No specific response sanitization for injection attacks (server-side responsibility)
- Token cleanup on logout requires manual file deletion or `logout()` method call

**Your Responsibilities:**
- **Protect local data**: Secure any files downloaded or exported by the client
- **File permissions**: Ensure configuration directory has appropriate access controls
- **Secure log outputs**: CLI output may contain project names, dataset IDs, and other metadata
- **Clean up tokens**: Use `logout` command when done, especially on shared systems
- **Monitor token files**: Check `~/.config/EdgeFirst Studio/token` for unauthorized access

### Dependency Security

**What We Do:**
- Regularly audit dependencies using `cargo audit`
- Update dependencies to address known vulnerabilities
- Pin dependencies to specific versions in releases
- Run security scans via SonarCloud in CI/CD
- Generate and publish third-party license information (THIRD_PARTY.md)

**Staying Secure:**
- Subscribe to GitHub Security Advisories for this repository
- Update to latest versions promptly when security patches are released
- Review CHANGELOG.md for security-related updates

## Reporting a Vulnerability

If you discover a security vulnerability in EdgeFirst Client, please help us maintain the security of our users by reporting it responsibly.

### 🔒 Private Disclosure (Preferred)

**GitHub Security Advisories** (Recommended):
1. Go to [Security Advisories](https://github.com/EdgeFirstAI/client/security/advisories)
2. Click "Report a vulnerability"
3. Fill out the advisory form with details

This creates a private discussion with the maintainers.

**Email**:
If you prefer email or cannot use GitHub Security Advisories:
- **Email**: support@au-zone.com
- **Subject**: [SECURITY] EdgeFirst Client - [Brief Description]

### ⚠️ Please Do Not:
- Open public GitHub issues for security vulnerabilities
- Disclose the vulnerability publicly before we've had a chance to address it
- Exploit the vulnerability beyond what's necessary to demonstrate the issue

### What to Include

Please provide as much information as possible:

```
1. Type of vulnerability (e.g., credential exposure, dependency vulnerability, etc.)
2. Affected version(s)
3. Steps to reproduce
4. Potential impact
5. Suggested fix (if you have one)
6. Your contact information for follow-up
```

### Response Timeline

We take security seriously and will respond promptly:

| Timeframe | Action |
|-----------|--------|
| **48 hours** | Initial acknowledgment of your report |
| **7 days** | Assessment and validation of the vulnerability |
| **30 days** | Target for patch development and release |
| **Coordinated disclosure** | Public disclosure after fix is released |

Actual timelines may vary based on complexity and severity.

### Severity Classification

We follow the Common Vulnerability Scoring System (CVSS) to assess severity:

- **Critical** (9.0-10.0): Immediate action required
- **High** (7.0-8.9): Prioritized for next patch release
- **Medium** (4.0-6.9): Scheduled for upcoming release
- **Low** (0.1-3.9): Addressed in regular maintenance cycle

## Security Best Practices for Users

### For CLI Users

```bash
# ✅ BEST: Use interactive login (stores session token)
edgefirst-client login
# Prompts for username and password interactively
# Session token stored in ~/.config/EdgeFirst Studio/token

# Use the client with stored session token
edgefirst-client organization

# ⚠️ ACCEPTABLE: Environment variables (for CI/CD and isolated systems only)
# WARNING: Only use in isolated environments (CI/CD runners, containers, dedicated VMs)
# Risks: Environment variables are inherited by child processes and may be visible
# to other processes on the same system (especially to root/admin users)
export STUDIO_USERNAME="your-username"
export STUDIO_PASSWORD="your-password"
edgefirst-client organization

# Alternative: Use STUDIO_TOKEN directly (preferred for CI/CD)
export STUDIO_TOKEN="your-session-token"
edgefirst-client organization

# ❌ BAD: Hardcoding credentials in command line
# This leaves credentials in shell history
edgefirst-client --username user --password pass123 organization
```

### For Library Users (Rust)

```rust
// ✅ Good: Load from environment variables
use edgefirst_client::Client;
use std::env;

let client = Client::new()?;
let client = match (env::var("STUDIO_USERNAME"), env::var("STUDIO_PASSWORD")) {
    (Ok(username), Ok(password)) => client.with_login(&username, &password).await?,
    _ => client.with_token_path(None)?, // Load from ~/.config/EdgeFirst Studio/token
};

// ❌ Bad: Hardcoded credentials
let client = Client::new()?.with_login("user", "hardcoded_password").await?;
```

### For Library Users (Python)

```python
# ✅ Good: Use environment variables
import os
from edgefirst_client import Client

client = Client(
    username=os.getenv("STUDIO_USERNAME"),
    password=os.getenv("STUDIO_PASSWORD")
)

# ❌ Bad: Hardcoded credentials
client = Client(username="user", password="hardcoded_password")
```

## Server-Side Security

The EdgeFirst Studio platform implements comprehensive security controls including:
- Multi-factor authentication (MFA)
- Role-based access control (RBAC)
- Audit logging
- Rate limiting and DDoS protection
- Data encryption at rest and in transit
- Regular security audits and penetration testing

For questions about server-side security, contact: support@au-zone.com

## Security Updates

Security patches are released as soon as possible after verification. Updates are announced through:

- **GitHub Security Advisories**: Automatic notifications for repository watchers
- **Release Notes**: Documented in CHANGELOG.md with `[SECURITY]` prefix
- **GitHub Releases**: Tagged releases with security notes
- **Community Discussions**: Posted to EdgeFirstAI organization discussions

Subscribe to repository notifications to stay informed.

## Additional Resources

- **OWASP REST Security**: https://cheatsheetseries.owasp.org/cheatsheets/REST_Security_Cheat_Sheet.html
- **Rust Security Guidelines**: https://anssi-fr.github.io/rust-guide/
- **Cargo Audit**: https://github.com/RustSec/rustsec

## Credits

We appreciate the security research community's efforts in making EdgeFirst Client more secure. Responsible disclosure of vulnerabilities helps protect all users.

---

Thank you for helping keep EdgeFirst Client and our users secure!
