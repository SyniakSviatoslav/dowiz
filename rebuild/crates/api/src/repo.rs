//! `PublicRepo` — the S1 storefront-read data-access trait. ONE trait (mirroring Node's single
//! `server.db`/`opts.db` handle threaded through every public route) so route handlers can be
//! unit-tested against a `FakeRepo` (`#[cfg(test)]`) without a live Postgres, per the build
//! brief ("handler logic with the DB layer behind a trait or cfg(test) stub").
//!
//! `PgRepo` is the real `sqlx` implementation. It uses ONLY runtime `sqlx::query`/`query_as`
//! (never the `query!`/`query_as!` compile-time macros — there is no reachable `DATABASE_URL` +
//! `.sqlx/` offline cache in this sandbox, see `rebuild/README.md` Open question 2).
//!
//! None of these queries go through `db::with_tenant`: every S1 route is unauthenticated and,
//! verified against the live Node source, NONE of them ever call `withTenant` — they read by
//! explicit `slug`/`id` predicate against a BYPASSRLS-role pool (`menu.ts:280-282` comment
//! confirms the pool role bypasses RLS; `withTenant` in spa-proxy.ts is used ONLY by the
//! authenticated owner routes below line 300, none of which are S1 operations). So `with_tenant`
//! correctly stays uncalled by this build too — see the lane report for why this resolves
//! `rebuild/README.md`'s "Open questions... dual tenant GUC" note rather than needing a fix.

use serde_json::Value as Json;
use sqlx::PgPool;
use sqlx::error::DatabaseError;
use uuid::Uuid;

/// Wraps `sqlx::Error` so callers outside this module never need the `sqlx` dependency directly
/// (route handlers only see `RepoError`).
#[derive(Debug, thiserror::Error)]
#[error("repo error: {0}")]
pub struct RepoError(#[from] pub sqlx::Error);

impl RepoError {
    /// True when the underlying Postgres error is `42883 undefined_function` — the
    /// migration-not-yet-applied signal `read_preview_menu`'s caller treats as "not a shadow",
    /// not a hard failure (`menu.ts:129-131`, `ssr.ts:43`). Uses the generic
    /// `DatabaseError::code()` (SQLSTATE, backend-populated) rather than downcasting to the
    /// Postgres-specific error type — one fewer import for the same information.
    pub fn is_undefined_function(&self) -> bool {
        self.0
            .as_database_error()
            .and_then(DatabaseError::code)
            .is_some_and(|code| code == "42883")
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LocationInfoRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub currency_code: String,
    pub currency_minor_unit: i32,
    pub default_locale: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub delivery_paused: bool,
    pub hours_json: Option<Json>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub kitchen_busy_until: Option<chrono::DateTime<chrono::Utc>>,
    pub delivery_fee_flat: Option<i64>,
    pub free_delivery_threshold: Option<i64>,
    pub min_order_value: Option<i64>,
    pub tax_rate: Option<f64>,
    pub price_includes_tax: bool,
    pub has_distance_tiers: bool,
    pub google_rating: Option<f64>,
    pub google_review_count: Option<i32>,
    pub google_maps_url: Option<String>,
    pub google_place_id: Option<String>,
    pub social_instagram: Option<String>,
    pub social_facebook: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MediaRow {
    pub id: Uuid,
    pub kind: String,
    pub storage_key: Option<String>,
    pub mime_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_ms: Option<i32>,
    pub poster_key: Option<String>,
    pub alt: Option<String>,
    pub sort_order: i32,
    pub meta: Option<Json>,
}

/// The narrow projection `menu.ts:173-176`'s batched primary-media resolution actually reads
/// (only `kind` decides which key becomes the card image; `mime_type`/`width`/etc. are never
/// touched there) — kept distinct from the full `MediaRow` so this query doesn't have to
/// fabricate placeholder values for columns it never selects.
#[derive(Debug, Clone)]
pub struct PrimaryMediaRow {
    pub id: Uuid,
    pub kind: String,
    pub storage_key: Option<String>,
    pub poster_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ThemeRow {
    pub primary_color: Option<String>,
    pub bg_color: Option<String>,
    pub text_color: Option<String>,
    pub logo_url: Option<String>,
    pub heading_font: Option<String>,
    pub body_font: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SpaShellTenantRow {
    pub name: String,
    pub address: Option<String>,
    pub frame_ancestors: Option<Vec<String>>,
    pub logo_url: Option<String>,
    /// `None` iff `organizations.owner_id IS NULL` — the shadow-tenant discriminator
    /// (P6-2/P6-3 privacy invariant, `spa-shell.ts:136`).
    pub owner_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct SitemapLocationRow {
    pub slug: String,
    pub supported_locales: Option<Vec<String>>,
    pub has_products: bool,
    pub lastmod: chrono::DateTime<chrono::Utc>,
}

/// `read_preview_menu`'s three-way outcome (`menu.ts:124-132`, `ssr.ts:32-44`): the migration
/// that adds the fn may not be applied yet, distinct from "applied, but this tenant isn't a
/// shadow" (both fall through to the live-tenant path — only the log line differs in Node).
pub enum PreviewLookup {
    Found(Json),
    NotShadow,
    FunctionMissing,
}

#[async_trait::async_trait]
pub trait PublicRepo: Send + Sync {
    /// `SELECT read_public_menu($1, $2) as menu` (menu.ts:116). `Ok(None)` = the DEFINER fn
    /// returned null (unknown OR never-live location) — caller tries the shadow-preview
    /// fallback next.
    async fn read_public_menu(
        &self,
        slug_or_id: &str,
        locale: &str,
    ) -> Result<Option<Json>, RepoError>;

    /// `SELECT read_preview_menu($1) as menu` (menu.ts:125, ssr.ts:33).
    async fn read_preview_menu(&self, slug: &str) -> Result<PreviewLookup, RepoError>;

    /// `SELECT id, name FROM locations WHERE id::text = $1 OR slug = $1` (menu.ts:139-142).
    async fn location_id_name(&self, slug_or_id: &str)
    -> Result<Option<(Uuid, String)>, RepoError>;

    /// `SELECT id, storage_key, poster_key, kind FROM product_media WHERE id = ANY($1) AND available`
    /// (menu.ts:173-176) — batched primary-media resolution for the menu card grid.
    async fn product_media_by_ids(&self, ids: &[Uuid]) -> Result<Vec<PrimaryMediaRow>, RepoError>;

    /// `SELECT id, plan FROM locations WHERE id::text = $1 OR slug = $1` (menu.ts:428-431).
    async fn location_id_plan(
        &self,
        slug_or_id: &str,
    ) -> Result<Option<(Uuid, Option<String>)>, RepoError>;

    /// The lazy product-media list (menu.ts:437-444).
    async fn product_media(
        &self,
        location_id: Uuid,
        product_id: Uuid,
    ) -> Result<Vec<MediaRow>, RepoError>;

    /// The `/info` row join (menu.ts:272-289).
    async fn location_info(&self, slug: &str) -> Result<Option<LocationInfoRow>, RepoError>;

    /// `SELECT id, name, supported_locales FROM locations WHERE slug = $1` (spa-proxy.ts:507).
    async fn theme_location(
        &self,
        slug: &str,
    ) -> Result<Option<(Uuid, String, Option<Vec<String>>)>, RepoError>;

    /// `SELECT primary_color, ... FROM location_themes WHERE location_id = $1` (spa-proxy.ts:511-514).
    async fn theme_colors(&self, location_id: Uuid) -> Result<Option<ThemeRow>, RepoError>;

    /// Resolves `locationId` (uuid or slug) to a uuid — `theme.ts:19-24`.
    async fn resolve_location_uuid(
        &self,
        location_id_or_slug: &str,
    ) -> Result<Option<Uuid>, RepoError>;

    /// `theme_versions` lookup by hash (immutable pin) or latest version (`theme.ts:26-36`).
    async fn theme_css_body(
        &self,
        location_uuid: Uuid,
        hash: Option<&str>,
    ) -> Result<Option<String>, RepoError>;

    /// `locations JOIN location_themes` name + primary_color for the PWA manifest (`pwa.ts:17-22`).
    async fn manifest_info(
        &self,
        slug: &str,
    ) -> Result<Option<(String, Option<String>)>, RepoError>;

    /// `fallback_config` jsonb + `phone` (`fallback-config.ts:16-20`).
    async fn fallback_config(
        &self,
        slug: &str,
    ) -> Result<Option<(Json, Option<String>)>, RepoError>;

    /// Latest ALL->EUR rate row (`rates.ts:16-20`).
    async fn latest_exchange_rate(
        &self,
    ) -> Result<Option<(f64, chrono::DateTime<chrono::Utc>)>, RepoError>;

    /// Active, non-shadow locations for the sitemap (`seo.ts:9-33`).
    async fn active_locations(&self) -> Result<Vec<SitemapLocationRow>, RepoError>;

    /// The SPA-shell tenant/CSP lookup (`spa-shell.ts:122-129`).
    async fn spa_shell_tenant(&self, slug: &str) -> Result<Option<SpaShellTenantRow>, RepoError>;
}

/// The real `sqlx`-backed implementation. Connects to `Pools::operational` directly (no
/// `with_tenant` — see module doc).
pub struct PgRepo {
    pool: PgPool,
}

impl PgRepo {
    pub fn new(pool: PgPool) -> Self {
        PgRepo { pool }
    }
}

#[async_trait::async_trait]
impl PublicRepo for PgRepo {
    async fn read_public_menu(
        &self,
        slug_or_id: &str,
        locale: &str,
    ) -> Result<Option<Json>, RepoError> {
        let row: Option<(Option<Json>,)> =
            sqlx::query_as("SELECT read_public_menu($1, $2) as menu")
                .bind(slug_or_id)
                .bind(locale)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.and_then(|(menu,)| menu))
    }

    async fn read_preview_menu(&self, slug: &str) -> Result<PreviewLookup, RepoError> {
        let result: Result<Option<(Option<Json>,)>, sqlx::Error> =
            sqlx::query_as("SELECT read_preview_menu($1) as menu")
                .bind(slug)
                .fetch_optional(&self.pool)
                .await;
        match result {
            Ok(Some((Some(menu),))) => Ok(PreviewLookup::Found(menu)),
            Ok(_) => Ok(PreviewLookup::NotShadow),
            Err(err) => {
                let repo_err = RepoError(err);
                if repo_err.is_undefined_function() {
                    Ok(PreviewLookup::FunctionMissing)
                } else {
                    Err(repo_err)
                }
            }
        }
    }

    async fn location_id_name(
        &self,
        slug_or_id: &str,
    ) -> Result<Option<(Uuid, String)>, RepoError> {
        let row: Option<(Uuid, String)> =
            sqlx::query_as("SELECT id, name FROM locations WHERE id::text = $1 OR slug = $1")
                .bind(slug_or_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    async fn product_media_by_ids(&self, ids: &[Uuid]) -> Result<Vec<PrimaryMediaRow>, RepoError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(Uuid, Option<String>, Option<String>, String)> = sqlx::query_as(
            "SELECT id, storage_key, poster_key, kind FROM product_media WHERE id = ANY($1::uuid[]) AND available",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, storage_key, poster_key, kind)| PrimaryMediaRow {
                id,
                kind,
                storage_key,
                poster_key,
            })
            .collect())
    }

    async fn location_id_plan(
        &self,
        slug_or_id: &str,
    ) -> Result<Option<(Uuid, Option<String>)>, RepoError> {
        let row: Option<(Uuid, Option<String>)> =
            sqlx::query_as("SELECT id, plan FROM locations WHERE id::text = $1 OR slug = $1")
                .bind(slug_or_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    async fn product_media(
        &self,
        location_id: Uuid,
        product_id: Uuid,
    ) -> Result<Vec<MediaRow>, RepoError> {
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            Uuid,
            String,
            Option<String>,
            String,
            Option<i32>,
            Option<i32>,
            Option<i32>,
            Option<String>,
            Option<String>,
            i32,
            Option<Json>,
        )> = sqlx::query_as(
            "SELECT id, kind, storage_key, mime_type, width, height, duration_ms,
                    poster_key, alt, sort_order, meta
               FROM product_media
              WHERE location_id = $1 AND product_id = $2 AND available = true
              ORDER BY sort_order ASC, created_at ASC",
        )
        .bind(location_id)
        .bind(product_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    kind,
                    storage_key,
                    mime_type,
                    width,
                    height,
                    duration_ms,
                    poster_key,
                    alt,
                    sort_order,
                    meta,
                )| MediaRow {
                    id,
                    kind,
                    storage_key,
                    mime_type,
                    width,
                    height,
                    duration_ms,
                    poster_key,
                    alt,
                    sort_order,
                    meta,
                },
            )
            .collect())
    }

    async fn location_info(&self, slug: &str) -> Result<Option<LocationInfoRow>, RepoError> {
        // A plain tuple can't represent this row: sqlx's built-in `FromRow` impls only cover
        // tuples up to a fixed arity, well under this query's 24 columns — `LocationInfoRow`
        // derives `sqlx::FromRow` directly instead (field order matches the SELECT list
        // 1:1, so no manual column-index mapping is needed).
        let row: Option<LocationInfoRow> = sqlx::query_as(
            "SELECT l.id, l.name, l.slug, l.currency_code, l.currency_minor_unit, l.default_locale,
                    l.lat, l.lng, l.delivery_paused, l.hours_json, l.address, l.phone, l.kitchen_busy_until,
                    l.delivery_fee_flat, l.free_delivery_threshold, l.min_order_value,
                    l.tax_rate, l.price_includes_tax,
                    (EXISTS (SELECT 1 FROM delivery_tiers dt WHERE dt.location_id = l.id)
                       AND l.lat IS NOT NULL AND l.lng IS NOT NULL) AS has_distance_tiers,
                    lt.google_rating, lt.google_review_count, lt.google_maps_url,
                    lt.google_place_id, lt.social_instagram, lt.social_facebook
             FROM locations l
             LEFT JOIN location_themes lt ON lt.location_id = l.id
             WHERE l.slug = $1",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn theme_location(
        &self,
        slug: &str,
    ) -> Result<Option<(Uuid, String, Option<Vec<String>>)>, RepoError> {
        let row: Option<(Uuid, String, Option<Vec<String>>)> =
            sqlx::query_as("SELECT id, name, supported_locales FROM locations WHERE slug = $1")
                .bind(slug)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    async fn theme_colors(&self, location_id: Uuid) -> Result<Option<ThemeRow>, RepoError> {
        let row: Option<(
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT primary_color, bg_color, text_color, logo_url, heading_font, body_font
               FROM location_themes WHERE location_id = $1",
        )
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(primary_color, bg_color, text_color, logo_url, heading_font, body_font)| ThemeRow {
                primary_color,
                bg_color,
                text_color,
                logo_url,
                heading_font,
                body_font,
            },
        ))
    }

    async fn resolve_location_uuid(
        &self,
        location_id_or_slug: &str,
    ) -> Result<Option<Uuid>, RepoError> {
        if crate::service::is_uuid_format(location_id_or_slug) {
            return Ok(Uuid::parse_str(location_id_or_slug).ok());
        }
        let row: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM locations WHERE slug = $1")
            .bind(location_id_or_slug)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|(id,)| id))
    }

    async fn theme_css_body(
        &self,
        location_uuid: Uuid,
        hash: Option<&str>,
    ) -> Result<Option<String>, RepoError> {
        let row: Option<(String,)> = match hash {
            Some(hash) => {
                sqlx::query_as(
                    "SELECT css_body FROM theme_versions WHERE location_id = $1 AND css_hash = $2",
                )
                .bind(location_uuid)
                .bind(hash)
                .fetch_optional(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as(
                    "SELECT css_body FROM theme_versions WHERE location_id = $1 ORDER BY version DESC LIMIT 1",
                )
                .bind(location_uuid)
                .fetch_optional(&self.pool)
                .await?
            }
        };
        Ok(row.map(|(css,)| css))
    }

    async fn manifest_info(
        &self,
        slug: &str,
    ) -> Result<Option<(String, Option<String>)>, RepoError> {
        let row: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT l.name, lt.primary_color FROM locations l
               LEFT JOIN location_themes lt ON lt.location_id = l.id
              WHERE l.slug = $1 LIMIT 1",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn fallback_config(
        &self,
        slug: &str,
    ) -> Result<Option<(Json, Option<String>)>, RepoError> {
        let row: Option<(Option<Json>, Option<String>)> = sqlx::query_as(
            "SELECT fallback_config, phone AS public_phone FROM locations WHERE slug = $1",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(config, phone)| (config.unwrap_or(Json::Object(Default::default())), phone)))
    }

    async fn latest_exchange_rate(
        &self,
    ) -> Result<Option<(f64, chrono::DateTime<chrono::Utc>)>, RepoError> {
        let row: Option<(String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
            "SELECT rate::text, fetched_at FROM exchange_rates
              WHERE base_currency = 'ALL' AND target_currency = 'EUR'
              ORDER BY fetched_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|(rate, fetched_at)| rate.parse::<f64>().ok().map(|r| (r, fetched_at))))
    }

    async fn active_locations(&self) -> Result<Vec<SitemapLocationRow>, RepoError> {
        let rows: Vec<(
            String,
            Option<Vec<String>>,
            bool,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        )> = sqlx::query_as(
            "SELECT l.slug, l.supported_locales,
                    EXISTS(SELECT 1 FROM products p WHERE p.location_id = l.id AND p.is_available = true LIMIT 1) as has_products,
                    mv.updated_at AS mv_updated_at,
                    l.created_at
             FROM locations l
             JOIN organizations o ON o.id = l.org_id
             LEFT JOIN menu_versions mv ON mv.location_id = l.id
             WHERE l.status IS DISTINCT FROM 'deleted'
               AND l.status IS DISTINCT FROM 'disabled'
               AND o.owner_id IS NOT NULL
             ORDER BY l.slug",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(slug, supported_locales, has_products, mv_updated_at, created_at)| {
                    let lastmod = mv_updated_at
                        .or(created_at)
                        .unwrap_or_else(chrono::Utc::now);
                    SitemapLocationRow {
                        slug,
                        supported_locales,
                        has_products,
                        lastmod,
                    }
                },
            )
            .collect())
    }

    async fn spa_shell_tenant(&self, slug: &str) -> Result<Option<SpaShellTenantRow>, RepoError> {
        let row: Option<(
            String,
            Option<String>,
            Option<Vec<String>>,
            Option<String>,
            Option<Uuid>,
        )> = sqlx::query_as(
            "SELECT l.name, l.address, lt.frame_ancestors, lt.logo_url, o.owner_id
                   FROM locations l
                   JOIN organizations o ON o.id = l.org_id
                   LEFT JOIN location_themes lt ON lt.location_id = l.id
                  WHERE l.slug = $1 LIMIT 1",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(name, address, frame_ancestors, logo_url, owner_id)| SpaShellTenantRow {
                name,
                address,
                frame_ancestors,
                logo_url,
                owner_id,
            },
        ))
    }
}

#[cfg(test)]
pub mod fake {
    //! `FakeRepo` — the `cfg(test)` stub the build brief asks for, so handler-logic tests never
    //! need a live Postgres. Every method returns canned data configured via the builder.
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// `(id, name, supported_locales)` — matches `PublicRepo::theme_location`'s return shape.
    /// Named here (rather than repeating the raw tuple) so clippy's `type_complexity` lint
    /// (workspace `deny`) doesn't flag the `FakeRepo` field below; the trait method itself stays
    /// a plain tuple since utoipa/sqlx code generation doesn't benefit from an alias there.
    type ThemeLocationEntry = (Uuid, String, Option<Vec<String>>);

    #[derive(Default)]
    pub struct FakeRepo {
        pub public_menus: Mutex<HashMap<String, Json>>,
        pub preview_menus: Mutex<HashMap<String, Json>>,
        pub location_info: Mutex<HashMap<String, LocationInfoRow>>,
        pub theme_locations: Mutex<HashMap<String, ThemeLocationEntry>>,
        pub theme_colors: Mutex<HashMap<Uuid, ThemeRow>>,
        pub manifest_info: Mutex<HashMap<String, (String, Option<String>)>>,
        pub fallback_configs: Mutex<HashMap<String, (Json, Option<String>)>>,
        pub exchange_rate: Mutex<Option<(f64, chrono::DateTime<chrono::Utc>)>>,
        pub sitemap_locations: Mutex<Vec<SitemapLocationRow>>,
        pub spa_shell_tenants: Mutex<HashMap<String, SpaShellTenantRow>>,
        pub location_id_name_map: Mutex<HashMap<String, (Uuid, String)>>,
        pub location_id_plan_map: Mutex<HashMap<String, (Uuid, Option<String>)>>,
        pub primary_media_rows: Mutex<HashMap<Uuid, PrimaryMediaRow>>,
        pub product_media_rows: Mutex<HashMap<(Uuid, Uuid), Vec<MediaRow>>>,
        pub theme_css_bodies: Mutex<HashMap<Uuid, String>>,
        pub location_uuids: Mutex<HashMap<String, Uuid>>,
    }

    #[async_trait::async_trait]
    impl PublicRepo for FakeRepo {
        async fn read_public_menu(
            &self,
            slug_or_id: &str,
            _locale: &str,
        ) -> Result<Option<Json>, RepoError> {
            Ok(self.public_menus.lock().unwrap().get(slug_or_id).cloned())
        }

        async fn read_preview_menu(&self, slug: &str) -> Result<PreviewLookup, RepoError> {
            match self.preview_menus.lock().unwrap().get(slug).cloned() {
                Some(v) => Ok(PreviewLookup::Found(v)),
                None => Ok(PreviewLookup::NotShadow),
            }
        }

        async fn location_id_name(
            &self,
            slug_or_id: &str,
        ) -> Result<Option<(Uuid, String)>, RepoError> {
            Ok(self
                .location_id_name_map
                .lock()
                .unwrap()
                .get(slug_or_id)
                .cloned())
        }

        async fn product_media_by_ids(
            &self,
            ids: &[Uuid],
        ) -> Result<Vec<PrimaryMediaRow>, RepoError> {
            let rows = self.primary_media_rows.lock().unwrap();
            Ok(ids.iter().filter_map(|id| rows.get(id).cloned()).collect())
        }

        async fn location_id_plan(
            &self,
            slug_or_id: &str,
        ) -> Result<Option<(Uuid, Option<String>)>, RepoError> {
            Ok(self
                .location_id_plan_map
                .lock()
                .unwrap()
                .get(slug_or_id)
                .cloned())
        }

        async fn product_media(
            &self,
            location_id: Uuid,
            product_id: Uuid,
        ) -> Result<Vec<MediaRow>, RepoError> {
            Ok(self
                .product_media_rows
                .lock()
                .unwrap()
                .get(&(location_id, product_id))
                .cloned()
                .unwrap_or_default())
        }

        async fn location_info(&self, slug: &str) -> Result<Option<LocationInfoRow>, RepoError> {
            Ok(self.location_info.lock().unwrap().get(slug).cloned())
        }

        async fn theme_location(
            &self,
            slug: &str,
        ) -> Result<Option<(Uuid, String, Option<Vec<String>>)>, RepoError> {
            Ok(self.theme_locations.lock().unwrap().get(slug).cloned())
        }

        async fn theme_colors(&self, location_id: Uuid) -> Result<Option<ThemeRow>, RepoError> {
            Ok(self.theme_colors.lock().unwrap().get(&location_id).cloned())
        }

        async fn resolve_location_uuid(
            &self,
            location_id_or_slug: &str,
        ) -> Result<Option<Uuid>, RepoError> {
            if let Ok(id) = Uuid::parse_str(location_id_or_slug) {
                return Ok(Some(id));
            }
            Ok(self
                .location_uuids
                .lock()
                .unwrap()
                .get(location_id_or_slug)
                .copied())
        }

        async fn theme_css_body(
            &self,
            location_uuid: Uuid,
            _hash: Option<&str>,
        ) -> Result<Option<String>, RepoError> {
            Ok(self
                .theme_css_bodies
                .lock()
                .unwrap()
                .get(&location_uuid)
                .cloned())
        }

        async fn manifest_info(
            &self,
            slug: &str,
        ) -> Result<Option<(String, Option<String>)>, RepoError> {
            Ok(self.manifest_info.lock().unwrap().get(slug).cloned())
        }

        async fn fallback_config(
            &self,
            slug: &str,
        ) -> Result<Option<(Json, Option<String>)>, RepoError> {
            Ok(self.fallback_configs.lock().unwrap().get(slug).cloned())
        }

        async fn latest_exchange_rate(
            &self,
        ) -> Result<Option<(f64, chrono::DateTime<chrono::Utc>)>, RepoError> {
            Ok(*self.exchange_rate.lock().unwrap())
        }

        async fn active_locations(&self) -> Result<Vec<SitemapLocationRow>, RepoError> {
            Ok(self.sitemap_locations.lock().unwrap().clone())
        }

        async fn spa_shell_tenant(
            &self,
            slug: &str,
        ) -> Result<Option<SpaShellTenantRow>, RepoError> {
            Ok(self.spa_shell_tenants.lock().unwrap().get(slug).cloned())
        }
    }
}
