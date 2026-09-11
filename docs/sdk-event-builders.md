# Typed manual event builders

The SDK exports builders for common lifecycle boundaries. They accept a shared
validated envelope context and generated protocol attribute types, returning
the exact wire event shape. Builders expose no parameter for raw signed
transaction bytes or private key material.
