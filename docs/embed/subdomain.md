# Subdomain Routing

Tenants are automatically provisioned a `slug.dowiz.org` subdomain.

## Fastify Middleware
The API utilizes a `onRequest` Fastify hook to intercept `.dowiz.org` hosts.
It rewrites the internal request URL to `/s/:slug` preserving all query parameters.
This ensures zero-config scaling for wildcard certificates.
