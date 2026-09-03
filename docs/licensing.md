# Licensing decision

Status: selected for the open-source repository; legal review required before a
production commercial release.

Landfall source code and project-authored documentation are licensed under the
[Apache License 2.0](../LICENSE), unless a file or bundled third-party artifact
states different terms. Package manifests use the SPDX identifier
`Apache-2.0`.

## Why Apache 2.0

Landfall's first product is a self-hosted open-source tool intended for adoption
inside infrastructure, payment, trading, and other engineering teams. A
permissive license reduces procurement and integration friction: users may run,
modify, redistribute, and use the software commercially. Compared with a short
permissive license, Apache 2.0 also provides an explicit contributor patent
grant and patent-litigation termination clause.

The license does not require customers to publish private modifications. That
supports adoption but also means competitors may legally build services around
the code. Landfall's initial commercial protection therefore comes from
execution rather than source exclusivity: paid reliability audits, deployment,
support, custom integrations, trusted expertise, and—only after validation—a
separately operated hosted offering.

## Boundaries and consequences

- The license permits commercial use; “open source” does not mean
  “non-commercial.”
- Apache 2.0 does not grant rights to Landfall names or trademarks except for
  customary attribution and describing origin.
- Distributed copies and modifications must satisfy the license's notice and
  attribution conditions.
- Contributions are accepted under Apache 2.0 section 5 unless explicitly
  designated otherwise; there is no CLA at this stage.
- Dependencies, generated artifacts, fonts, icons, and other third-party
  material retain their own licenses and must pass repository license checks.
- Paid support, warranties, or indemnities are separate commercial agreements;
  the open-source distribution remains provided under the license disclaimer.

This is an engineering and product decision, not legal advice. Before a paid
production pilot or formal release, counsel should confirm ownership,
contributor terms, warranty/financial-risk disclaimers, trademarks, dependency
compatibility, and any customer-specific data-processing obligations.
