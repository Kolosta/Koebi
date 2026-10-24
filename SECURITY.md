# Security policy

## Supported versions

Koebi is in early development. Only the latest release and the `develop` branch receive
security fixes.

## Reporting a vulnerability

Please report vulnerabilities privately through GitHub:
[Security > Report a vulnerability](https://github.com/Kolosta/Koebi/security/advisories/new).
Do not open a public issue.

Include what you can: affected version or commit, platform and GPU, steps to reproduce, impact,
and a proof of concept if you have one.

You will get an acknowledgement within 7 days. Confirmed issues are fixed on a `hotfix/*`
branch and published in a release with a GitHub security advisory crediting the reporter,
unless you prefer to stay anonymous.

## Scope

In scope: memory safety issues in Koebi's code (including `unsafe` FFI and Vulkan usage),
crashes triggered by input files or events, and weaknesses in the build and release pipeline.

Out of scope: vulnerabilities in GPU drivers, the Vulkan loader, MoltenVK or SDL themselves
(report them upstream; we will update or work around them).
