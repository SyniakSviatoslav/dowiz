//! THE CATALOGUE PROJECTION'S TWO ROUTES (R2, `fold::menu`): `/fold/menu` and
//! `/fold/products`, answered from the memo the object keeps per generation of
//! the three images it is folded from.
//!
//! A HIT MAKES NO STORAGE CALL. The key is compared against the generations of
//! the images IN MEMORY: `image()` put every image that exists there when the
//! memo was built, every write replaces the copy (a new generation, so the key
//! no longer matches) or removes it (a failed write: generation 0, same), and
//! `put_image_as` drops the memo before any write to the three. An image that
//! did not exist is 0 in both, which is why a venue with no translations does
//! not pay a storage miss per menu.

use super::{HubImages, CATALOG_IMAGE};
use crate::fold::menu::{Answer, Gens, Memo, Rails};
use worker::*;

/// The images the menu is folded from. A write to any of them drops the memo.
pub(super) const MENU_INPUTS: [&str; 3] = [CATALOG_IMAGE, crate::hubstore::IMAGE_I18N, crate::hubstore::IMAGE_SETTINGS];

fn json_body(body: String) -> Result<Response> {
    let mut res = Response::ok(body)?;
    res.headers_mut().set("content-type", "application/json")?;
    Ok(res)
}

fn bytes_of(image: &Option<(super::Meta, Vec<u8>)>) -> Option<&[u8]> {
    image.as_ref().map(|(_, b)| b.as_slice())
}

impl HubImages {
    /// The input generations as this object holds them, without a storage call.
    fn menu_gens_in_memory(&self) -> Gens {
        let mem = self.mem.borrow();
        let g = |id: &str| mem.get(id).map_or(0, |(m, _)| m.generation);
        Gens { catalog: g(MENU_INPUTS[0]), i18n: g(MENU_INPUTS[1]), settings: g(MENU_INPUTS[2]) }
    }

    /// Make sure the memo is current: a hit is three map lookups, a miss reads
    /// the three images (from memory when warm) and folds them once.
    async fn menu_memo(&self) -> Result<()> {
        let now = self.menu_gens_in_memory();
        if crate::fold::menu::is_current(&self.menu.borrow(), now) {
            return Ok(());
        }
        let catalog = self.image(MENU_INPUTS[0]).await?;
        let i18n = self.image(MENU_INPUTS[1]).await?;
        let settings = self.image(MENU_INPUTS[2]).await?;
        let gen = |i: &Option<(super::Meta, Vec<u8>)>| i.as_ref().map_or(0, |(m, _)| m.generation);
        let gens = Gens { catalog: gen(&catalog), i18n: gen(&i18n), settings: gen(&settings) };
        let rails = Rails {
            stripe_key: self.env.secret("STRIPE_PUBLISHABLE_KEY").ok().map(|v| v.to_string()),
            telegram_bot: self.env.secret("TELEGRAM_BOT_USERNAME").ok().map(|v| v.to_string()),
        };
        let memo = Memo::from_images(gens, bytes_of(&catalog), bytes_of(&i18n), bytes_of(&settings), rails)
            .map_err(Error::RustError)?;
        *self.menu.borrow_mut() = Some(memo);
        Ok(())
    }

    /// `GET /fold/menu?slug=&locale=&now=[&fresh=1]`: the storefront's menu as bytes.
    pub(super) async fn fold_menu(&self, req: &Request) -> Result<Response> {
        let url = req.url()?;
        let q = |k: &str| url.query_pairs().find(|(n, _)| n == k).map(|(_, v)| v.to_string());
        let (Some(slug), Some(now_ms)) = (q("slug"), q("now").and_then(|n| n.parse::<i64>().ok())) else {
            return Response::error("a menu needs a slug and a clock", 400);
        };
        self.menu_memo().await?;
        let answer = match self.menu.borrow_mut().as_mut() {
            Some(m) => m.menu_as(&slug, q("locale").as_deref(), q("fresh").is_some(), now_ms),
            None => Answer::Broken("the menu memo is missing".into()),
        };
        match answer {
            Answer::Body(b) => json_body(b),
            Answer::NotFound => Response::error("not found", 404),
            Answer::Broken(e) => Response::error(e, 500),
        }
    }

    /// `GET /fold/products?ids=a&ids=b`: the venue record and the products asked
    /// for, as stored -- what the live estimate needs, not the catalogue.
    pub(super) async fn fold_products(&self, req: &Request) -> Result<Response> {
        let url = req.url()?;
        let ids: Vec<String> = url.query_pairs().filter(|(k, _)| k == "ids").map(|(_, v)| v.to_string()).collect();
        self.menu_memo().await?;
        let body = self.menu.borrow().as_ref().map(|m| m.products(&ids));
        match body {
            Some(b) => json_body(b),
            None => Response::error("the menu memo is missing", 500),
        }
    }
}
