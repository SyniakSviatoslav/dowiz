# deploy/ — native (non-Docker) pgrust deployment

DK-05 of the Docker full-swap. pgrust runs as a **native, systemd-managed
process** — it is a trusted first-party component (the per-node
source-of-truth), so the Docker container boundary it used to live in is
dropped entirely.

## Files

| File | Purpose |
|------|---------|
| `pgrust.service` | systemd unit. `Type=simple`, native `ExecStart=/usr/local/bin/pgrust ...`, hardened (`NoNewPrivileges`, `ProtectSystem=strict`, `PrivateTmp`, minimal `CapabilityBoundingSet`). |
| `pgrust.env` | Non-secret env vars loaded by the unit (`PGRUST_LISTEN`, `PGRUST_RLS_CROSS_TENANT=deny`, …). **No secrets committed.** |
| `pgrust.toml` | Minimal pgrust config: listen addr, `data_dir`, and the app-level RLS cross-tenant gate (`deny`). |
| `check-no-docker.sh` | RED gate. Exits non-zero if `ExecStart` references docker/podman/nerdctl/containerd. |

## Isolation model (DK-05)

pgrust's actual risk surface is **SQL injection / RLS bypass / credential
leakage**, none of which a process/VM boundary addresses. The isolation story
is therefore:

1. **Host process model** — systemd supervision, restart-on-failure, hardened
   security directives (no new privileges, strict system protection, private
   tmp, empty capability set).
2. **App-level RLS cross-tenant gate** — `PGRUST_RLS_CROSS_TENANT=deny` /
   `rls.cross_tenant = "deny"` ensures zero cross-tenant data access at the
   application layer. This is the *real* tenant-isolation boundary for pgrust.

microVM (Firecracker/Kata) is **only** an opt-in, VM-grade defense-in-depth
tier an operator may switch on if they want VM-grade DB isolation — it is NOT
the default and NOT required by DK-05.

## Native PgBouncer sidecar

Front pgrust with a **native PgBouncer** sidecar for connection pooling and
client multiplexing rather than exposing pgrust directly to tenants. Configure
`PGRUST_POOLER` / `pooler` if used. PgBouncer also runs as a native process —
no container runtime involved.

## RED gate

```sh
./deploy/check-no-docker.sh
```

Must exit `0`. CI runs this to guarantee pgrust never regresses to a container.

## Install

```sh
sudo useradd -r -s /usr/sbin/nologin pgrust
sudo mkdir -p /var/lib/pgrust /etc/pgrust
sudo cp pgrust.toml pgrust.env /etc/pgrust/
sudo cp pgrust.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now pgrust
```
