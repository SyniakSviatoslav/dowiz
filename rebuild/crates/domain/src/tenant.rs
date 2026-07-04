//! `TenantId` — the tenant (location) identity newtype. The live schema's RLS policies key off
//! `location_id` (a `uuid` column) via `current_setting('app.current_tenant')` — see
//! REBUILD-MAP inventory/12 §2/§7. This newtype exists so "a tenant id" and "some other uuid" are
//! distinct types at the domain layer; the actual GUC-scoping mechanics (the `with_tenant`
//! combinator) live in the `api` crate, which is the only place IO happens.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TenantId(Uuid);

impl TenantId {
    pub const fn new(id: Uuid) -> Self {
        TenantId(id)
    }

    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TenantId {
    fn from(id: Uuid) -> Self {
        TenantId(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_uuid() {
        let id = Uuid::new_v4();
        let tenant = TenantId::from(id);
        assert_eq!(tenant.as_uuid(), id);
    }

    #[test]
    fn serde_round_trip_is_a_bare_string() {
        let id = Uuid::new_v4();
        let tenant = TenantId::new(id);
        let json = serde_json::to_string(&tenant).unwrap();
        assert_eq!(json, format!("\"{id}\""));

        let decoded: TenantId = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, tenant);
    }

    #[test]
    fn distinct_ids_are_not_equal() {
        let a = TenantId::from(Uuid::new_v4());
        let b = TenantId::from(Uuid::new_v4());
        assert_ne!(a, b);
    }
}
