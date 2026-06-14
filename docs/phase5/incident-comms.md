# Incident Communication Templates (P32)

## Owner-Facing Messages

### Scenario A: Primary DB Issue / Maintenance

**Albanian (SQ):**
> Të nderuar partnerë,
>
> Ne kemi identifikuar një problem me bazën e të dhënave. Ekipi ynë teknik po punon për rivendosjen e shërbimit. Procesi i rivendosjes mund të zgjasë deri në 4 orë.
>
> **Porositë tuaja janë të sigurta.** Të gjitha porositë aktuale do të ruhen dhe do të përpunohen pas rivendosjes.
>
> Për ndihmë shtesë, na kontaktoni në support@dowiz.org ose përmes panelit të adminit.
>
> Faleminderit për durimin,
> Ekipi DeliveryOS

**English (EN):**
> Dear partners,
>
> We've identified a database issue. Our engineering team is working on restoration. Recovery may take up to 4 hours.
>
> **Your orders are safe.** All active orders are preserved and will be processed after restoration.
>
> For additional support, contact us at support@dowiz.org or via the admin panel.
>
> Thank you for your patience,
> DeliveryOS Team

### Scenario B: Backup Verification Failure

**SQ:**
> Një kontroll rutinë i kopjeve rezervë tregoi një problem. Kopjet rezervë ekzistuese mbeten të paprekura, por kopjet e ardshme do të monitorohen nga afër. Ekipi ynë po heton.

**EN:**
> A routine backup verification check detected an issue. Existing backups remain unaffected, but future backups will be closely monitored. Our team is investigating.

### Scenario C: R2 Backup Degradation

**SQ:**
> Një pjesë e sistemit tonë të ruajtjes rezervë ka një problem. Kopjet rezervë të reja mund të vonohen. Kopjet ekzistuese janë të paprekura. Ekipi ynë po punon për zgjidhjen.

**EN:**
> Part of our backup storage system is experiencing issues. New backups may be delayed. Existing backups are unaffected. Our team is working on a resolution.

## Internal Runbook

### Alert Response Flow

1. **Telegram alert** received → acknowledge within 5 min
2. Check `/health` → determine scope (DB/R2/worker)
3. If DB issue → follow Scenario A in [disaster-recovery.md](./disaster-recovery.md)
4. If R2 issue → follow Scenario B
5. If verify issue → check `pnpm backup:verify --backup-id=<ID>` manually
6. Update incident status in admin

### Escalation

| Severity | Response | Escalate to |
|----------|----------|-------------|
| P0 — unhealthy (503) | Immediate | Senior engineer + ops |
| P1 — verify failed | < 15 min | On-call engineer |
| P2 — stale verify | < 24h | Ops team |
