# Security Policy

Overview

VeilBox is a Windows client built with Wails (Go + React) that shells an external sing-box binary. This policy explains how to report security vulnerabilities and what to expect.
Scope

In scope: VeilBox source code, build scripts, installer configuration, and bundled app artifacts.
Out of scope: Upstream projects (e.g., sing-box, Wails, WebView2) — report those to their respective projects.
Hosted infrastructure is not part of this repo; do not test any non-local services.
Supported Versions

Security fixes are provided for:
main branch (active development)
latest released version
the previous minor release, when feasible
Older versions may receive best-effort guidance only.
How To Report

Preferred: GitHub Security Advisory (privately) for this repository.
Alternative: Email the maintainer with “SECURITY” in the subject. Include:
A clear description of the issue and impact
A minimal proof of concept (PoC) or reproduction steps
Affected version/commit, OS version, and configuration
Suggested CVSS vector (optional) and any mitigations
Do not open a public issue for security problems.
Response Targets

Acknowledgement: within 3 business days
Triage & initial assessment: within 7 business days
Fix or mitigation target: within 30 days for high/critical issues, 90 days for others
Public disclosure: coordinated with you after a fix/mitigation is available or within 90 days, whichever comes first
Coordinated Disclosure

Please allow us time to validate and release a fix before public disclosure.
We will credit reporters in release notes unless anonymity is requested.
Safe Harbor

We support good-faith research and reporting:
Do not access, modify, or exfiltrate data you do not own.
Do not degrade service or impact other users.
Do not perform social engineering, physical attacks, or denial of service.
Keep all exploits and details private until coordinated disclosure.
If you follow this policy in good faith, we will not pursue legal action.
What We Care About

Code execution, sandbox escapes, or privilege escalation
Local file system or credential exposure via the app or installer
Unsafe update or configuration paths that lead to MiTM or proxy abuse
Insecure defaults that materially reduce security on supported Windows versions
Supply-chain risks in build and packaging (e.g., DLL search order hijacking)
What’s Typically Out of Scope

Upstream vulnerabilities in Wails, sing-box, WebView2, or Go runtime (report upstream)
Issues requiring unsupported or developer-only flags/builds
Vulnerabilities that depend on already-compromised local machines
UI/UX issues without a clear security impact
Dependencies

sing-box (GPLv3): distributed alongside the app as a separate executable. Security issues within sing-box should be reported upstream.
Wails / WebView2: issues in the embedded runtime should be reported upstream; if an integration problem is specific to VeilBox, report it here.
Temporary Mitigations

We may recommend registry or config tweaks (e.g., disabling auto-proxy, removing a vulnerable DNS upstream) while a fix is prepared.
Installer-related risks may be mitigated by verifying checksums and running offline.
Security Hardening Tips

Install from trusted releases and verify checksums/signatures if provided.
Avoid running VeilBox with elevated privileges.
Keep Windows, WebView2, and sing-box up to date.
Review imported subscription URLs; treat third-party configs as untrusted input.
No Bounty Program

We currently do not offer monetary rewards. Responsible disclosures are appreciated and may be acknowledged.
If you have any questions about this policy or are unsure whether your finding is in scope, contact us privately and we’ll help triage.

## Supported Versions

Use this section to tell people about which versions of your project are
currently being supported with security updates.

| Version | Supported          |
| ------- | ------------------ |
| 5.1.x   | :white_check_mark: |
| 5.0.x   | :x:                |
| 4.0.x   | :white_check_mark: |
| < 4.0   | :x:                |

## Reporting a Vulnerability

Use this section to tell people how to report a vulnerability.

Tell them where to go, how often they can expect to get an update on a
reported vulnerability, what to expect if the vulnerability is accepted or
declined, etc.
